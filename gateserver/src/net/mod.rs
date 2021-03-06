pub mod http;
pub mod tcp_client;
pub mod tcp_server;
pub mod websocket;
use crate::{CONF_MAP, REDIS_INDEX_USERS, REDIS_KEY_UID_2_PID};
use crate::{REDIS_KEY_USERS, REDIS_POOL};
use log::{debug, error, info, warn};
use protobuf::Message;
use std::sync::Arc;
use tools::tcp::ClientHandler;

use crate::mgr::channel_mgr::ChannelMgr;

use tools::util::packet::Packet;

use tools::cmd_code::GameCode;
use tools::protos::protocol::{C_USER_LOGIN, S_USER_LOGIN};

use async_std::sync::{Mutex, MutexGuard};
use serde_json::Value;
use std::str::FromStr;

type Lock = Arc<Mutex<ChannelMgr>>;

///从redis查找user_id
pub fn query_pid_from_redis(user_id: u32) -> anyhow::Result<String> {
    let user_id_str = user_id.to_string();
    //校验用户中心是否登陆过，如果有，则不往下执行
    let mut redis_write = REDIS_POOL.lock().unwrap();
    let res: Option<String> =
        redis_write.hget(REDIS_INDEX_USERS, REDIS_KEY_UID_2_PID, user_id_str.as_str());
    if res.is_none() {
        anyhow::bail!("this account is invalid!user_id:{:?}", user_id)
    }
    let pid = res.unwrap();
    Ok(pid)
}

///从redis查找user_id
pub fn query_user_id_from_redis(platform_value: &str) -> anyhow::Result<u32> {
    //校验用户中心是否登陆过，如果有，则不往下执行
    let mut redis_write = REDIS_POOL.lock().unwrap();
    let res: Option<String> = redis_write.hget(REDIS_INDEX_USERS, REDIS_KEY_USERS, platform_value);
    if res.is_none() {
        anyhow::bail!(
            "this account is invalid!platform_value:{:?}",
            platform_value
        )
    }
    let json_value = res.unwrap();

    let json_value = Value::from_str(json_value.as_str());
    match json_value {
        Ok(json_value) => {
            let user_id = json_value["user_id"].as_u64();
            match user_id {
                Some(user_id) => {
                    return Ok(user_id as u32);
                }
                None => {
                    anyhow::bail!(
                        "this account is invalid!platform_value:{:?}",
                        platform_value
                    )
                }
            }
        }
        Err(e) => anyhow::bail!("{:?}", e.to_string()),
    }
}

///校验用户中心是否在线
fn check_uc_online(user_id: &u32) -> anyhow::Result<bool> {
    //校验用户中心是否登陆过，如果有，则不往下执行
    let mut redis_write = REDIS_POOL.lock().unwrap();
    let pid: Option<String> = redis_write.hget(
        REDIS_INDEX_USERS,
        REDIS_KEY_UID_2_PID,
        user_id.to_string().as_str(),
    );
    if pid.is_none() {
        anyhow::bail!("this user_id is invalid!user_id:{}", user_id)
    }
    let pid = pid.unwrap();
    let res: Option<String> = redis_write.hget(0, REDIS_KEY_USERS, pid.as_str());
    if res.is_none() {
        anyhow::bail!("this user_id is invalid!user_id:{}", user_id)
    }
    let res = res.unwrap();
    let json = Value::from_str(res.as_str());
    match json {
        Ok(json_value) => {
            let bool_res = json_value["on_line"].as_bool();
            if bool_res.is_some() && bool_res.unwrap() {
                return Ok(true);
            } else {
                return Ok(false);
            }
        }
        Err(e) => anyhow::bail!("{:?}", e.to_string()),
    }
}

///校验内存是否在线，并做处理
fn check_mem_online(user_id: &u32, write: &mut MutexGuard<ChannelMgr>) -> bool {
    //校验内存是否已经登陆
    let gate_user = write.get_mut_user_channel(user_id);
    let mut res: bool = false;
    //如果有，则执行T下线
    if gate_user.is_some() {
        // let token = gate_user.unwrap().get_token();
        // write.close_remove(&token);
        res = true;
    }
    res
}
