use crate::battle::{battle::BattleData, battle_player::BattlePlayer};
use crate::robot::goal_evaluator::GoalEvaluator;
use crate::robot::robot_status::open_cell_action::OpenCellRobotAction;
use crate::robot::robot_task_mgr::RobotTask;
use crossbeam::channel::Sender;

#[derive(Default)]
pub struct OpenCellGoalEvaluator {
    // desirability: AtomicCell<u32>,
}

impl GoalEvaluator for OpenCellGoalEvaluator {
    fn calculate_desirability(&self, robot: &BattlePlayer) -> u32 {
        if !robot.get_current_cter().map_cell_index_is_choiced() {
            return 0;
        }
        let robot_data = robot.robot_data.as_ref().unwrap();
        let pair_index = robot_data.can_pair_index();
        unsafe {
            let battle_data = robot_data.battle_data.as_ref().unwrap();
            if pair_index.is_some() && robot.flow_data.residue_movement_points > 0 {
                return 70;
            } else if battle_data.tile_map.un_pair_map.is_empty()
                || robot.flow_data.residue_movement_points == 0
            {
                return 0;
            }
            50
        }
    }

    fn set_status(
        &self,
        robot: &BattlePlayer,
        sender: Sender<RobotTask>,
        battle_data: *mut BattleData,
    ) {
        let mut res = OpenCellRobotAction::new(battle_data, sender);
        res.cter_id = robot.get_cter_temp_id();
        res.robot_id = robot.get_user_id();
        res.temp_id = robot.robot_data.as_ref().unwrap().temp_id;
        robot.change_robot_status(Box::new(res));
    }
}
