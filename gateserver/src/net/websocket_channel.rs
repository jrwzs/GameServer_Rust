use super::*;

pub struct WebSocketChannel {
    pub user_id: u32,
    pub sender: Arc<WsSender>,
}

impl WebSocketChannel {
    #[warn(unused_must_use)]
    pub fn init(user_id: u32, sender: Arc<WsSender>) -> Self {
        WebSocketChannel {
            user_id: user_id,
            sender: sender,
        }
    }
}
