use crossbeam::channel::Sender;
use log::warn;
use protobuf::Message;
use serde_json::Value;
use std::collections::HashMap;
use tools::cmd_code::{BattleCode, GameCode, GateCode, RankCode, RoomCode, ServerCommonCode};
use tools::net_message_io::NetHandler;
use tools::protos::server_protocol::{R_B_START, R_S_UPDATE_SEASON, UPDATE_WORLD_BOSS_PUSH};
use tools::util::packet::Packet;

#[derive(Default)]
pub struct GameCenterMgr {
    pub rank_server: Option<Sender<Vec<u8>>>,         //排行榜服
    pub room_center: Option<Sender<Vec<u8>>>,         //房间中心
    pub gate_clients: HashMap<usize, GateClient>,     //gate路由服客户端,key:token,value:GateClient
    pub battle_clients: HashMap<usize, BattleClient>, //战斗服客户端,key:token,value:BattleClient
    pub user_w_gate: HashMap<u32, usize>,             //玩家对应gate
    pub user_w_battle: HashMap<u32, usize>,           //玩家对应战斗服
}

impl GameCenterMgr {
    pub fn new() -> Self {
        GameCenterMgr::default()
    }

    ///通知更新服务器更新赛季
    pub fn notice_update_season(&mut self, value: Value) {
        let map = value.as_object();
        if let None = map {
            return;
        }
        let map = map.unwrap();
        let season_id = map.get("season_id");
        if season_id.is_none() {
            warn!("the season_id is None!");
            return;
        }
        let season_id = season_id.unwrap();

        let round = map.get("round");
        if round.is_none() {
            warn!("the round is None!");
            return;
        }
        let round = round.unwrap();

        let next_update_time = map.get("next_update_time");
        if next_update_time.is_none() {
            warn!("the next_update_time is None!");
            return;
        }
        let next_update_time = next_update_time.unwrap().as_u64();
        if next_update_time.is_none() {
            warn!("the next_update_time is None!");
            return;
        }
        let next_update_time = next_update_time.unwrap();

        let mut usn = R_S_UPDATE_SEASON::new();
        usn.set_season_id(season_id.as_u64().unwrap() as i32);
        usn.set_round(round.as_u64().unwrap() as u32);
        usn.set_next_update_time(next_update_time);

        let cmd = RankCode::UpdateSeasonPush.into_u32();
        let mut packet = Packet::new(cmd, 0, 0);
        packet.set_is_client(false);
        packet.set_is_broad(true);
        packet.set_data(&usn.write_to_bytes().unwrap()[..]);
        let bytes = packet.build_server_bytes();
        //通知排行榜服
        let res = self.get_rank_center_mut().send(bytes);
        if let Err(e) = res {
            warn!("{:?}", e);
        }
    }

    ///通知更新服务器更新赛季
    pub fn notice_update_worldboss(&mut self, value: Value) {
        let map = value.as_object();
        if let None = map {
            return;
        }
        let map = map.unwrap();
        let world_boss_id = map.get("cter_id");
        if world_boss_id.is_none() {
            warn!("the worldboss_id is None!");
            return;
        }
        let world_boss_id = world_boss_id.unwrap();
        let world_boss_id = world_boss_id.as_u64().unwrap() as i32;

        let next_update_time = map.get("next_update_time");
        if next_update_time.is_none() {
            warn!("the next_update_time is None!");
            return;
        }
        let next_update_time = next_update_time.unwrap().as_u64();
        if next_update_time.is_none() {
            warn!("the next_update_time is None!");
            return;
        }
        let next_update_time = next_update_time.unwrap();

        let mut usn = UPDATE_WORLD_BOSS_PUSH::new();
        usn.set_world_boss_id(world_boss_id);
        usn.set_next_update_time(next_update_time);

        let mut packet = Packet::new(RoomCode::UpdateWorldBossPush.into_u32(), 0, 0);
        packet.set_is_client(false);
        packet.set_is_broad(true);
        packet.set_data(&usn.write_to_bytes().unwrap()[..]);
        let bytes = packet.build_server_bytes();
        let _ = self.get_room_center_mut().send(bytes);

        packet.set_cmd(GameCode::UpdateWorldBossPush.into_u32());
        let bytes = packet.build_server_bytes();
        //推送给所有游戏服
        for gate_client in self.gate_clients.values_mut() {
            gate_client.send(bytes.as_slice());
        }
    }

    ///停服
    pub fn stop_all_server_handler(&mut self) {
        let bytes =
            Packet::build_packet_bytes(GateCode::StopServer.into_u32(), 0, Vec::new(), true, false);
        let bytes = bytes.as_slice();
        for gate_client in self.gate_clients.values() {
            gate_client.send(bytes);
        }
    }

    pub fn kick_player_handler(&mut self, user_id: u32) {
        if user_id == 0 {
            return;
        }

        let bytes = Packet::build_packet_bytes(
            GateCode::KickPlayer.into_u32(),
            user_id,
            Vec::new(),
            true,
            false,
        );
        let bytes = bytes.as_slice();
        for gate_client in self.gate_clients.values() {
            gate_client.send(bytes);
        }
    }

    pub fn notice_reload_temps(&mut self) {
        let bytes = Packet::build_packet_bytes(
            ServerCommonCode::ReloadTemps.into_u32(),
            0,
            Vec::new(),
            true,
            false,
        );
        let bytes_slice = bytes.as_slice();
        //通知gate reload_temps
        for gate_client in self.gate_clients.values_mut() {
            gate_client.send(bytes_slice);
        }

        //通知战斗服
        for battle_client in self.battle_clients.values_mut() {
            battle_client.send(bytes_slice);
        }

        //通知房间服
        let res = self.get_room_center_mut().send(bytes);
        if let Err(e) = res {
            warn!("{:?}", e);
        }
    }

    pub fn handler(&mut self, packet: &Packet, gate_token: Option<usize>) {
        let cmd = packet.get_cmd();
        //开始战斗,负载均衡，分配战斗服务器
        if cmd == BattleCode::Start.into_u32() {
            self.slb(packet.clone());
        }
        //绑定玩家到gate
        let user_id = packet.get_user_id();
        if user_id <= 0 {
            return;
        }
        if gate_token.is_none() {
            return;
        }
        self.bound_user_w_gate(user_id, gate_token.unwrap());
    }

    ///将玩家绑定到路由服
    pub fn bound_user_w_gate(&mut self, user_id: u32, token: usize) {
        if user_id <= 0 {
            return;
        }

        let res = self.user_w_gate.get(&user_id);
        match res {
            Some(t) => {
                let t = *t;
                if t != token {
                    self.user_w_gate.insert(user_id, token);
                }
            }
            None => {
                self.user_w_gate.insert(user_id, token);
            }
        }
    }

    ///负载均衡
    pub fn slb(&mut self, packet: Packet) {
        let mut proto = R_B_START::new();
        let res = proto.merge_from_bytes(packet.get_data());
        if let Err(e) = res {
            warn!("{:?}", e);
            return;
        }

        //找出房间数最小的那个服，若没有，则默认第一个
        let bc_res = self
            .battle_clients
            .values_mut()
            .min_by(|x, y| x.room_num.cmp(&y.room_num));
        if let None = bc_res {
            warn!("could not find min room num of battle server!");
            return;
        }
        let bc_res = bc_res.unwrap();
        let battle_token = bc_res.net_handler.endpoint.resource_id().raw();
        for member in proto.get_room_pt().members.iter() {
            let user_id = member.user_id;
            self.user_w_battle.insert(user_id, battle_token);
        }
        bc_res.room_num += 1;
    }

    ///玩家离开
    pub fn user_leave(&mut self, cmd: u32, user_id: u32) {
        if cmd == GameCode::UnloadUser.into_u32() {
            self.user_w_battle.remove(&user_id);
            self.user_w_gate.remove(&user_id);
        }
    }

    ///负载均衡资源回收
    pub fn slb_back(&mut self, cmd: u32, battle_token: Option<usize>) {
        if cmd == RoomCode::Summary.into_u32() && battle_token.is_some() {
            let battle_client = self.battle_clients.get_mut(&battle_token.unwrap());
            if let Some(battle_client) = battle_client {
                if battle_client.room_num > 0 {
                    battle_client.room_num -= 1;
                }
            }
        }
    }

    pub fn get_room_center_mut(&mut self) -> &mut Sender<Vec<u8>> {
        self.room_center.as_mut().unwrap()
    }

    pub fn get_rank_center_mut(&mut self) -> &mut Sender<Vec<u8>> {
        self.rank_server.as_mut().unwrap()
    }

    pub fn get_gate_client(&self, user_id: u32) -> anyhow::Result<&GateClient> {
        let res = self.user_w_gate.get(&user_id);
        if let None = res {
            anyhow::bail!("could not find gate's token by user_id:{}!", user_id)
        }
        let token = res.unwrap();
        let res = self.gate_clients.get(token);
        if let None = res {
            anyhow::bail!("could not find GateClient by token:{}!", token)
        }
        let gc = res.unwrap();
        Ok(gc)
    }

    pub fn get_battle_client(&self, user_id: u32) -> anyhow::Result<&BattleClient> {
        let res = self.user_w_battle.get(&user_id);
        if let None = res {
            anyhow::bail!("could not find battle's token by user_id:{}!", user_id)
        }
        let token = res.unwrap();
        let res = self.battle_clients.get(token);
        if let None = res {
            anyhow::bail!("could not find BattleClient by token:{}!", token)
        }
        let gc = res.unwrap();
        Ok(gc)
    }

    pub fn set_room_sender(&mut self, sender: Sender<Vec<u8>>) {
        self.room_center = Some(sender);
    }

    pub fn set_rank_sender(&mut self, sender: Sender<Vec<u8>>) {
        self.rank_server = Some(sender);
    }

    pub fn add_gate_client(&mut self, net_handler: NetHandler) {
        let key = net_handler.endpoint.resource_id().raw();
        let gc = GateClient::new(net_handler);
        self.gate_clients.insert(key, gc);
    }

    pub fn add_battle_client(&mut self, net_handler: NetHandler) {
        let key = net_handler.endpoint.resource_id().raw();
        let rc = BattleClient::new(net_handler);
        self.battle_clients.insert(key, rc);
    }
}

pub struct GateClient {
    net_handler: NetHandler,
}

impl GateClient {
    pub fn new(net_handler: NetHandler) -> Self {
        let gc = GateClient { net_handler };
        gc
    }

    pub fn send(&self, bytes: &[u8]) {
        let endpoint = self.net_handler.endpoint;
        self.net_handler
            .node_handler
            .network()
            .send(endpoint, bytes);
    }
}

pub struct BattleClient {
    net_handler: NetHandler,
    room_num: u32,
}

impl BattleClient {
    pub fn new(net_handler: NetHandler) -> Self {
        let rc = BattleClient {
            net_handler,
            room_num: 0,
        };
        rc
    }

    pub fn send(&self, bytes: &[u8]) {
        let endpoint = self.net_handler.endpoint;
        self.net_handler
            .node_handler
            .network()
            .send(endpoint, bytes);
    }
}
