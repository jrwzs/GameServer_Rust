use crate::CONF_MAP;
use log::info;
use mysql::{Error, Params, Pool, QueryResult, Value};

pub struct DbPool {
    pub pool: Pool,
}

impl DbPool {
    ///创建一个db结构体
    pub fn new() -> DbPool {
        let str: &str = CONF_MAP.get_str("mysql");
        let pool = mysql::Pool::new(str).unwrap();
        info!("初始化dbpool完成!");
        DbPool { pool: pool }
    }

    ///执行sql
    pub fn exe_sql(
        &self,
        sql: &str,
        params: Option<Vec<Value>>,
    ) -> Result<QueryResult<'static>, Error> {
        match params {
            Some(params) => self.pool.prep_exec(sql, Params::Positional(params)),
            None => self.pool.prep_exec(sql, ()),
        }
    }
}
