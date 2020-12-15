use crate::entity::gateuser::GateUser;
use log::{error, info, warn};
use protobuf::Message;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;
use tools::cmd_code::{GameCode, RoomCode};
use tools::protos::server_protocol::UPDATE_SEASON_NOTICE;
use tools::tcp::TcpSender;
use tools::util::packet::Packet;
use ws::Sender as WsSender;

///channel管理结构体
pub struct ChannelMgr {
    //游戏服tcpstream
    pub game_client_channel: Option<TcpStream>,
    //房间服stream
    pub room_client_channel: Option<TcpStream>,
    //玩家channels
    pub user_channel: HashMap<u32, GateUser>,
    //token,user_id
    pub channels: HashMap<usize, u32>,
}

impl ChannelMgr {
    ///创建channelmgr结构体
    pub fn new() -> Self {
        let players: HashMap<u32, GateUser> = HashMap::new();
        let cm = ChannelMgr {
            game_client_channel: None,
            room_client_channel: None,
            user_channel: players,
            channels: HashMap::new(),
        };
        cm
    }

    pub fn set_game_client_channel(&mut self, ts: TcpStream) {
        self.game_client_channel = Some(ts);
    }

    pub fn set_room_client_channel(&mut self, ts: TcpStream) {
        self.room_client_channel = Some(ts);
    }

    ///处理离线事件
    /// token：sender唯一标识
    pub fn off_line(&mut self, token: usize) {
        let user_id = self.get_channels_user_id(&token);
        match user_id {
            Some(user_id) => {
                let user_id = *user_id;
                self.notice_off_line(user_id, &token);
            }
            None => {
                warn!("user_id is none for token:{},so nothing to do!", token);
            }
        }
    }

    ///通知下线
    fn notice_off_line(&mut self, user_id: u32, token: &usize) {
        //关闭连接
        self.close_remove(token);
        //初始化包
        let mut packet = Packet::default();
        packet.set_user_id(user_id);
        packet.set_len(14_u32);
        packet.set_is_client(false);
        packet.set_is_broad(false);

        //发给游戏服
        packet.set_cmd(GameCode::LineOff as u32);
        self.write_to_game(packet.clone());
        //发给房间服
        packet.set_cmd(RoomCode::LineOff as u32);
        self.write_to_room(packet);
    }

    ///写到游戏服
    pub fn write_to_game(&mut self, packet: Packet) {
        if self.game_client_channel.is_none() {
            error!("disconnect with Game-Server,pls connect Game-Server before send packet!");
            return;
        }
        let gc = self.game_client_channel.as_mut().unwrap();
        let size = gc.write(&packet.build_server_bytes()[..]);
        match size {
            Ok(_) => {
                let res = gc.flush();
                if let Err(e) = res {
                    error!("flush has error!mess:{:?}", e);
                }
            }
            Err(e) => {
                error!("{:?}", e.to_string());
                return;
            }
        }
    }

    ///写到房间服
    #[warn(unused_must_use)]
    pub fn write_to_room(&mut self, packet: Packet) {
        if self.room_client_channel.is_none() {
            error!("disconnect with Room-Server,pls connect Room-Server before send packet!");
            return;
        }
        let rc = self.room_client_channel.as_mut().unwrap();
        let size = rc.write(&packet.build_server_bytes()[..]);
        match size {
            Ok(_) => {
                let res = rc.flush();
                if let Err(e) = res {
                    error!("{:?}", e);
                }
            }
            Err(e) => {
                error!("{:?}", e);
                return;
            }
        }
    }

    //添加gateuser
    pub fn add_gate_user(
        &mut self,
        user_id: u32,
        ws: Option<Arc<WsSender>>,
        tcp: Option<TcpSender>,
    ) {
        let mut token = 0;
        if ws.is_some() {
            token = ws.as_ref().unwrap().token().0;
        }
        if tcp.is_some() {
            token = tcp.as_ref().unwrap().token;
        }
        self.insert_channels(token, user_id);
        self.insert_user_channel(user_id, GateUser::new(user_id, ws, tcp));
    }

    ///插入channel,key：userid,v:channel
    pub fn insert_user_channel(&mut self, token: u32, gate_user: GateUser) {
        self.user_channel.insert(token, gate_user);
    }
    ///插入token-userid的映射
    pub fn insert_channels(&mut self, token: usize, user_id: u32) {
        self.channels.insert(token, user_id);
    }
    ///获得玩家channel k:userid
    pub fn get_user_channel(&self, user_id: &u32) -> Option<&GateUser> {
        self.user_channel.get(user_id)
    }

    ///根据token获得userid
    pub fn get_channels_user_id(&self, token: &usize) -> Option<&u32> {
        self.channels.get(token)
    }

    ///通过userid获得channel
    pub fn get_mut_user_channel_channel(&mut self, user_id: &u32) -> Option<&mut GateUser> {
        self.user_channel.get_mut(user_id)
    }

    ///关闭channel句柄，并从内存中删除
    pub fn close_remove(&mut self, token: &usize) {
        let user_id = self.channels.remove(token);
        if user_id.is_none() {
            info!("channel_mgr:user_id is none for token:{}", token);
            return;
        }
        let user_id = &user_id.unwrap();
        let gate_user = self.user_channel.get_mut(user_id);
        if gate_user.is_none() {
            info!("channel_mgr:gate_user is none for user_id:{}", user_id);
            return;
        }
        gate_user.unwrap().close();
        self.user_channel.remove(user_id);
        info!("channel_mgr:玩家断开连接，关闭句柄释放资源：{}", user_id);
    }

    ///T掉所有玩家
    pub fn kick_all(&mut self) {
        let res = self.channels.clone();
        for (token, user_id) in res.iter() {
            self.notice_off_line(*user_id, token);
        }
    }

    ///通知热更静态配置
    pub fn notice_reload_temps(&mut self) {
        let mut packet = Packet::new(GameCode::ReloadTemps as u32, 0, 0);
        packet.set_is_client(false);
        packet.set_is_broad(false);
        self.write_to_game(packet.clone());
        packet.set_cmd(RoomCode::ReloadTemps as u32);
        self.write_to_room(packet);
    }

    ///通知更新服务器更新赛季
    pub fn notice_update_season(&mut self, value: Value) {
        let mut packet = Packet::new(GameCode::UpdateSeason as u32, 0, 0);
        packet.set_is_client(false);
        packet.set_is_broad(true);
        let map = value.as_object();
        if let None = map {
            return;
        }
        let map = map.unwrap();
        let season_id = map.get("season_id");
        if season_id.is_none() {
            return;
        }
        let season_id = season_id.unwrap();
        let last_update_time = map.get("last_update_time");
        if last_update_time.is_none() {
            return;
        }
        let last_update_time = last_update_time.unwrap();

        let next_update_time = map.get("next_update_time");
        if next_update_time.is_none() {
            return;
        }
        let next_update_time = next_update_time.unwrap();

        let mut usn = UPDATE_SEASON_NOTICE::new();
        usn.set_season_id(season_id.as_u64().unwrap() as u32);
        usn.set_last_update_time(last_update_time.to_string());
        usn.set_next_update_time(next_update_time.to_string());
        packet.set_data(&usn.write_to_bytes().unwrap()[..]);
        self.write_to_game(packet.clone());
        packet.set_cmd(RoomCode::UpdateSeason as u32);
        self.write_to_room(packet);
    }
}
