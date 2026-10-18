//! 动作系统（`actions/AbstractGameAction` + `GameActionManager`，记录 01 §2-3、09 §0）。
//!
//! 决策 D-05：动作时长即时完成（文字 UI 无动画），但保留每个动作作为
//! **队列屏障**的语义——顺序、入队位置（top/bottom/turnStart/nextCombat）、
//! 构造器副作用时序与原版一致。
//!
//! 原版四条队列按固定优先级消费（getNextAction，记录 01 §3）：
//! 1. `actions`  2. `pre_turn`  3. `card_queue`  4. `monster_queue`(+flags)  5. 回合推进

use crate::power::{ApplyCtx, Power, PowerKind};

/// 目标引用（原版 AbstractCreature 双向引用 → 索引定位，docs/03 §4.3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetRef {
    Player,
    Monster(usize),
}

/// 动作种类。字段对齐原版动作类；`duration` 不建模（D-05）。
/// TODO 批次 3.10：actions/ 全包 ~280 类逐个补全。
#[derive(Clone, Debug)]
pub enum GameAction {
    /// `actions/common/DamageAction`：两段式（动画→结算）在即时模型下合并为一 tick；
    /// THORNS 豁免取消判定（记录 09 §7）。
    Damage {
        target: TargetRef,
        info: crate::damage::DamageInfo,
    },
    /// `LoseHPAction`：HP_LOSS 直扣（毒/自伤等）。
    LoseHp {
        target: TargetRef,
        source: Option<TargetRef>,
        amount: i32,
    },
    GainBlock {
        target: TargetRef,
        amount: i32,
    },
    Heal {
        target: TargetRef,
        amount: i32,
    },
    /// `ApplyPowerAction`：构造器副作用（Corruption 卡费/SnakeSkull——TODO 2.4/3.6）+ 首帧逻辑。
    ApplyPower {
        target: TargetRef,
        source: Option<TargetRef>,
        power: Power,
        from_monster: bool,
    },
    /// `RemoveSpecificPowerAction`。
    RemovePower {
        target: TargetRef,
        kind: PowerKind,
    },
    /// `ReducePowerAction`。
    ReducePower {
        target: TargetRef,
        kind: PowerKind,
        amount: i32,
    },
    /// `DrawCardAction`（endTurnDraw=true 时触发回合充能——原版寄生在 PlayerTurnEffect，记录 01 风险3）。
    DrawCard {
        amount: i32,
        end_turn_draw: bool,
    },
    /// `EmptyDeckShuffleAction`：构造器触发遗物 onShuffle（TODO 3.6）；弃牌堆洗回抽牌堆。
    EmptyDeckShuffle,
    /// `GainEnergyAction`。
    GainEnergy {
        amount: i32,
    },
    /// 战斗开始给初始能量 + 启用控制（`GainEnergyAndEnableControlsAction`，含 onEnergyRecharge 钩子位）。
    GainEnergyAndEnableControls {
        amount: i32,
    },
    /// `WaitAction`：即时模型下为空屏障（保序）。
    Wait,
    /// `EnableEndTurnButtonAction`：逻辑无副作用（UI 层事件）。
    EnableEndTurnButton,
    /// `EndTurnAction`：turnHasEnded = true。
    EndTurn,
    /// `MonsterStartTurnAction`：全体怪 applyPreTurnLogic（无 Barricade 掉格挡 + atStartOfTurnPowers）。
    MonsterStartTurn,
    /// `RollMoveAction`：怪 rollMove（aiRng；脚本怪为确定性）。
    RollMove {
        monster: usize,
    },
    /// `ClearCardQueueAction`。
    ClearCardQueue,
    /// `DiscardAtEndOfTurnAction`（回合末弃牌；retain/ethereal 钩子 TODO 2.4）。
    DiscardAtEndOfTurn,
    /// 战斗结束标记（由 endBattle 流程入队；即时模型下只翻转状态）。
    BattleOver,
    // ---------- 批次 3 卡牌灌注：高频动作 ----------
    /// `DamageAllEnemiesAction`：预计算矩阵（索引, 伤害），执行时跳过已死目标（记录 09 §7）。
    DamageAll {
        matrix: Vec<(usize, i32)>,
        ty: crate::power::DamageType,
    },
    /// `DiscardAction`：amount<0 表示任意张选择屏；is_random 用 cardRandomRng。
    Discard {
        amount: i32,
        is_random: bool,
        any_number: bool,
    },
    /// `DrawCardAction` 变体已在（DrawCard）。
    /// `GainEnergyAction` 已在（GainEnergy）。
    /// `MakeTempCardInHandAction`：生成临时卡进手牌。
    MakeTempCardInHand {
        sid: usize,
        upgraded: bool,
        count: i32,
    },
    /// `MakeTempCardInDrawPileAction`：random_spot=随机位置（cardRandomRng），否则顶。
    MakeTempCardInDrawPile {
        sid: usize,
        upgraded: bool,
        count: i32,
        random_spot: bool,
    },
    /// `MakeTempCardInDiscardAction`（Anger 等）。
    MakeTempCardInDiscard {
        sid: usize,
        upgraded: bool,
        count: i32,
    },
    /// `ChannelAction`：引动充能球。
    ChannelOrb {
        kind: crate::orb::OrbKind,
    },
    /// `EvokeOrbAction`：引动 N 个球（最老优先）。
    EvokeOrb {
        count: i32,
    },
    /// `EvokeOrbAction` 变体：引动但不移除（Dualcast 半段——双倍效果不消耗球）。
    EvokeWithoutRemove {
        count: i32,
    },
    /// `IncreaseMaxOrbAction`：球槽 +amount（可为负：Consume）。
    IncreaseOrbSlots {
        amount: i32,
    },
    /// `ChangeStanceAction`。
    ChangeStance {
        stance: crate::stance::StanceKind,
    },
    /// `ScryAction`：观/弃（选择屏，批次 6 交互；核心先实现"进入选择"标记）。
    Scry {
        amount: i32,
    },
    /// 卡牌专属例程（对应原版 actions/unique 与卡牌专属 Action 类，共 ~110 种）。
    /// 例：WhirlwindAction / FeedAction / ExhumeAction。执行入口 `cards::routines::run`。
    CardRoutine {
        call: RoutineCall,
    },
    /// 批次 3.10 兜底：未转译动作（审计用描述）。
    Future {
        desc: &'static str,
    },
    /// PenNib：本次攻击出牌结束 → 熄灭 ×2 标记（effects 内已消费）。
    PenNibConsume,
}

/// 卡牌专属例程调用参数（对齐原版各卡牌 Action 类的构造参数子集）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoutineCall {
    /// 例程名（= 原版 Action 类名，硬契约便于对照）。
    pub id: &'static str,
    pub target: Option<usize>,
    pub amount: i32,
    pub amount2: i32,
    /// 关联卡 uuid（原版动作持卡引用/uuid 反查）。
    pub card_uuid: u64,
    /// 关联卡 sid（生成临时卡等）。
    pub sid: usize,
    pub upgraded: bool,
}

impl GameAction {
    /// ActionType 等价（战斗结束清理白名单用，记录 01 §2：保留 HEAL/GainBlock/UseCard/DAMAGE）。
    pub fn survives_post_combat_clear(&self) -> bool {
        matches!(
            self,
            GameAction::Heal { .. }
                | GameAction::GainBlock { .. }
                | GameAction::Damage { .. }
                | GameAction::LoseHp { .. } // DAMAGE 类
        )
    }
}

/// 打卡队列项（`cards/CardQueueItem`；card 为 None = 回合末触发占位，记录 01 §3）。
#[derive(Clone, Debug)]
pub struct CardQueueItem {
    /// 卡在 hand 组中的 uuid（None = callEndOfTurnActions 占位）。
    pub card: Option<u64>,
    pub target: Option<usize>,
    pub energy_on_use: i32,
    pub autoplay_card: bool,
    pub random_target: bool,
    pub is_end_turn_auto_play: bool,
    /// 打出时的卡实例快照参数（cost_for_turn 等，由消费方使用）。
    pub free_to_play: bool,
}

impl CardQueueItem {
    pub fn end_turn_placeholder() -> Self {
        Self {
            card: None,
            target: None,
            energy_on_use: 0,
            autoplay_card: false,
            random_target: false,
            is_end_turn_auto_play: false,
            free_to_play: false,
        }
    }
}

/// 调度器（GameActionManager 的队列与标志；执行逻辑在 `battle.rs`）。
#[derive(Debug)]
pub struct ActionManager {
    pub actions: std::collections::VecDeque<GameAction>,
    pub pre_turn: std::collections::VecDeque<GameAction>,
    pub next_combat: Vec<GameAction>,
    pub card_queue: Vec<CardQueueItem>,
    /// 怪物索引队列。
    pub monster_queue: std::collections::VecDeque<usize>,
    pub phase: Phase,
    pub has_control: bool,
    pub turn_has_ended: bool,
    pub using_card: bool,
    /// 原版初始为 true（branch 4 首次消费时置 true 并排队怪物——记录 01 §3-4 语义：置回 true）。
    pub monster_attacks_queued: bool,
    pub skip_monster_turn: bool,
    pub turn: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    WaitingOnUser,
    ExecutingActions,
}

impl Default for ActionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionManager {
    pub fn new() -> Self {
        Self {
            actions: Default::default(),
            pre_turn: Default::default(),
            next_combat: Vec::new(),
            card_queue: Vec::new(),
            monster_queue: Default::default(),
            phase: Phase::WaitingOnUser,
            has_control: true,
            turn_has_ended: false,
            using_card: false,
            monster_attacks_queued: true,
            skip_monster_turn: false,
            turn: 1, // clear() 置 1（记录 01 §3）
        }
    }

    /// `addToBottom`：队尾；**非 COMBAT 静默丢弃**语义在 BattleState 层实现。
    pub fn add_bottom(&mut self, a: GameAction) {
        self.actions.push_back(a);
    }

    /// `addToTop`：队头插队。
    pub fn add_top(&mut self, a: GameAction) {
        self.actions.push_front(a);
    }

    /// `addToTurnStart`：pre_turn 队头。
    pub fn add_turn_start(&mut self, a: GameAction) {
        self.pre_turn.push_front(a);
    }

    /// `addToNextCombat`：跨战斗延迟。
    pub fn add_next_combat(&mut self, a: GameAction) {
        self.next_combat.push(a);
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// `clearPostCombatActions`（记录 01 §3）：保留 Heal/GainBlock/Damage/UseCard 类。
    pub fn clear_post_combat(&mut self) {
        self.actions.retain(|a| a.survives_post_combat_clear());
    }

    /// 全清（战斗重置；turn=1）。
    pub fn clear(&mut self) {
        self.actions.clear();
        self.pre_turn.clear();
        self.card_queue.clear();
        self.monster_queue.clear();
        self.turn_has_ended = false;
        self.using_card = false;
        self.monster_attacks_queued = true;
        self.skip_monster_turn = false;
        self.phase = Phase::WaitingOnUser;
        self.turn = 1;
    }

    /// `apply_power` 便捷：施加 + 延迟移除入队（top，对应原版 addToTop(Remove)）。
    pub fn apply_and_enqueue(
        &mut self,
        powers: &mut Vec<Power>,
        to_apply: Power,
        ctx: ApplyCtx,
        target: TargetRef,
    ) -> super::power::ApplyOutcome {
        let (outcome, pending) = super::power::ops::apply_power(powers, to_apply, ctx);
        for kind in pending {
            self.add_top(GameAction::RemovePower { target, kind });
        }
        outcome
    }
}
