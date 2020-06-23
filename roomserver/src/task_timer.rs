use crate::entity::member::MemberState;
use crate::entity::room_model::{RoomCache, RoomModel};
use crate::handlers::room_handler::emoji;
use crate::mgr::room_mgr::RoomMgr;
use log::{error, info, warn};
use protobuf::Message;
use serde_json::Value as JsonValue;
use std::sync::mpsc::{channel, sync_channel, Receiver};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tools::cmd_code::ClientCode;
use tools::protos::room::S_START;
use tools::util::packet::Packet;

pub enum TaskCmd {
    MatchRoomStart = 101, //匹配房间开始任务
}

impl From<u16> for TaskCmd {
    fn from(v: u16) -> Self {
        if v == TaskCmd::MatchRoomStart as u16 {
            return TaskCmd::MatchRoomStart;
        }
        TaskCmd::MatchRoomStart
    }
}

#[derive(Debug, Clone, Default)]
pub struct Task {
    pub cmd: u16,        //要执行的命令
    pub delay: u64,      //要延迟执行的时间
    pub data: JsonValue, //数据
}

///初始化定时执行任务
pub fn init_timer(rm: Arc<RwLock<RoomMgr>>) {
    let m = move || {
        let (sender, rec) = sync_channel(512);
        let mut write = rm.write().unwrap();
        write.task_sender = Some(sender);
        std::mem::drop(write);
        loop {
            let res = rec.recv();
            if res.is_err() {
                error!("{:?}", res.err().unwrap());
                continue;
            }
            let task = res.unwrap();
            let task_cmd = TaskCmd::from(task.cmd);
            let rm_clone = rm.clone();
            match task_cmd {
                TaskCmd::MatchRoomStart => {
                    let m = move || {
                        std::thread::sleep(Duration::from_secs(task.delay));
                        match_room_start(rm_clone, task);
                    };
                    std::thread::spawn(m);
                }
                _ => {}
            }
        }
    };
    std::thread::spawn(m);
    info!("初始化定时器任务执行器成功!");
}

///执行匹配房间任务
fn match_room_start(rm: Arc<RwLock<RoomMgr>>, task: Task) {
    let json_value = task.data;
    let res = json_value.as_object();
    if res.is_none() {
        return;
    }
    let map = res.unwrap();
    let battle_type = map.get("battle_type");
    if battle_type.is_none() {
        return;
    }
    let battle_type = battle_type.unwrap().as_u64();
    if battle_type.is_none() {
        return;
    }
    let battle_type = battle_type.unwrap() as u8;

    let room_id = map.get("room_id");
    if room_id.is_none() {
        return;
    }
    let room_id = room_id.unwrap();
    let room_id = room_id.as_u64();
    if room_id.is_none() {
        return;
    }
    let room_id = room_id.unwrap() as u32;

    let mut write = rm.write().unwrap();
    let match_room = write.match_rooms.get_match_room_mut(&battle_type);

    let room = match_room.get_room_mut(&room_id);
    if room.is_none() {
        return;
    }
    let room = room.unwrap();

    let mut v = Vec::new();
    for member in room.members.values() {
        if member.state == MemberState::NotReady as u8 {
            v.push(member.user_id);
        }
    }
    if v.len() > 0 {
        for member_id in &v[..] {
            let res = match_room.leave_room(&room_id, member_id);
            if res.is_err() {
                error!("{:?}", res.err().unwrap());
            }
        }
        return;
    }

    // let mut ss = S_START::new();
    // ss.is_succ = true;
    // for member_id in room.members.keys() {
    //     let bytes = Packet::build_packet_bytes(
    //         ClientCode::Start as u32,
    //         *member_id,
    //         ss.write_to_bytes().unwrap(),
    //         true,
    //         true,
    //     );
    //     room.sender.write(bytes);
    // }
}
