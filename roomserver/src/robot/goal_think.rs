use crate::robot::goal_evaluator::attack_goal_evaluator::AttackTargetGoalEvaluator;
use crate::robot::goal_evaluator::GoalEvaluator;
use crate::room::character::BattleCharacter;
use crossbeam::atomic::AtomicCell;
use std::collections::VecDeque;
use std::ops::Deref;

#[derive(Default)]
pub struct GoalThink {
    goal_evaluators: Vec<Box<dyn GoalEvaluator>>,
}

impl Clone for GoalThink {
    fn clone(&self) -> Self {
        GoalThink::new()
    }
}

impl GoalThink {
    pub fn new() -> Self {
        let mut gt = GoalThink::default();
        let attack = Box::new(AttackTargetGoalEvaluator::default());
        gt.goal_evaluators.push(attack);
        gt
    }

    ///仲裁goal
    pub fn arbitrate(&self, cter: &mut BattleCharacter) {
        println!("开始执行仲裁");
        let mut best_desirabilty = 0;
        let mut best_index = 0;
        if self.goal_evaluators.len() == 0 {
            return;
        }
        for index in 0..self.goal_evaluators.len() {
            let ge = self.goal_evaluators.get(index).unwrap();
            let desirabilty = ge.calculate_desirability();
            if desirabilty > best_desirabilty {
                best_desirabilty = desirabilty;
                best_index = index;
            }
        }

        // let best_goal_evaluator = self.goal_evaluators.get(best_index).unwrap();
        // best_goal_evaluator.set_goal(cter);
    }
}
