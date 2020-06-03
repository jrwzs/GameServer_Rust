use crate::templates::template::{Template, TemplateMgrTrait};
use std::collections::HashMap;
use crate::result::errors::Error;
use crate::result::errors::Result;

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
pub struct TileMapTemp {
    pub id: u32,
    pub map: Vec<u32>,
    pub cell_type: Vec<CellType>,
    pub world_cell: Vec<WorldCell>,
    pub map_type: u8,
}

impl Template for TileMapTemp {}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
pub struct CellType {
    pub cell_id: u32,
    pub count: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
pub struct WorldCell {
    pub name: String,
    pub x_index: u32,
    pub y_index: u32,
    pub buff_id: Vec<u32>,
}

#[derive(Debug, Default, Clone)]
pub struct TileMapTempMgr {
    pub temps: HashMap<u32, TileMapTemp>,
}

impl TileMapTempMgr {
    pub fn get_temp(&self, map_id: u32) -> Result<&TileMapTemp> {
        let res = self.temps.get(&map_id);
        if res.is_none(){
            let str = format!("TileMapTemp is none for map_id:{}",map_id);
            return error_chain::bail!(str);
        };
        Ok(res.unwrap())
    }

    pub fn init(&mut self, t: Vec<TileMapTemp>) {
        for tt in t {
            self.temps.insert(tt.id, tt);
        }
    }
}

impl TemplateMgrTrait for TileMapTempMgr {
    fn is_empty(&self) -> bool {
        self.temps.is_empty()
    }
}