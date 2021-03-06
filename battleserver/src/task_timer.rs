use crate::mgr::battle_mgr::BattleMgr;
use crate::room::{MemberLeaveNoticeType, RoomState};
use crate::{JsonValue, Lock, SCHEDULED_MGR};
use async_std::sync::{Arc, Mutex};
use async_std::task::block_on;
use log::{error, info, warn};
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use std::time::Duration;

#[derive(Debug, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u16)]
pub enum TaskCmd {
    None,               //没有任何意义,默认值
    MatchRoomStart,     //匹配房间开始任务
    ChoiceIndex,        //选择占位
    BattleTurnTime,     //战斗时间回合限制
    MaxBattleTurnTimes, //战斗turn达到最大
}

impl Default for TaskCmd {
    fn default() -> Self {
        TaskCmd::None
    }
}

#[derive(Debug, Clone, Default)]
pub struct Task {
    pub cmd: TaskCmd,    //要执行的命令
    pub delay: u64,      //要延迟执行的时间
    pub turn: u32,       //turn
    pub battle_id: u32,  //战斗id
    pub data: JsonValue, //数据
}

///初始化定时执行任务
pub fn init_timer(bm: Lock) {
    let m = move || {
        let (sender, rec) = crossbeam::channel::bounded(1024);
        let mut lock = block_on(bm.lock());
        lock.task_sender = Some(sender);
        std::mem::drop(lock);

        loop {
            let res = rec.recv();
            if let Err(e) = res {
                error!("{:?}", e);
                continue;
            }
            let task = res.unwrap();
            let delay = task.delay;

            let task_cmd = TaskCmd::from(task.cmd);
            let rm_clone = bm.clone();
            let f = match task_cmd {
                TaskCmd::ChoiceIndex => choice_index,
                TaskCmd::BattleTurnTime => battle_turn_time,
                TaskCmd::MaxBattleTurnTimes => max_battle_turn_limit,
                _ => none,
            };
            let m = move || f(rm_clone, task);
            SCHEDULED_MGR.execute_after(Duration::from_millis(delay), m);
        }
    };
    let timer_thread = std::thread::Builder::new().name("TIMER_THREAD".to_owned());
    let res = timer_thread.spawn(m);
    if let Err(e) = res {
        error!("{:?}", e);
        std::process::abort();
    }
    info!("初始化定时器任务执行器成功!");
}
fn none(_: Arc<Mutex<BattleMgr>>, _: Task) {}

///占位任务，没选的直接t出房间
fn choice_index(bm: Arc<Mutex<BattleMgr>>, task: Task) {
    let task_batlte_id = task.battle_id;
    let json_value = task.data;
    let res = json_value.as_object();
    if res.is_none() {
        return;
    }
    let map = res.unwrap();
    let user_id = map.get("user_id");
    if user_id.is_none() {
        return;
    }
    let user_id = user_id.unwrap().as_u64();
    if user_id.is_none() {
        return;
    }
    let user_id = user_id.unwrap() as u32;

    let mut lock = block_on(bm.lock());

    let room = lock.get_room_mut(&user_id);
    if room.is_none() {
        return;
    }
    let room = room.unwrap();

    //判断房间状态
    if room.state != RoomState::ChoiceIndex {
        return;
    }
    let next_user = room.get_turn_user(None);
    if let Err(_) = next_user {
        return;
    }
    let next_user = next_user.unwrap();
    //判断是否轮到自己
    if next_user != user_id {
        return;
    }

    if room.battle_data.battle_id != task_batlte_id {
        return;
    }

    //移除玩家
    room.remove_member(MemberLeaveNoticeType::Kicked, &user_id, true);

    info!("定时检测选占位任务,没有选择都人T出去,user_id:{}", user_id);

    let is_empty = room.is_empty();
    let room_state = room.state;
    if is_empty || room_state == RoomState::BattleOvered {
        let room_id = room.get_room_id();
        lock.rm_room(room_id);
    }
    lock.player_room.remove(&user_id);
}

fn battle_turn_time(bm: Arc<Mutex<BattleMgr>>, task: Task) {
    let task_batlte_id = task.battle_id;
    let json_value = task.data;
    let turn = task.turn;
    let res = json_value.as_object();
    if res.is_none() {
        warn!("json_value.as_object() is None!");
        return;
    }
    let map = res.unwrap();
    let user_id = map.get("user_id");
    if user_id.is_none() {
        warn!("user_id is None!");
        return;
    }
    let user_id = user_id.unwrap().as_u64();
    if user_id.is_none() {
        warn!("user_id is None!");
        return;
    }
    let user_id = user_id.unwrap() as u32;

    let mut lock = block_on(bm.lock());

    let room = lock.get_room_mut(&user_id);
    if room.is_none() {
        return;
    }
    let room = room.unwrap();

    if room.battle_data.battle_id != task_batlte_id {
        return;
    }

    //校验房间状态
    if room.state != RoomState::BattleStarted {
        warn!(
            "battle_turn_time,the room state is not RoomState::BattleStarted!room_id:{},room_state:{:?}",
            room.get_room_id(),room.state
        );
        return;
    }

    //校验当前是不是这个人
    let next_user_id = room.get_turn_user(None);
    if let Err(e) = next_user_id {
        warn!("{:?}", e);
        return;
    }
    let next_user_id = next_user_id.unwrap();
    if next_user_id != user_id {
        return;
    }

    //如果不是同一个turn,就返回
    if room.battle_data.turn != turn {
        return;
    }

    info!("当前turn:{}", room.battle_data.turn);

    let battle_player = room.battle_data.get_battle_player(Some(user_id), true);
    if let Err(e) = battle_player {
        warn!("{:?}", e);
        return;
    }
    let battle_player = battle_player.unwrap();

    //机器人不用管
    if battle_player.is_robot() {
        return;
    }

    //如果玩家啥都没做，就T出房间
    let need_kick = battle_player.flow_data.open_map_cell_vec.is_empty();
    if need_kick {
        room.remove_member(MemberLeaveNoticeType::Kicked.into(), &user_id, true);
        info!("定时检turn任务,没有翻的人T出去,user_id:{}", user_id);
        let room_state = room.state;
        let is_empty = room.is_empty();
        if is_empty || room_state == RoomState::BattleOvered {
            let room_id = room.get_room_id();
            lock.rm_room(room_id);
        }
        lock.player_room.remove(&user_id);
    } else {
        let need_refresh_map = room.battle_data.check_refresh_map();
        //如果需要刷新地图，走地图刷新next turn逻辑
        if need_refresh_map {
            room.refresh_map();
            room.check_next_choice_index();
            info!("定时检测turn任务,刷新地图");
        } else {
            //如果用过移动点数就帮他跳过
            room.battle_data.next_turn(true);
            info!("定时检测turn任务,自动帮玩家跳过turn!user_id:{}", user_id);
        }
    }
}

fn max_battle_turn_limit(bm: Arc<Mutex<BattleMgr>>, task: Task) {
    let json_value = task.data;
    let res = json_value.as_object();
    if res.is_none() {
        return;
    }
    let map = res.unwrap();
    let user_id = map.get("user_id");
    if user_id.is_none() {
        return;
    }
    let user_id = user_id.unwrap().as_u64();
    if user_id.is_none() {
        return;
    }
    let user_id = user_id.unwrap() as u32;

    let mut lock = block_on(bm.lock());

    let room = lock.get_room_mut(&user_id);
    if room.is_none() {
        return;
    }
    let room = room.unwrap();
    let room_id = room.get_room_id();

    lock.rm_room(room_id);
}
