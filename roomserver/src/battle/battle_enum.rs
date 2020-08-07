///默认每个turn翻地图块次数
pub static TURN_DEFAULT_OPEN_CELL_TIMES: u8 = 2;

///触发范围一圈不包括中心
pub static TRIGGER_SCOPE_NEAR: [isize; 6] = [-6, -5, -1, 1, 5, 6];

///buff类型
pub mod buff_type {
    ///增加攻击力并变成AOE
    pub static ADD_ATTACK_AND_AOE: [u32; 1] = [4];
    ///增加攻击力
    pub static ADD_ATTACK: [u32; 2] = [4, 7];
    ///减伤buff
    pub static SUB_ATTACK_DAMAGE: [u32; 2] = [8, 10001];
    ///获得道具
    pub static AWARD_ITEM: [u32; 5] = [10003, 30011, 30021, 30031, 30041];
    ///配对恢复生命
    pub static PAIR_CURE: [u32; 1] = [30012];
    ///获得buff
    pub static AWARD_BUFF: [u32; 1] = [30022];
    ///相临技能cd增加
    pub static NEAR_ADD_CD: [u32; 1] = [30032];
    ///相临造成技能伤害
    pub static NEAR_SKILL_DAMAGE: [u32; 1] = [30042];
    ///相临造成技能伤害
    /// 配对属性一样的地图块+hp
    pub static WORLD_CELL_PAIR_ADD_HP: [u32; 1] = [9];
    /// 翻开属性一样的地图块+攻击
    pub static SAME_CELL_ELEMENT_ADD_ATTACK: [u32; 1] = [1001];
    /// 翻开地图块干点啥，配对又干点啥
    pub static OPEN_CELL_AND_PAIR: [u32; 1] = [1004];
}

///pos操作类型
#[derive(Clone, Debug, PartialEq)]
pub enum PosType {
    ChangePos = 1, //切换架势
    CancelPos = 2, //取消架势
}

///效果类型
#[derive(Clone, Debug, PartialEq)]
pub enum EffectType {
    ///技能伤害
    SkillDamage = 1,
    ///攻击伤害
    AttackDamage = 2,
    ///治疗血量
    Cure = 3,
    ///减攻击伤害
    SubDamage = 4,
    ///技能减少cd
    SubSkillCd = 5,
    ///获得道具
    RewardItem = 6,
    ///增加技能cd
    AddSkillCd = 7,
    ///增加能量
    AddEnergy = 8,
}

///被动触发效果类型
pub enum TriggerEffectType {
    ///触发buff
    Buff = 1,
}

//技能消耗类型
pub enum SkillConsumeType {
    Energy = 1, //能量
}

///回合行为类型
#[derive(Clone, Debug, PartialEq)]
pub enum BattleCterState {
    Alive = 0,
    Die = 1,
}

///回合行为类型
#[derive(Clone, Debug, PartialEq)]
pub enum ActionType {
    ///无效值
    None = 0,
    ///普通攻击
    Attack = 1,
    ///使用道具
    UseItem = 2,
    ///跳过turn
    Skip = 3,
    ///翻块
    Open = 4,
    ///使用技能
    Skill = 5,
    ///触发buff
    Buff = 6,
}

impl From<u32> for ActionType {
    fn from(action_type: u32) -> Self {
        match action_type {
            1 => ActionType::Attack,
            2 => ActionType::UseItem,
            3 => ActionType::Skip,
            4 => ActionType::Open,
            5 => ActionType::Skill,
            _ => ActionType::None,
        }
    }
}

///目标类型枚举
#[derive(Clone, Debug, PartialEq)]
pub enum TargetType {
    None = 0,            //无效目标
    Cell = 1,            //地图块
    AnyPlayer = 2,       //任意玩家
    PlayerSelf = 3,      //玩家自己
    AllPlayer = 4,       //所有玩家
    OtherAllPlayer = 5,  //除自己外所有玩家
    OtherAnyPlayer = 6,  //除自己外任意玩家
    UnOpenCell = 7,      //未翻开的地图块
    UnPairCell = 8,      //未配对的地图块
    NullCell = 9,        //空的地图块，上面没人
    UnPairNullCell = 10, //未配对的地图块
    CellPlayer = 11,     //地图块上的玩家
}

impl From<u32> for TargetType {
    fn from(value: u32) -> Self {
        match value {
            1 => TargetType::Cell,
            2 => TargetType::AnyPlayer,
            3 => TargetType::PlayerSelf,
            4 => TargetType::AllPlayer,
            5 => TargetType::OtherAllPlayer,
            6 => TargetType::OtherAnyPlayer,
            7 => TargetType::UnOpenCell,
            8 => TargetType::UnPairCell,
            9 => TargetType::NullCell,
            10 => TargetType::UnPairNullCell,
            11 => TargetType::CellPlayer,
            _ => TargetType::None,
        }
    }
}

///元素类型
pub enum ElementType {
    Nature = 1, //生命元素
    Earth = 2,  //土元素
    Water = 3,  //水元素
    Fire = 4,   //火元素
}

///行动单位
#[derive(Clone, Debug, Default)]
pub struct ActionUnit {
    pub team_id: u32,
    pub user_id: u32,
    pub turn_index: u32,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, Default)]
pub struct Action {
    pub action_type: u8,
    pub action_value: u32,
}