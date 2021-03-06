use crate::robot::{RememberCell, RobotData};
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use rand::Rng;

///触发器类型
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum RobotTriggerType {
    None = 0,
    SeeMapCell = 1,  //看到地图块
    MapCellPair = 2, //配对地图块
}
impl Default for RobotTriggerType {
    fn default() -> Self {
        RobotTriggerType::None
    }
}

impl RobotTriggerType {
    pub fn into_u8(self) -> u8 {
        let value: u8 = self.into();
        value
    }
}

impl RobotData {
    pub fn trigger_see_map_cell(&mut self, rc: RememberCell) {
        let size = self.remember_map_cell.len();
        let max_size = self.remember_size as usize;
        //如果这个块已经被记忆，则刷新位置
        let res = self
            .remember_map_cell
            .iter()
            .enumerate()
            .find(|(_, re)| re.cell_index == rc.cell_index);

        if let Some((rm_index, _)) = res {
            self.remember_map_cell.remove(rm_index);
        }

        self.remember_map_cell.push_front(rc);
        //如果数量大于5则忘记尾端
        if size > max_size {
            let mut rand = rand::thread_rng();
            let res = rand.gen_range(0..100);
            let forget = (size - 2) * 10;
            //50%机率忘记队列前面的
            if res < forget {
                self.remember_map_cell.pop_back();
            }
        }
    }

    pub fn trigger_pair_map_cell(&mut self, rc: RememberCell) {
        let res = self
            .remember_map_cell
            .iter()
            .enumerate()
            .find(|(_, re)| re.cell_index == rc.cell_index);

        if let Some((rm_index, _)) = res {
            self.remember_map_cell.remove(rm_index);
        }
    }
}
