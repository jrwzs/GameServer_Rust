use super::*;
use async_std::sync::Mutex;
use async_std::task::block_on;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use log::error;
use tools::cmd_code::{ClientCode, RoomCode, ServerCommonCode};

pub enum TcpClientType {
    GameServer,
    GameCenter,
}

pub struct TcpClientHandler {
    client_type: TcpClientType,
    ts: Option<Sender<Vec<u8>>>,
    cp: Arc<Mutex<ChannelMgr>>,
}

impl TcpClientHandler {
    pub fn new(cp: Arc<Mutex<ChannelMgr>>, client_type: TcpClientType) -> TcpClientHandler {
        let tch = TcpClientHandler {
            ts: None,
            cp,
            client_type,
        };
        tch
    }

    ///数据包转发
    fn arrange_packet(&mut self, packet: Packet) {
        let cmd = packet.get_cmd();
        let mut lock = block_on(self.cp.lock());
        //转发到游戏服
        if (cmd == ServerCommonCode::ReloadTemps.into_u32()
            || cmd == ServerCommonCode::UpdateSeason.into_u32())
            || (cmd >= GameCode::Min.into_u32() && cmd <= GameCode::Max.into_u32())
        {
            lock.write_to_game(packet);
            return;
        }
        //转发到房间服
        if cmd >= RoomCode::Min as u32 && cmd <= RoomCode::Max as u32 {
            lock.write_to_game_center(packet);
            return;
        }
    }
}

#[async_trait]
impl ClientHandler for TcpClientHandler {
    async fn on_open(&mut self, ts: Sender<Vec<u8>>) {
        match self.client_type {
            TcpClientType::GameServer => {
                block_on(self.cp.lock()).set_game_client_channel(ts.clone());
            }
            TcpClientType::GameCenter => {
                block_on(self.cp.lock()).set_game_center_client_channel(ts.clone());
            }
        }
        self.ts = Some(ts);
    }

    async fn on_close(&mut self) {
        let address: Option<&str>;
        match self.client_type {
            TcpClientType::GameServer => {
                address = Some(CONF_MAP.get_str("game_port"));
            }
            TcpClientType::GameCenter => {
                address = Some(CONF_MAP.get_str("game_center_port"));
            }
        }
        self.on_read(address.unwrap().to_string()).await;
    }

    async fn on_message(&mut self, mess: Vec<u8>) {
        let packet_array = Packet::build_array_from_server(mess);

        if let Err(e) = packet_array {
            error!("{:?}", e.to_string());
            return;
        }
        let packet_array = packet_array.unwrap();

        for mut packet in packet_array {
            //判断是否是发给客户端消息
            if packet.is_client() && packet.get_cmd() > 0 {
                let mut lock = block_on(self.cp.lock());
                let gate_user = lock.get_mut_user_channel_channel(&packet.get_user_id());
                match gate_user {
                    Some(user) => {
                        user.get_tcp_mut_ref().send(packet.build_client_bytes());
                        info!(
                            "回给客户端消息,user_id:{},cmd:{}",
                            packet.get_user_id(),
                            packet.get_cmd(),
                        );
                    }
                    None => {
                        if packet.get_cmd() == ClientCode::LeaveRoom.into_u32()
                            || packet.get_cmd() == ClientCode::MemberLeaveNotice.into_u32()
                        {
                            continue;
                        }
                        warn!(
                            "user data is null,id:{},cmd:{}",
                            &packet.get_user_id(),
                            packet.get_cmd()
                        );
                    }
                }
            } else {
                //判断是否要转发到其他服务器进程消息
                self.arrange_packet(packet);
            }
        }
    }
}
