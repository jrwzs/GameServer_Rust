use crate::templates::template::{Template, TemplateMgrTrait};
use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
pub struct ConstantTemp {
    pub id: String,
    pub value: String,
}

impl Template for ConstantTemp {}

///常量结构体管理器
#[derive(Debug, Default, Clone)]
pub struct ConstantTempMgr {
    pub temps: HashMap<String, ConstantTemp>,
}

impl ConstantTempMgr {
    pub fn init(&mut self, t: Vec<ConstantTemp>) {
        for tt in t {
            let key = tt.id.clone();
            self.temps.insert(key, tt);
        }
    }
}

impl TemplateMgrTrait for ConstantTempMgr {
    fn is_empty(&self) -> bool {
        self.temps.is_empty()
    }

    fn clear(&mut self) {
        self.temps.clear();
    }
}
