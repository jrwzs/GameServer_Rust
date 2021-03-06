use crate::handlers::battle_handler::{
    action, buy, choice_index, emoji, leave_room, off_line, pos, reload_temps, start, update_season,
};
use crate::robot::robot_task_mgr::RobotTask;
use crate::room::room::Room;
use crate::room::{MemberLeaveNoticeType, RoomState};
use crate::task_timer::Task;
use crossbeam::channel::Sender;
use log::{info, warn};
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use tools::cmd_code::{BattleCode, ServerCommonCode};
use tools::util::packet::Packet;

type CmdFn = HashMap<u32, fn(&mut BattleMgr, Packet), RandomState>;

///战斗管理器
#[derive(Default)]
pub struct BattleMgr {
    pub player_room: HashMap<u32, u32>,               //玩家对应房间
    pub rooms: HashMap<u32, Room>,                    //房间map
    pub cmd_map: CmdFn,                               //命令函数指针
    pub game_center_channel: Option<Sender<Vec<u8>>>, //tcp客户的
    pub task_sender: Option<Sender<Task>>,            //task channel的发送方
    pub robot_task_sender: Option<Sender<RobotTask>>, //机器人task channel的发送方
}

tools::get_mut_ref!(BattleMgr);

impl BattleMgr {
    pub fn set_game_center_channel(&mut self, ts: Sender<Vec<u8>>) {
        self.game_center_channel = Some(ts);
    }

    pub fn new() -> BattleMgr {
        let mut bm = BattleMgr::default();
        bm.cmd_init();
        bm
    }

    pub fn get_game_center_channel_clone(&self) -> crossbeam::channel::Sender<Vec<u8>> {
        self.game_center_channel.as_ref().unwrap().clone()
    }

    pub fn get_task_sender_clone(&self) -> crossbeam::channel::Sender<Task> {
        self.task_sender.as_ref().unwrap().clone()
    }

    pub fn get_robot_task_sender_clone(&self) -> crossbeam::channel::Sender<RobotTask> {
        self.robot_task_sender.as_ref().unwrap().clone()
    }

    pub fn send_2_server(&mut self, cmd: u32, user_id: u32, bytes: Vec<u8>) {
        let room = self.get_room_ref(&user_id);
        match room {
            Some(room) => {
                let battle_player = room.get_battle_player_ref(&user_id);
                if let Some(battle_player) = battle_player {
                    if battle_player.is_robot() {
                        return;
                    }
                }
            }
            None => {}
        }
        let bytes = Packet::build_packet_bytes(cmd, user_id, bytes, true, false);
        let res = self.get_game_center_channel_mut();
        let size = res.send(bytes);
        if let Err(e) = size {
            warn!("{:?}", e);
        }
    }

    pub fn get_game_center_channel_mut(&mut self) -> &mut Sender<Vec<u8>> {
        self.game_center_channel.as_mut().unwrap()
    }

    ///执行函数，通过packet拿到cmd，然后从cmdmap拿到函数指针调用
    pub fn invok(&mut self, packet: Packet) {
        let cmd = packet.get_cmd();
        let f = self.cmd_map.get_mut(&cmd);
        if f.is_none() {
            warn!("there is no handler of cmd:{:?}!", cmd);
            return;
        }
        let _ = f.unwrap()(self, packet);
    }

    pub fn get_room_mut(&mut self, user_id: &u32) -> Option<&mut Room> {
        let res = self.player_room.get(user_id);
        if res.is_none() {
            return None;
        }
        let res = res.unwrap();
        let room_id = *res;
        self.rooms.get_mut(&room_id)
    }

    #[allow(dead_code)]
    pub fn get_room_ref(&self, user_id: &u32) -> Option<&Room> {
        let res = self.player_room.get(user_id);
        if res.is_none() {
            return None;
        }
        let res = res.unwrap();
        let room_id = *res;
        self.rooms.get(&room_id)
    }

    ///删除房间
    pub fn rm_room(&mut self, room_id: u32) {
        let room = self.rooms.remove(&room_id);
        if let Some(room) = room {
            let room_type = room.get_room_type();
            let room_id = room.get_room_id();
            for user_id in room.members.keys() {
                self.player_room.remove(user_id);
            }

            info!(
                "房间战斗结束！删除房间，释放内存！room_type:{:?},room_id:{}",
                room_type, room_id
            );
        }
    }

    ///处理玩家离开战斗
    ///need_push_self:是否需要推送给自己
    pub fn handler_leave(
        &mut self,
        room_id: u32,
        notice_type: MemberLeaveNoticeType,
        user_id: u32,
        need_push_self: bool,
    ) {
        let room = self.rooms.get_mut(&room_id).unwrap();
        let battle_player = room.get_battle_player_ref(&user_id);
        if battle_player.is_none() {
            return;
        }
        let battle_player = battle_player.unwrap();

        let room_type = room.get_room_type();
        //如果是主动推出房间，房间为匹配房，玩家没死，不允许退出房间
        if notice_type == MemberLeaveNoticeType::Leave
            && room_type.is_match_type()
            && !battle_player.is_died()
        {
            warn!(
                "player can not leave room!room_type: {:?},user_id:{},battle_state:{:?} ",
                room_type, user_id, battle_player.status.battle_state
            );
        }
        let room_id = room.get_room_id();
        room.remove_member(notice_type, &user_id, need_push_self);
        info!("玩家离线战斗服务!room_id={},user_id={}", room_id, user_id);
        //判断战斗是否结束
        if room.is_empty() || room.is_all_robot() || room.state == RoomState::BattleOvered {
            self.rm_room(room_id);
        }
    }

    ///命令初始化
    fn cmd_init(&mut self) {
        //热更静态配置
        self.cmd_map
            .insert(ServerCommonCode::ReloadTemps.into_u32(), reload_temps);
        //更新赛季信息
        self.cmd_map
            .insert(BattleCode::UpdateSeasonPush.into_u32(), update_season);
        //离线
        self.cmd_map
            .insert(BattleCode::OffLine.into_u32(), off_line);
        //离开房间
        self.cmd_map
            .insert(BattleCode::LeaveRoom.into_u32(), leave_room);
        //开始战斗
        self.cmd_map.insert(BattleCode::Start.into_u32(), start);

        //发送表情
        self.cmd_map.insert(BattleCode::Emoji.into_u32(), emoji);

        //选择占位
        self.cmd_map
            .insert(BattleCode::ChoiceIndex.into_u32(), choice_index);
        //------------------------------------以下是战斗相关的--------------------------------
        //请求行动
        self.cmd_map.insert(BattleCode::Action.into_u32(), action);
        //请求pos
        self.cmd_map.insert(BattleCode::Pos.into_u32(), pos);
        //购物
        self.cmd_map.insert(BattleCode::Buy.into_u32(), buy);
    }
}
