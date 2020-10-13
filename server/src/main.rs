mod db;
mod entity;
mod helper;
mod mgr;
mod net;
use crate::db::dbtool::DbPool;
use crate::mgr::game_mgr::GameMgr;
use crate::net::http::{SavePlayerHttpHandler, StopPlayerHttpHandler};
use crate::net::tcp_server;
use tools::thread_pool::MyThreadPool;

use std::sync::{Arc, Mutex};

use crate::mgr::timer_mgr::init_timer;
use log::{error, info};
use serde_json::Value;
use std::env;
use tools::conf::Conf;
use tools::http::HttpServerHandler;
use tools::my_log::init_log;
use tools::redis_pool::RedisPoolTool;
use tools::templates::template::{init_temps_mgr, TemplatesMgr};

#[macro_use]
extern crate lazy_static;

//初始化全局线程池
lazy_static! {

    ///线程池
    static ref THREAD_POOL: MyThreadPool = {
        let game_name = "game_name".to_string();
        let user_name = "user_name".to_string();
        let sys_name = "sys_name".to_string();
        let mtp = MyThreadPool::init(game_name, 8, user_name, 8, sys_name, 2);
        mtp
    };

    ///数据库链接池
    static ref DB_POOL: DbPool = {
        let db_pool = DbPool::new();
        db_pool
    };

    ///配置文件
    static ref CONF_MAP: Conf = {
        let path = env::current_dir().unwrap();
        let str = path.as_os_str().to_str().unwrap();
        let res = str.to_string()+"/config/config.conf";
        let conf = Conf::init(res.as_str());
        conf
    };

    ///静态配置文件
    static ref TEMPLATES: TemplatesMgr = {
        let path = env::current_dir().unwrap();
        let str = path.as_os_str().to_str().unwrap();
        let res = str.to_string()+"/template";
        let conf = init_temps_mgr(res.as_str());
        conf
    };

    ///reids客户端
    static ref REDIS_POOL:Arc<Mutex<RedisPoolTool>>={
        let add: &str = CONF_MAP.get_str("redis_add");
        let pass: &str = CONF_MAP.get_str("redis_pass");
        let redis = RedisPoolTool::init(add,pass);
        let redis:Arc<Mutex<RedisPoolTool>> = Arc::new(Mutex::new(redis));
        redis
    };
}
const REDIS_INDEX_USERS: u32 = 0;

const REDIS_KEY_USERS: &str = "users";

const REDIS_KEY_UID_2_PID: &str = "uid_2_pid";

const REDIS_INDEX_GAME_SEASON: u32 = 1;

const REDIS_KEY_GAME_SEASON: &str = "game_season";

///赛季结构体
#[derive(Default)]
pub struct Season {
    season_id: u32,
    last_update_time: u64,
    next_update_time: u64,
}

///赛季信息
pub static mut SEASON: Season = new_season();

pub const fn new_season() -> Season {
    let res = Season {
        season_id: 0,
        last_update_time: 0,
        next_update_time: 0,
    };
    res
}

///程序主入口,主要作用是初始化日志，数据库连接，redis连接，线程池，websocket，http
fn main() {
    let game_mgr = Arc::new(Mutex::new(GameMgr::new()));

    let info_log = CONF_MAP.get_str("info_log_path");
    let error_log = CONF_MAP.get_str("error_log_path");

    //初始化日志模块
    init_log(info_log, error_log);

    //初始化配置
    init_temps();

    //初始化定时器任务管理
    init_timer(game_mgr.clone());

    //初始化赛季
    init_season();

    //初始化http服务端
    init_http_server(game_mgr.clone());

    //初始化tcp服务端
    init_tcp_server(game_mgr.clone());
}

///初始化赛季信息
fn init_season() {
    let mut lock = REDIS_POOL.lock().unwrap();
    unsafe {
        let res: Option<String> = lock.hget(REDIS_INDEX_GAME_SEASON, REDIS_KEY_GAME_SEASON, "101");
        if let None = res {
            error!("redis do not has season data about game:{}", 101);
            return;
        }

        let str = res.unwrap();
        let value: Value = serde_json::from_str(str.as_str()).unwrap();
        let map = value.as_object().unwrap();
        let season_id = map.get("season_id").unwrap().as_u64().unwrap() as u32;
        let last_update_time: &str = map.get("last_update_time").unwrap().as_str().unwrap();
        let next_update_time: &str = map.get("next_update_time").unwrap().as_str().unwrap();
        let last_update_time =
            chrono::NaiveDateTime::parse_from_str(last_update_time, "%Y-%m-%d %H:%M:%S")
                .unwrap()
                .timestamp() as u64;

        let next_update_time =
            chrono::NaiveDateTime::parse_from_str(next_update_time, "%Y-%m-%d %H:%M:%S")
                .unwrap()
                .timestamp() as u64;
        SEASON.season_id = season_id;
        SEASON.last_update_time = last_update_time;
        SEASON.next_update_time = next_update_time;
    }
}

fn init_temps() {
    let time = std::time::SystemTime::now();
    lazy_static::initialize(&TEMPLATES);
    let spend_time = time.elapsed().unwrap().as_millis();
    info!("初始化templates成功!耗时:{}ms", spend_time);
}

///初始化http服务端
fn init_http_server(gm: Arc<Mutex<GameMgr>>) {
    let mut http_vec: Vec<Box<dyn HttpServerHandler>> = Vec::new();
    http_vec.push(Box::new(SavePlayerHttpHandler::new(gm.clone())));
    http_vec.push(Box::new(StopPlayerHttpHandler::new(gm.clone())));
    let http_port: &str = CONF_MAP.get_str("http_port");
    async_std::task::spawn(tools::http::http_server(http_port, http_vec));
}

///init tcp server
fn init_tcp_server(gm: Arc<Mutex<GameMgr>>) {
    let tcp_port: &str = CONF_MAP.get_str("tcp_port");
    tcp_server::new(tcp_port, gm);
}
