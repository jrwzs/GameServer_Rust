use crate::net::Forward;
use crate::Lock;
use async_trait::async_trait;
use log::{error, info};
use tools::net_message_io::NetHandler;
use tools::util::packet::Packet;

use super::new_gate_server_tcp;

///处理客户端所有请求,每个客户端单独分配一个handler
#[derive(Clone)]
pub struct GateTcpServerHandler {
    pub token: usize,
    pub gm: Lock,
}

unsafe impl Send for GateTcpServerHandler {}

unsafe impl Sync for GateTcpServerHandler {}

impl Forward for GateTcpServerHandler {
    fn get_battle_token(&self) -> Option<usize> {
        None
    }

    fn get_gate_token(&self) -> Option<usize> {
        Some(self.token)
    }

    fn get_game_center_mut(&mut self) -> &mut Lock {
        &mut self.gm
    }
}

#[async_trait]
impl tools::net_message_io::MessageHandler for GateTcpServerHandler {
    async fn try_clone(&self) -> Self {
        self.clone()
    }

    ///客户端tcp链接激活事件
    async fn on_open(&mut self, net_handler: NetHandler) {
        self.token = net_handler.endpoint.resource_id().raw();
        self.gm.lock().await.add_gate_client(net_handler);
        info!("new gate_client is connect!token:{}", self.token);
    }

    ///客户端tcp链接关闭事件
    async fn on_close(&mut self) {
        let token = self.token;
        let mut lock = self.gm.lock().await;
        //删除玩家对应的battle服
        let mut remove_vec = vec![];
        for (&user_id, &token_value) in lock.user_w_gate.iter() {
            if token_value != token {
                continue;
            }
            remove_vec.push(user_id);
        }
        for user_id in remove_vec {
            lock.user_w_gate.remove(&user_id);
        }
        //删除gate服
        lock.gate_clients.remove(&token);
        info!("gate_client is closed!token:{}", token);
    }

    ///客户端读取事件
    async fn on_message(&mut self, mess: &[u8]) {
        let packet_array = Packet::build_array_from_server(mess.to_vec());

        if let Err(e) = packet_array {
            error!("{:?}", e);
            return;
        }
        let packet_array = packet_array.unwrap();
        self.forward_packet(packet_array).await;
    }
}

///创建新的tcp服务器,如果有问题，终端进程
pub fn new(address: String, rm: Lock) {
    let sh = GateTcpServerHandler { token: 0, gm: rm };
    let m = new_gate_server_tcp(address, sh);
    async_std::task::spawn(m);
}
