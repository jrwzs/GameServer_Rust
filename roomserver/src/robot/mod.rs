pub mod goal_evaluator;
pub mod goal_think;
pub mod robot_action;
pub mod robot_status;
pub mod robot_task_mgr;
pub mod robot_trigger;

use crate::battle::battle::BattleData;
use crate::robot::goal_think::GoalThink;
use crate::robot::robot_action::RobotStatusAction;
use crate::robot::robot_task_mgr::RobotTask;
use crate::robot::robot_trigger::RobotTriggerType;
use crate::room::character::BattleCharacter;
use crossbeam::channel::Sender;
use std::collections::VecDeque;
use tools::macros::GetMutRef;

///记忆地图块结构体
#[derive(Default, Clone)]
pub struct RememberCell {
    pub cell_index: usize, //地图块下标
    pub cell_id: u32,      //地图块id
}

impl RememberCell {
    pub fn new(cell_index: usize, cell_id: u32) -> Self {
        let mut rc = RememberCell::default();
        rc.cell_index = cell_index;
        rc.cell_id = cell_id;
        rc
    }
}

///机器人数据结构体
pub struct RobotData {
    pub robot_id: u32,
    pub battle_data: *const BattleData,
    pub goal_think: GoalThink,                            //机器人think
    pub robot_status: Option<Box<dyn RobotStatusAction>>, //状态,
    pub remember_map_cell: VecDeque<RememberCell>,        //记忆地图块
    pub sender: Sender<RobotTask>,                        //机器人任务sender
}

tools::get_mut_ref!(RobotData);

impl RobotData {
    ///创建robotdata结构体
    pub fn new(robot_id: u32, battle_data: *const BattleData, sender: Sender<RobotTask>) -> Self {
        RobotData {
            robot_id,
            battle_data,
            goal_think: GoalThink::default(),
            robot_status: None,
            remember_map_cell: VecDeque::new(),
            sender,
        }
    }

    pub fn clone_battle_data_ptr(&self) -> *const BattleData {
        self.battle_data.clone()
    }

    pub fn get_battle_cter_mut_ref(&self) -> &mut BattleCharacter {
        unsafe {
            let res = self
                .battle_data
                .as_ref()
                .unwrap()
                .battle_cter
                .get(&self.robot_id)
                .unwrap();
            res.get_mut_ref()
        }
    }

    pub fn thinking_do_something(&self) {
        let cter = self.get_battle_cter_mut_ref();
        self.goal_think
            .arbitrate(cter, self.sender.clone(), self.clone_battle_data_ptr());
    }

    pub fn trigger(&self, rc: RememberCell, trigger_type: RobotTriggerType) {
        match trigger_type {
            RobotTriggerType::SeeMapCell => {
                self.trigger_see_map_cell(rc);
            }
            RobotTriggerType::MapCellPair => {
                self.trigger_pair_map_cell(rc);
            }
            _ => {
                self.trigger_see_map_cell(rc);
            }
        }
    }
}

impl Clone for RobotData {
    fn clone(&self) -> Self {
        RobotData {
            robot_id: self.robot_id,
            battle_data: self.battle_data.clone(),
            goal_think: self.goal_think.clone(),
            robot_status: None,
            remember_map_cell: self.remember_map_cell.clone(),
            sender: self.sender.clone(),
        }
    }
}
