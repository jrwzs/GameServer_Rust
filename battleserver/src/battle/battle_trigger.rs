use crate::battle::battle::SummaryUser;
use crate::battle::battle_enum::buff_type::{
    ATTACKED_ADD_ENERGY, CAN_NOT_MOVED, CHANGE_SKILL, DEFENSE_NEAR_MOVE_SKILL_DAMAGE, LOCK_SKILLS,
    TRANSFORM_BUFF, TRAP_ADD_BUFF, TRAP_SKILL_DAMAGE,
};
use crate::battle::battle_enum::skill_judge_type::{LIMIT_ROUND_TIMES, LIMIT_TURN_TIMES};
use crate::battle::battle_enum::skill_type::{
    CHARGE_SKILL_DAMGE_ABSORPTION, FULL_MAP_DAMAGE, WATER_TURRET,
};
use crate::battle::battle_enum::EffectType::AddSkill;
use crate::battle::battle_enum::{ActionType, EffectType, TRIGGER_SCOPE_NEAR};
use crate::battle::battle_skill::Skill;
use crate::robot::robot_trigger::RobotTriggerType;
use crate::robot::RememberCell;
use crate::TEMPLATES;
use crate::{battle::battle::BattleData, room::map_data::MapCell};
use log::{error, warn};
use protobuf::Message;
use std::convert::TryFrom;
use std::str::FromStr;
use tools::cmd_code::ClientCode;
use tools::macros::GetMutRef;
use tools::protos::base::{ActionUnitPt, EffectPt, TargetPt, TriggerEffectPt};
use tools::protos::battle::S_ACTION_NOTICE;

use super::battle::DayNight;
use super::battle_cter::BattleCharacter;
use super::battle_enum::buff_type::{
    ALL_CROW_DIE_REFRESH_SKILL_CD, ATTACKED_SUB_CD, FIRE_MAP_CELL,
    RETURN_ATTACKED_DAMAGE_TO_SKILL_AOE, ROUND_CHANGE_SKILL, SUMMON_CROW,
};
use super::battle_enum::buff_type::{DIE_END_TURN, OPEN_ELEMENT_CELL_CLEAR_CD};
use super::battle_enum::skill_type::SKILL_PAIR_LIMIT_DAMAGE;
use super::battle_enum::{DamageType, FromType, TargetType};
use super::battle_helper::build_action_unit_pt;
use super::mission::{trigger_mission, MissionTriggerType};

///触发事件trait
pub trait TriggerEvent {
    ///round开始时候触发
    fn round_start_trigger(&mut self);

    ///回合开始时候触发，主要触发buff
    fn turn_start_trigger(&mut self);

    ///回合结束时候触发
    fn turn_end_trigger(&mut self);

    ///翻开地图块时候触发,主要触发buff和游戏机制上的东西
    fn open_map_cell_trigger(
        &mut self,
        cter_id: u32,
        au: &mut ActionUnitPt,
        is_pair: bool,
    ) -> anyhow::Result<Option<(u32, ActionUnitPt)>>;

    ///看到地图块触发
    fn map_cell_trigger_for_robot(&mut self, index: usize, robot_trigger_type: RobotTriggerType);

    ///被移动前触发buff
    fn before_moved_trigger(&self, from_cter_id: u32, target_cter_id: u32) -> anyhow::Result<()>;

    ///移动位置后触发事件
    fn after_move_trigger(
        &mut self,
        battle_cter: &mut BattleCharacter,
        index: isize,
        is_change_index_both: bool,
        au: &mut ActionUnitPt,
    ) -> (bool, Vec<(u32, ActionUnitPt)>);

    ///使用技能后触发
    fn after_use_skill_trigger(
        &mut self,
        cter_id: u32,
        skill_id: u32,
        is_item: bool,
        au: &mut ActionUnitPt,
    );

    fn before_use_skill_trigger(&mut self, cter_id: u32) -> anyhow::Result<()>;

    ///被攻击前触发
    fn attacked_before_trigger(&mut self, cter_id: u32, target_pt: &mut TargetPt);

    ///被攻击后触发
    fn attacked_after_trigger(&mut self, target_cter_id: u32, target_pt: &mut TargetPt);

    ///受到攻击伤害后触发
    fn attacked_hurted_trigger(
        &mut self,
        from_cter_id: u32,
        target_cter_id: u32,
        damage: u32,
        target_pt: &mut TargetPt,
    ) -> Vec<TargetPt>;

    ///地图刷新时候触发buff
    fn before_map_refresh_buff_trigger(&mut self);

    ///buff失效时候触发
    fn buff_lost_trigger(&mut self, cter_id: u32, buff_id: u32);

    ///角色死亡触发
    fn after_cter_died_trigger(&mut self, cter_id: u32) -> Option<TargetPt>;

    ///玩家死亡触发
    fn after_player_died_trigger(
        &mut self,
        from_user_id: u32,
        die_user_id: u32,
        is_last_one: bool,
        is_punishment: bool,
        str: Option<String>,
    );

    fn hurted_trigger(&mut self, target_cter_id: u32) -> Option<TargetPt>;
}

impl BattleData {
    ///触发陷阱
    pub fn trigger_trap(
        &mut self,
        battle_cter: &mut BattleCharacter,
        index: usize,
        au: &mut ActionUnitPt,
    ) -> Option<Vec<(u32, ActionUnitPt)>> {
        let map_cell = self.tile_map.map_cells.get_mut(index);
        if let None = map_cell {
            warn!("map do not has this map_cell!index:{}", index);
            return None;
        }
        let mut au_v = Vec::new();
        let turn_index = self.next_turn_index;
        let cter_id = battle_cter.base_attr.cter_id;
        let map_cell = map_cell.unwrap() as *mut MapCell;
        unsafe {
            for buff in map_cell.as_ref().unwrap().get_traps() {
                let buff_id = buff.get_id();
                let buff_function_id = buff.function_id;
                let mut target_pt = None;
                let mut aup = build_action_unit_pt(
                    buff.from_cter.unwrap(),
                    ActionType::Buff,
                    Some(buff.get_id()),
                );
                //判断是否是上buff的陷阱
                if TRAP_ADD_BUFF.contains(&buff_function_id) {
                    let buff_id = buff.buff_temp.par1;
                    let buff_temp = TEMPLATES.buff_temp_mgr().get_temp(&buff_id);
                    if let Err(e) = buff_temp {
                        warn!("{:?}", e);
                        continue;
                    }
                    battle_cter.add_buff(buff.from_cter, None, buff_id, Some(turn_index));

                    let mut target_pt_tmp = TargetPt::new();
                    target_pt_tmp
                        .target_value
                        .push(battle_cter.get_map_cell_index() as u32);
                    target_pt_tmp.add_buffs.push(buff_id);
                    target_pt = Some(target_pt_tmp);
                } else if TRAP_SKILL_DAMAGE.contains(&buff_function_id) {
                    if buff_function_id == FIRE_MAP_CELL {
                        let fire_protect_buff = battle_cter.get_fire_protect_buff();
                        match fire_protect_buff {
                            Some(fire_protect_buff) => {
                                let add_energy = fire_protect_buff.buff_temp.par2 as i8;
                                let mut target_pt = TargetPt::new();
                                target_pt
                                    .target_value
                                    .push(battle_cter.get_map_cell_index() as u32);
                                let mut ep = EffectPt::new();
                                ep.set_effect_type(EffectType::AddEnergy.into_u32());
                                ep.set_effect_value(add_energy as u32);
                                target_pt.effects.push(ep);
                                au.targets.push(target_pt);
                                battle_cter.add_energy(add_energy);
                                continue;
                            }
                            None => {}
                        }
                    }

                    //造成技能伤害的陷阱
                    let skill_damage = buff.buff_temp.par1 as i16;

                    let mut target_pt_temp = self.new_target_pt(cter_id).unwrap();
                    let res = self.deduct_hp(
                        buff.from_cter.unwrap(),
                        cter_id,
                        DamageType::Skill(skill_damage),
                        &mut target_pt_temp,
                        true,
                    );

                    match res {
                        Ok((_, other_target_pts)) => {
                            for i in other_target_pts {
                                aup.targets.push(i);
                            }
                        }
                        Err(e) => {
                            error!("{:?}", e);
                            continue;
                        }
                    }

                    target_pt = Some(target_pt_temp);
                }

                if target_pt.is_none() {
                    continue;
                }
                if buff.from_cter.is_none() {
                    continue;
                }
                let mut target_pt = target_pt.unwrap();

                let lost_buff = self.consume_buff(buff_id, None, Some(index), false);
                if let Some(lost_buff) = lost_buff {
                    target_pt.lost_buffs.push(lost_buff);
                }
                aup.targets.push(target_pt);
                au_v.push((0, aup));
            }
        }

        Some(au_v)
    }
}

impl TriggerEvent for BattleData {
    fn open_map_cell_trigger(
        &mut self,
        cter_id: u32,
        au: &mut ActionUnitPt,
        is_pair: bool,
    ) -> anyhow::Result<Option<(u32, ActionUnitPt)>> {
        let self_ptr = self as *mut BattleData;
        unsafe {
            let self_mut = self_ptr.as_mut().unwrap();
            let battle_cter = self_mut.get_battle_cter_mut(cter_id, true)?;
            let user_id = battle_cter.base_attr.user_id;
            let index = battle_cter.get_map_cell_index();
            //匹配玩家身上的buff和地图快上面的buff
            self.trigger_open_map_cell_buff(index, cter_id, au, is_pair);

            let battle_player = self.battle_player.get_mut(&user_id).unwrap();
            let user_id = battle_player.get_user_id();
            let map_cell = self.tile_map.map_cells.get(index).unwrap();
            let element = map_cell.element as u32;
            //配对了加金币
            if is_pair {
                //把配对可用的技能加入
                let mut skill_function_id;
                let mut skill_id;
                for skill in battle_cter.skills.values() {
                    skill_function_id = skill.function_id;
                    skill_id = skill.id;
                    if !SKILL_PAIR_LIMIT_DAMAGE.contains(&skill_function_id) {
                        continue;
                    }
                    battle_player.flow_data.pair_usable_skills.insert(skill_id);
                }
                //配对了奖励金币
                let res;
                let temp = crate::TEMPLATES
                    .constant_temp_mgr()
                    .temps
                    .get("reward_gold_pair_cell");
                match temp {
                    Some(temp) => {
                        let value = u32::from_str(temp.value.as_str());
                        match value {
                            Ok(value) => res = value,
                            Err(e) => {
                                error!("{:?}", e);
                                res = 2;
                            }
                        }
                    }
                    None => {
                        res = 2;
                    }
                }
                battle_player.add_gold(res as i32);
                //触发翻地图块任务;触发获得金币;触发配对任务
                trigger_mission(
                    self,
                    user_id,
                    vec![
                        (MissionTriggerType::Pair, 1),
                        (MissionTriggerType::GetGold, res as u16),
                    ],
                    (element, 0),
                );
            }

            //翻开地图块触发buff
            let mut buff_function_id;
            let mut cter_index;
            let mut buff_id;

            for &temp_cter_id in self.cter_player.keys() {
                let cter = self_mut.get_battle_cter_mut(temp_cter_id, true);
                if cter.is_err() {
                    continue;
                }
                let battle_cter = cter.unwrap();
                cter_index = battle_cter.get_map_cell_index();
                for (_, buff) in battle_cter.battle_buffs.buffs() {
                    buff_function_id = buff.function_id;
                    buff_id = buff.get_id();
                    if buff_function_id == OPEN_ELEMENT_CELL_CLEAR_CD {
                        let temp = crate::TEMPLATES.buff_temp_mgr().get_temp(&buff.get_id())?;
                        if temp.par1 == element {
                            let mut target_pt = TargetPt::new();
                            target_pt.target_value.push(cter_index as u32);
                            let mut tep = TriggerEffectPt::new();

                            let skill = battle_cter.skills.get_mut(&temp.par1).unwrap();
                            tep.set_field_type(EffectType::RefreshSkillCd.into_u32());
                            tep.buff_id = buff_id;
                            tep.set_value(skill.id);
                            skill.clean_cd();
                            target_pt.passiveEffect.push(tep);
                            au.targets.push(target_pt);
                        }
                    }
                }
            }
            Ok(None)
        }
    }

    fn map_cell_trigger_for_robot(&mut self, index: usize, robot_trigger_type: RobotTriggerType) {
        let map_cell = self.tile_map.map_cells.get(index).unwrap();
        let rc = RememberCell::new(index, map_cell.id);
        for battle_player in self.battle_player.values_mut() {
            //如果不是机器人就continue；
            if !battle_player.is_robot() {
                continue;
            }
            battle_player
                .robot_data
                .as_mut()
                .unwrap()
                .trigger(rc.clone(), robot_trigger_type);
        }
    }

    fn before_moved_trigger(&self, from_cter_id: u32, target_cter_id: u32) -> anyhow::Result<()> {
        //先判断目标位置的角色是否有不动泰山被动技能
        let target_cter = self.get_battle_cter(target_cter_id, true);
        if let Err(_) = target_cter {
            return Ok(());
        }
        let target_cter = target_cter.unwrap();
        let mut buff_function_id;
        for buff in target_cter.battle_buffs.buffs().values() {
            buff_function_id = buff.function_id;
            if buff_function_id == CAN_NOT_MOVED && from_cter_id != target_cter_id {
                anyhow::bail!(
                    "this cter can not be move!cter_id:{},buff_id:{}",
                    target_cter_id,
                    CAN_NOT_MOVED
                )
            }
        }

        Ok(())
    }

    ///移动后触发事件，大多数为buff
    fn after_move_trigger(
        &mut self,
        battle_cter: &mut BattleCharacter,
        index: isize,
        is_change_index_both: bool,
        au: &mut ActionUnitPt,
    ) -> (bool, Vec<(u32, ActionUnitPt)>) {
        let mut v = Vec::new();
        let self_mut = self.get_mut_ref();
        //触发陷阱
        let res = self_mut.trigger_trap(battle_cter, index as usize, au);
        if let Some(res) = res {
            v.extend_from_slice(res.as_slice());
        }
        let mut is_died = false;
        let mut buff_function_id;
        let mut buff_id;
        let target_cter = battle_cter.base_attr.cter_id;
        let mut from_user;
        let mut other_cter_id;
        //触发别人的范围
        for other_player in self.battle_player.values() {
            if other_player.is_died() {
                continue;
            }
            from_user = other_player.user_id;
            for other_cter in other_player.cters.values() {
                other_cter_id = other_cter.get_cter_id();

                let cter_index = other_cter.get_map_cell_index() as isize;
                //踩到别人到范围
                for buff in other_cter.battle_buffs.buffs().values() {
                    buff_function_id = buff.function_id;
                    buff_id = buff.get_id();
                    if !DEFENSE_NEAR_MOVE_SKILL_DAMAGE.contains(&buff_function_id) {
                        continue;
                    }
                    //换位置不触发"DEFENSE_NEAR_MOVE_SKILL_DAMAGE"
                    if is_change_index_both {
                        continue;
                    }
                    for scope_index in TRIGGER_SCOPE_NEAR.iter() {
                        let res = cter_index + scope_index;
                        if index != res {
                            continue;
                        }
                        let mut other_aupt =
                            build_action_unit_pt(other_cter_id, ActionType::Buff, Some(buff_id));
                        unsafe {
                            let mut target_pt = self.new_target_pt(target_cter).unwrap();
                            let res = self_mut.deduct_hp(
                                from_user,
                                target_cter,
                                DamageType::Skill(buff.buff_temp.par1 as i16),
                                &mut target_pt,
                                true,
                            );

                            match res {
                                Ok((_, other_target_pts)) => {
                                    for i in other_target_pts {
                                        other_aupt.targets.push(i);
                                    }
                                }
                                Err(e) => {
                                    error!("{:?}", e);
                                }
                            }

                            other_aupt.targets.push(target_pt);
                            v.push((0, other_aupt));
                            break;
                        }
                    }
                    if battle_cter.is_died() {
                        is_died = true;
                        break;
                    }
                }
            }

            //别人进入自己的范围触发
            //现在没有种buff，先注释代码
            // if battle_player.get_user_id() == other_get_current_cter_mut().get_user_id() {
            //     continue;
            // }
            // for buff in battle_player.buffs.values_mut() {
            //     if !DEFENSE_NEAR_MOVE_SKILL_DAMAGE.contains(&buff.id) {
            //         continue;
            //     }
            //     let mut need_rank = true;
            //     for scope_index in TRIGGER_SCOPE_NEAR.iter() {
            //         let res = index + scope_index;
            //         if cter_index != res {
            //             continue;
            //         }
            //         let target_pt = self.deduct_hp(
            //             battle_player.get_user_id(),
            //             other_get_current_cter_mut().get_user_id(),
            //             Some(buff.buff_temp.par1 as i32),
            //             need_rank,
            //         );
            //         match target_pt {
            //             Ok(target_pt) => {
            //                 let mut self_aupt = ActionUnitPt::new();
            //                 self_aupt.from_user = user_id;
            //                 self_aupt.action_type = ActionType::Buff as u32;
            //                 self_aupt.action_value.push(buff.id);
            //                 self_aupt.targets.push(target_pt);
            //                 v.push(self_aupt);
            //                 break;
            //             }
            //             Err(e) => error!("{:?}", e),
            //         }
            //     }
            //     need_rank = false;
            // }
        }
        (is_died, v)
    }

    ///使用技能后触发
    fn after_use_skill_trigger(
        &mut self,
        cter_id: u32,
        skill_id: u32,
        is_item: bool,
        au: &mut ActionUnitPt,
    ) {
        let self_ptr = self as *mut BattleData;
        let battle_cter = self.get_battle_cter_mut(cter_id, true);
        if let Err(e) = battle_cter {
            error!("{:?}", e);
            return;
        }
        let battle_cter = battle_cter.unwrap();
        let user_id = battle_cter.get_user_id();
        unsafe {
            let self_mut = self_ptr.as_mut().unwrap();
            let battle_player = self_mut.get_battle_player_mut(None, true).unwrap();
            //添加技能限制条件
            let skill_temp = TEMPLATES.skill_temp_mgr().get_temp(&skill_id).unwrap();
            if skill_temp.skill_judge == LIMIT_TURN_TIMES as u16 {
                battle_player.flow_data.turn_limit_skills.push(skill_id);
            } else if skill_temp.skill_judge == LIMIT_ROUND_TIMES as u16 {
                battle_player.flow_data.round_limit_skills.push(skill_id);
            }

            //战斗角色身上的技能
            let mut skill_s;
            let mut skill = None;
            if is_item {
                let res = TEMPLATES.skill_temp_mgr().get_temp(&skill_id);
                if let Err(e) = res {
                    error!("{:?}", e);
                    return;
                }
                let skill_temp = res.unwrap();
                skill_s = Skill::from_skill_temp(skill_temp, true);
                skill = Some(&mut skill_s);
            } else {
                let res = battle_cter.skills.get_mut(&skill_id);
                if let Some(res) = res {
                    skill = Some(res);
                }
            }
            if let Some(skill) = skill {
                skill.turn_use_times += 1;
                let skill_function_id = skill.function_id;

                //替换技能,水炮
                match skill_function_id {
                    WATER_TURRET | CHARGE_SKILL_DAMGE_ABSORPTION | FULL_MAP_DAMAGE => {
                        let skill_temp;
                        if skill_function_id == FULL_MAP_DAMAGE {
                            skill_temp =
                                TEMPLATES.skill_temp_mgr().get_temp(&skill.skill_temp.par3);
                        } else {
                            skill_temp =
                                TEMPLATES.skill_temp_mgr().get_temp(&skill.skill_temp.par2);
                        }
                        battle_cter.skills.remove(&skill_id);
                        if let Err(e) = skill_temp {
                            error!("{:?}", e);
                            return;
                        }
                        let st = skill_temp.unwrap();

                        let mut target_pt = TargetPt::new();
                        //封装角色位置
                        target_pt
                            .target_value
                            .push(battle_cter.get_map_cell_index() as u32);
                        //封装丢失技能
                        target_pt.lost_skills.push(skill_id);
                        //封装增加的技能
                        let mut ep = EffectPt::new();
                        ep.effect_type = AddSkill.into_u32();
                        ep.effect_value = st.id;
                        target_pt.effects.push(ep);
                        //将新技能封装到内存
                        let skill = Skill::from_skill_temp(st, false);
                        battle_cter.skills.insert(skill.id, skill);
                        //将target封装到proto
                        au.targets.push(target_pt);
                    }
                    _ => {}
                }

                //使用后删除可用状态
                if SKILL_PAIR_LIMIT_DAMAGE.contains(&skill_function_id) {
                    battle_player.flow_data.pair_usable_skills.remove(&skill_id);
                }
            }
        }

        //使用技能任务
        trigger_mission(
            self,
            user_id,
            vec![(MissionTriggerType::UseSkill, 1)],
            (0, 0),
        );
    }

    fn before_use_skill_trigger(&mut self, cter_id: u32) -> anyhow::Result<()> {
        let battle_cter = self.get_battle_cter_mut(cter_id, true);
        if let Err(e) = battle_cter {
            anyhow::bail!("{:?}", e)
        }
        let battle_cter = battle_cter.unwrap();
        let mut buff_function_id;
        for buff in battle_cter.battle_buffs.buffs().values() {
            buff_function_id = buff.function_id;
            if LOCK_SKILLS.contains(&buff_function_id) {
                anyhow::bail!(
                    "this cter can not use skill!was locked!cter's buff:{}",
                    buff.get_id()
                )
            }
        }
        Ok(())
    }

    ///被攻击前触发
    fn attacked_before_trigger(&mut self, _: u32, _: &mut TargetPt) {}

    ///被攻击后触发
    fn attacked_after_trigger(&mut self, target_cter_id: u32, target_pt: &mut TargetPt) {
        let target_battle_cter = self.get_battle_cter_mut(target_cter_id, true);
        if let Err(_) = target_battle_cter {
            return;
        }
        let target_battle_cter = target_battle_cter.unwrap();
        let max_energy = target_battle_cter.base_attr.max_energy;
        let mut buff_function_id;
        let buff_ids: Vec<u32> = target_battle_cter
            .battle_buffs
            .buffs()
            .keys()
            .copied()
            .collect();
        for buff_id in buff_ids {
            let buff = target_battle_cter
                .battle_buffs
                .buffs()
                .get(&buff_id)
                .unwrap();
            let par1 = buff.buff_temp.par1;
            buff_function_id = buff.function_id;

            //被攻击增加能量
            if ATTACKED_ADD_ENERGY.contains(&buff_function_id) && max_energy > 0 {
                let mut tep = TriggerEffectPt::new();
                tep.set_field_type(EffectType::AddEnergy.into_u32());
                tep.set_buff_id(buff_id);
                tep.set_value(par1);
                target_battle_cter.add_energy(par1 as i8);
                target_pt.passiveEffect.push(tep);
            }

            //被攻击减技能cd
            if ATTACKED_SUB_CD == buff_function_id {
                let mut tep = TriggerEffectPt::new();
                tep.set_field_type(EffectType::SubSkillCd.into_u32());
                tep.set_buff_id(buff_id);
                tep.set_value(par1);
                target_battle_cter.sub_skill_cd(Some(par1 as i8));
                target_pt.passiveEffect.push(tep);
            }
        }
    }

    ///受到普通攻击触发的buff
    fn attacked_hurted_trigger(
        &mut self,
        from_cter_id: u32,
        target_cter_id: u32,
        damage: u32,
        target_pt: &mut TargetPt,
    ) -> Vec<TargetPt> {
        let mut v = vec![];
        let battle_data_ptr = self as *mut BattleData;
        let target_battle_cter = self.get_battle_cter_mut(target_cter_id, false);
        if let Err(_) = target_battle_cter {
            return v;
        }
        let target_battle_cter = target_battle_cter.unwrap();
        let target_cter_index = target_battle_cter.get_map_cell_index();
        let mut buff_id;
        let mut buff_function_id;
        unsafe {
            for buff in target_battle_cter.battle_buffs.buffs().values() {
                buff_id = buff.get_id();
                buff_function_id = buff.function_id;

                //被攻击打断技能
                if CHANGE_SKILL == buff_function_id {
                    battle_data_ptr.as_mut().unwrap().consume_buff(
                        buff_id,
                        Some(target_cter_id),
                        None,
                        false,
                    );
                    target_pt.lost_buffs.push(buff_id);
                }
                //返伤
                if RETURN_ATTACKED_DAMAGE_TO_SKILL_AOE == buff_function_id {
                    let target_type = TargetType::try_from(buff.buff_temp.target).unwrap();
                    let res = battle_data_ptr.as_ref().unwrap().cal_scope(
                        target_cter_id,
                        target_cter_index as isize,
                        target_type,
                        None,
                        None,
                    );
                    if res.1.contains(&from_cter_id) {
                        for cter_id in res.1 {
                            let mut target_pt = battle_data_ptr
                                .as_mut()
                                .unwrap()
                                .build_target_pt(
                                    Some(target_cter_id),
                                    cter_id,
                                    EffectType::SkillDamage,
                                    damage,
                                    Some(buff_id),
                                )
                                .unwrap();
                            target_pt.effects.clear();
                            let res = battle_data_ptr.as_mut().unwrap().deduct_hp(
                                target_cter_id,
                                cter_id,
                                DamageType::Skill(damage as i16),
                                &mut target_pt,
                                true,
                            );
                            if let Ok((_, res)) = res {
                                for i in res {
                                    v.push(i);
                                }
                            }
                            v.push(target_pt);
                        }
                    }
                }
            }
        }
        v
    }

    fn before_map_refresh_buff_trigger(&mut self) {
        let self_ptr = self as *mut BattleData;
        unsafe {
            let self_mut = self_ptr.as_mut().unwrap();
            //如果存活玩家>=2并且地图未配对的数量<=2则刷新地图
            for map_cell in self.tile_map.map_cells.iter() {
                for buff in map_cell.buffs.values() {
                    let from_cter_id = buff.from_cter;
                    if from_cter_id.is_none() {
                        continue;
                    }
                    let from_cter_id = from_cter_id.unwrap();
                    let from_skill = buff.from_skill.unwrap();
                    let battle_cter = self_mut.get_battle_cter_mut(from_cter_id, true);

                    if let Err(_) = battle_cter {
                        continue;
                    }
                    let battle_cter = battle_cter.unwrap();
                    if battle_cter.is_died() {
                        continue;
                    }
                    let skill = battle_cter.skills.get_mut(&from_skill);
                    if skill.is_none() {
                        continue;
                    }
                    let skill = skill.unwrap();
                    skill.is_active = false;
                }
            }
        }
    }

    ///buff失效时候触发
    fn buff_lost_trigger(&mut self, cter_id: u32, buff_id: u32) {
        let battle_cter = self.get_battle_cter_mut(cter_id, true);
        if let Err(e) = battle_cter {
            error!("{:?}", e);
            return;
        }

        let buff_temp = TEMPLATES.buff_temp_mgr().get_temp(&buff_id).unwrap();
        let buff_function_id = buff_temp.function_id;
        let battle_cter = battle_cter.unwrap();
        //如果是变身buff,那就变回来
        if TRANSFORM_BUFF.contains(&buff_function_id) {
            battle_cter.transform_back();
        }
    }

    ///此函数只管角色死亡，不要管玩家死亡
    fn after_cter_died_trigger(&mut self, cter_id: u32) -> Option<TargetPt> {
        let mut target_pt_res = None;
        let map_cell = self.tile_map.get_map_cell_mut_by_cter_id(cter_id);
        if let Some(map_cell) = map_cell {
            map_cell.cter_id = 0;
        }
        let self_mut_ptr = self as *mut BattleData;
        let &user_id = self.cter_player.get(&cter_id).unwrap();
        let battle_cter = self.get_battle_cter(cter_id, false);
        if let Err(e) = battle_cter {
            warn!("{:?}", e);
            return target_pt_res;
        }
        let battle_cter = battle_cter.unwrap();
        //是否主角色
        let is_major = battle_cter.is_major;
        //是否是宠物
        let is_minon = battle_cter.owner.is_some();
        //是否是主人
        let is_owner = !battle_cter.minons.is_empty();

        let cter_temp_id = battle_cter.get_cter_temp_id();

        //-----------------------以下处理召唤物相关的死亡逻辑
        //如果是随从死了，就触发相关buff
        if is_minon {
            let (owner_cter_id, from_type) = battle_cter.owner.unwrap();
            unsafe {
                let self_mut = self_mut_ptr.as_mut().unwrap();
                let owner_battle_cter = self_mut.get_battle_cter_mut(owner_cter_id, true).unwrap();
                let owner_cter_index = owner_battle_cter.get_map_cell_index();
                //删除宠物id
                owner_battle_cter.minons.remove(&cter_id);
                let battle_player = self_mut_ptr
                    .as_mut()
                    .unwrap()
                    .get_battle_player_mut(Some(user_id), false)
                    .unwrap();
                battle_player.cters.remove(&cter_id);
                if owner_battle_cter.minons.is_empty() {
                    match from_type {
                        FromType::Skill(skill_id) => {
                            let skill = owner_battle_cter.skills.get_mut(&skill_id).unwrap();
                            skill.is_active = false;
                        }
                        _ => {}
                    }
                }

                for buff in owner_battle_cter.battle_buffs.buffs().values() {
                    if buff.function_id == SUMMON_CROW {
                        let crow_buff = crate::TEMPLATES
                            .buff_temp_mgr()
                            .get_temp(&buff.get_id())
                            .unwrap();
                        let crow_temp_id = crow_buff.par1;

                        //判断是否是小乌鸦
                        if cter_temp_id == crow_temp_id {
                            let count = self.get_minon_count(crow_temp_id);
                            if count == 0 {
                                let mut buff_function_id;
                                let mut buff_id;
                                for buff in owner_battle_cter.battle_buffs.buffs().values() {
                                    buff_function_id = buff.function_id;
                                    buff_id = buff.get_id();
                                    if buff_function_id == ALL_CROW_DIE_REFRESH_SKILL_CD {
                                        let day_night = self.get_day_night();
                                        let refresh_skill_id;
                                        match day_night {
                                            DayNight::Day => {
                                                refresh_skill_id = buff.buff_temp.par1;
                                            }
                                            DayNight::Night => {
                                                refresh_skill_id = buff.buff_temp.par2;
                                            }
                                        }
                                        let mut target_pt = TargetPt::new();
                                        target_pt.target_value.push(owner_cter_index as u32);
                                        let mut tep = TriggerEffectPt::new();

                                        let skill = owner_battle_cter
                                            .skills
                                            .get_mut(&refresh_skill_id)
                                            .unwrap();
                                        tep.set_field_type(EffectType::RefreshSkillCd.into_u32());
                                        tep.buff_id = buff_id;
                                        tep.set_value(skill.id);
                                        skill.clean_cd();
                                        target_pt.passiveEffect.push(tep);
                                        target_pt_res = Some(target_pt);
                                    }
                                }
                            } else {
                            }
                        }
                    }
                }

                let mut buff_function_id;
                for buff in battle_cter.battle_buffs.buffs().values() {
                    buff_function_id = buff.function_id;
                    if buff_function_id == DIE_END_TURN {
                        self_mut_ptr.as_mut().unwrap().next_turn(false);
                    }
                }
            }
        }
        //判断当前死亡角色有没有召唤物
        if is_owner && !is_major {
            let battle_player = self.get_battle_player_mut(Some(user_id), false).unwrap();
            let cter = battle_player.cters.get_mut(&cter_id).unwrap();
            //找出它的随从
            let minons = cter.minons.clone();
            cter.minons.clear();
            for minon_id in minons {
                battle_player.cters.remove(&minon_id);
            }
        }
        self.cter_player.remove(&cter_id);
        let battle_player = self.get_battle_player_mut(Some(user_id), false).unwrap();
        if battle_player.current_cter.0 == cter_id {
            battle_player.current_cter = battle_player.major_cter;
        }
        target_pt_res
    }

    ///此函数只管玩家死亡逻辑
    fn after_player_died_trigger(
        &mut self,
        from_user_id: u32,
        die_user_id: u32,
        mut is_last_one: bool,
        is_punishment: bool,
        str: Option<String>,
    ) {
        let self_ptr = self as *mut BattleData;
        unsafe {
            let self_mut = self_ptr.as_mut().unwrap();

            let die_battle_player = self
                .get_battle_player_mut(Some(die_user_id), false)
                .unwrap();
            die_battle_player.player_die(str);

            let self_league = die_battle_player.league.get_league_id();
            let player_name = die_battle_player.name.clone();
            let mut punishment_score = -50;
            let mut reward_score;
            let con_temp = crate::TEMPLATES
                .constant_temp_mgr()
                .temps
                .get("punishment_summary");
            if let Some(con_temp) = con_temp {
                let reward_score_temp = f64::from_str(con_temp.value.as_str());
                match reward_score_temp {
                    Ok(reward_score_temp) => punishment_score = reward_score_temp as i32,
                    Err(e) => warn!("{:?}", e),
                }
            }

            let mut sp = SummaryUser::default();
            sp.user_id = die_user_id;
            sp.name = player_name;
            sp.cter_temp_id = die_battle_player.get_cter_temp_id();
            sp.league = die_battle_player.league.clone();
            sp.grade = die_battle_player.grade;
            let gold = die_battle_player.gold;
            let player_count = self.get_alive_player_num() as i32;
            let rank_vec_temp = &mut self.summary_vec_temp;
            rank_vec_temp.push(sp);

            //将死掉的玩家金币都给攻击方
            if from_user_id != die_user_id {
                let from_cter = self_mut.get_battle_player_mut(Some(from_user_id), true);
                if let Ok(from_cter) = from_cter {
                    from_cter.add_gold(gold);
                }
            }
            //如果是worldboss房间，不用往下执行，到战斗结束的时候那一刻再结算
            if self.room_type.is_boss_type() {
                return;
            }
            if player_count == 1 {
                is_last_one = true;
            }
            //处理结算
            if is_last_one {
                let index = player_count as usize;
                let res = self.summary_vec.get_mut(index);
                if let None = res {
                    warn!(
                        "the rank_vec's len is {},but the index is {}",
                        self.summary_vec.len(),
                        index
                    );
                    return;
                }
                let rank_vec = res.unwrap();
                let count = rank_vec_temp.len();
                let summary_award_temp_mgr = crate::TEMPLATES.summary_award_temp_mgr();
                let con_temp_mgr = crate::TEMPLATES.constant_temp_mgr();
                let res = con_temp_mgr.temps.get("max_grade");
                let mut max_grade = 2;
                if self.room_type.is_match_type() {
                    match res {
                        None => {
                            warn!("max_grade config is None!");
                        }
                        Some(res) => {
                            max_grade = match u8::from_str(res.value.as_str()) {
                                Ok(res) => res,
                                Err(e) => {
                                    warn!("{:?}", e);
                                    max_grade
                                }
                            }
                        }
                    }
                }

                for sp in rank_vec_temp.iter_mut() {
                    sp.summary_rank = index as u8;
                    //不是匹配房间不结算段位，积分
                    if !self.room_type.is_match_type() {
                        continue;
                    }
                    //进行结算
                    if is_punishment {
                        reward_score = punishment_score;
                        sp.grade -= 1;
                        if sp.grade <= 0 {
                            sp.grade = 1;
                        }
                    } else {
                        //计算基础分
                        let mut rank = sp.summary_rank + 1;
                        if rank == 1 {
                            sp.grade += 1;
                            if sp.grade > 2 {
                                sp.grade = max_grade;
                            }
                        } else {
                            sp.grade -= 1;
                            if sp.grade <= 0 {
                                sp.grade = 1;
                            }
                        }
                        let mut base_score = 0;
                        for index in 0..count {
                            rank += index as u8;
                            let score_temp = summary_award_temp_mgr.get_score_by_rank(rank);
                            if let Err(e) = score_temp {
                                warn!("{:?}", e);
                                continue;
                            }
                            base_score += score_temp.unwrap();
                        }
                        base_score /= count as i16;
                        //计算浮动分
                        let mut average_league = 0;
                        let mut league_count = 0;
                        for (&id, league_id) in self.leave_map.iter() {
                            if id == die_user_id {
                                continue;
                            }
                            league_count += 1;
                            average_league += *league_id;
                        }
                        average_league /= league_count;
                        let mut unstable = 0;
                        if self_league >= average_league {
                            unstable = 0;
                        } else if average_league > self_league {
                            unstable = (average_league - self_league) * 10;
                        }
                        reward_score = (base_score + unstable as i16) as i32;
                    }
                    sp.reward_score = reward_score;
                    let res = sp.league.update_score(reward_score);
                    if res == 0 {
                        sp.reward_score = 0;
                    }
                }
                rank_vec.extend_from_slice(&rank_vec_temp[..]);
                rank_vec_temp.clear();
            }
        }
    }

    fn turn_start_trigger(&mut self) {
        let frist_order_user_id = self.get_frist_order_user_id();
        let user_id = self.get_turn_user(None);
        match user_id {
            Ok(user_id) => {
                if frist_order_user_id == user_id {
                    self.cycle_count += 1;
                }
            }
            Err(_) => {}
        }

        //计算玩家回合的
        self.player_turn_start_cal();

        //判断周期的
        self.cycle_cal();
    }

    fn turn_end_trigger(&mut self) {
        //清空翻开地图玩家id
        self.clear_open_cells();
        let self_ptr = self as *mut BattleData;
        let battle_player = self.get_battle_player_mut(None, true);
        if battle_player.is_err() {
            return;
        }
        let battle_player = battle_player.unwrap();
        let major_cter_id = battle_player.major_cter.0;

        unsafe {
            let self_mut = self_ptr.as_mut().unwrap();
            let mut buff;
            let mut crow_count = 0;
            let buff_temp = crate::TEMPLATES
                .buff_temp_mgr()
                .get_temp(&SUMMON_CROW)
                .unwrap();
            let count = buff_temp.par2;
            for cter in battle_player.cters.values_mut() {
                cter.skills.values_mut().for_each(|x| x.turn_use_times = 0);
                buff = cter.battle_buffs.get_buff(SUMMON_CROW);
                if buff.is_some() {
                    crow_count = cter.minons.iter().count() as u32;
                }
            }
            //如果不满足3个就召唤
            if crow_count >= 1 && crow_count < 3 {
                let mut au =
                    build_action_unit_pt(major_cter_id, ActionType::Buff, Some(SUMMON_CROW));
                let res = self_mut.summon_minon(
                    major_cter_id,
                    FromType::Buff(SUMMON_CROW),
                    Some(count - crow_count),
                    &mut au,
                );

                match res {
                    Ok(res) => {
                        let mut proto = S_ACTION_NOTICE::new();
                        proto.action_uints.push(au);
                        for (_, aupt) in res.1 {
                            proto.action_uints.push(aupt);
                        }
                        let bytes = proto.write_to_bytes().unwrap();
                        self_mut.send_2_all_client(ClientCode::ActionNotice, bytes);
                    }
                    Err(e) => warn!("{:?}", e),
                }
            }
        }

        //turn结束重制
        battle_player.turn_end_reset();
    }

    fn round_start_trigger(&mut self) {
        let mut buff_function_id;
        //增加round
        self.round += 1;
        let round = self.round;
        unsafe {
            let self_ptr = self as *mut BattleData;
            let self_mut = self_ptr.as_mut().unwrap();
            for &cter_id in self_mut.cter_player.keys() {
                let cter = self.get_battle_cter_mut(cter_id, true);
                if cter.is_err() {
                    continue;
                }
                let cter = cter.unwrap();
                for buff in cter.battle_buffs.buffs().clone().values() {
                    buff_function_id = buff.function_id;

                    //round开始变技能
                    if buff_function_id == ROUND_CHANGE_SKILL {
                        let mut skills = vec![];
                        skills.push(buff.buff_temp.par1);
                        skills.push(buff.buff_temp.par2);
                        skills.push(buff.buff_temp.par3);
                        skills.push(buff.buff_temp.par4);

                        let mut skill_id = 0;

                        for id in skills {
                            let skill_res = cter.skills.get(&id);
                            if let Some(skill_res) = skill_res {
                                skill_id = skill_res.id;
                                break;
                            }
                        }
                        let change_skill_id;
                        let skill = cter.skills.get(&skill_id).unwrap();
                        match round {
                            2 => {
                                change_skill_id = buff.buff_temp.par2;
                            }
                            3 => {
                                change_skill_id = buff.buff_temp.par3;
                            }
                            i if i >= 4 => {
                                change_skill_id = buff.buff_temp.par4;
                            }
                            _ => {
                                change_skill_id = skill.id;
                            }
                        }
                        //如果是原技能，就直接return
                        if change_skill_id == skill_id {
                            return;
                        }
                        let cd = skill.cd_times;
                        let is_active = skill.is_active;
                        cter.skills.remove(&skill_id);
                        let change_skill_temp = crate::TEMPLATES
                            .skill_temp_mgr()
                            .get_temp(&change_skill_id)
                            .unwrap();
                        let mut change_skill = Skill::from_skill_temp(change_skill_temp, true);
                        change_skill.cd_times = cd;
                        change_skill.is_active = is_active;
                        cter.skills.insert(change_skill.id, change_skill);
                    }
                }
            }
        }
    }

    fn hurted_trigger(&mut self, target_cter_id: u32) -> Option<TargetPt> {
        let target_cter = self.get_battle_cter_mut(target_cter_id, true);
        if let Err(_) = target_cter {
            return None;
        }
        let target_cter = target_cter.unwrap();
        let stone_buff = target_cter.get_stone_buff();
        if stone_buff.is_none() {
            return None;
        }
        let (target_pt, other_buffs) = target_cter.transform_back();
        let mut from_cter_id;
        let mut from_skill_id;
        for buff in other_buffs {
            from_cter_id = buff.from_cter.unwrap();
            from_skill_id = buff.from_skill.unwrap();
            let cter = self.get_battle_cter_mut(from_cter_id, true);
            match cter {
                Ok(cter) => {
                    let skill = cter.skills.get_mut(&from_skill_id);
                    match skill {
                        Some(skill) => skill.is_active = false,
                        None => {}
                    }
                }
                Err(_) => {}
            }
        }
        Some(target_pt)
    }
}
