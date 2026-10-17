//! 卡牌专属例程注册表（原版卡牌专属 Action 类的执行体；GameAction::CardRoutine 落点）。
//!
//! 原版对应：actions/unique 与各卡伴生 Action 类（WhirlwindAction、HeadbuttAction…）。
//! 例程执行时机 = 队列消费到该项（即时模型下一个 tick 内完成全部逻辑）。
//!
//! 占位约定：需要选择屏（批次 6）或随机（4.1 RNG 接线）的点用确定性占位并加
//! `// TODO` 注释；进度在 docs/modules/cards-red.md 清账。

use crate::action::{GameAction, RoutineCall, TargetRef};
use crate::battle::{BattleState, CardInst};
use crate::cards::{attack_info, damage_matrix};
use crate::power::DamageType;

type RoutineFn = fn(&mut BattleState, &RoutineCall);

/// 注册表（id → 执行体）。灌注进度：批次 3.10 前应有 ~110 项。
const ROUTINES: &[(&str, RoutineFn)] = &[
    // actions/unique/WhirlwindAction：energyOnUse 次 × 全体伤害（矩阵在执行时计算，同 tick 等价）
    ("WhirlwindAction", whirlwind),
    // actions/unique/ArmamentsAction：升级手牌（upgraded=全体；否则选择 1 张）
    ("ArmamentsAction", armaments),
    // actions/common/ExhaustAction：amount2=1 表示随机消耗
    ("ExhaustAction", exhaust_action),
    // actions/unique/ModifyDamageAction：Rampage 永久成长（misc += amount）
    ("ModifyDamageAction", modify_damage),
    // actions/unique/FiendFireAction：消耗整手 + 每张一次伤害
    ("FiendFireAction", fiend_fire),
    // actions/unique/LimitBreakAction：力量翻倍（叠加）
    ("LimitBreakAction", limit_break),
    // actions/unique/FeedAction：伤害 + 斩杀加最大生命
    ("FeedAction", feed),
    // actions/unique/InfernalBladeAction：随机卡进手（0 费）——TODO 4.1 RNG
    ("InfernalBladeAction", infernal_blade),
];

pub fn run(b: &mut BattleState, call: &RoutineCall) {
    match ROUTINES.iter().find(|(id, _)| *id == call.id) {
        Some((_, f)) => f(b, call),
        None => {
            #[cfg(debug_assertions)]
            eprintln!("[sts-core] untranslated card routine: {}", call.id);
        }
    }
}

/// WhirlwindAction：消耗 energyOnUse 点能量（amount2），每点对全体敌人打一次 baseDamage。
fn whirlwind(b: &mut BattleState, call: &RoutineCall) {
    let base = sts_data::gen::CARDS[call.sid].base_damage;
    for _ in 0..call.amount.max(0) {
        let matrix = damage_matrix(b, base, DamageType::Normal);
        b.mgr
            .add_bottom(GameAction::DamageAll { matrix, ty: DamageType::Normal });
    }
    // X 费的能量扣减由动作本体承担（useCard 对 cost=-1 不扣）
    b.energy = (b.energy - call.amount2).max(0);
}

/// ArmamentsAction：升级手牌。upgraded=全体；否则选择 1 张（选择屏 TODO 批次 6，占位取首张可升级）。
fn armaments(b: &mut BattleState, call: &RoutineCall) {
    upgrade_hand(b, call.upgraded);
}

/// 升级手牌。
pub fn upgrade_hand(b: &mut BattleState, all: bool) {
    if all {
        for c in &mut b.hand.group {
            if c.can_upgrade() {
                c.upgraded = true;
                c.times_upgraded += 1;
                c.cost_for_turn = c.eff_cost();
            }
        }
    } else {
        // TODO 批次 6：选择屏（玩家选 1 张）
        if let Some(c) = b.hand.group.iter_mut().find(|c| c.can_upgrade()) {
            c.upgraded = true;
            c.times_upgraded += 1;
            c.cost_for_turn = c.eff_cost();
        }
    }
}

/// ExhaustAction：消耗手牌。amount2=1 → 随机（cardRandomRng TODO 4.1，取顶占位）；否则选择屏 TODO 批次 6。
fn exhaust_action(b: &mut BattleState, call: &RoutineCall) {
    let n = call.amount.min(b.hand.group.len() as i32);
    for _ in 0..n {
        let idx = if call.amount2 == 1 {
            b.hand.group.len().saturating_sub(1) // TODO 4.1：随机位
        } else {
            0 // TODO 批次 6：选择屏
        };
        if idx < b.hand.group.len() {
            let c = b.hand.group.remove(idx);
            exhaust_one(b, c);
        }
    }
}

/// 消耗一张手牌（进消耗堆 + Blood for Blood 费用联动）。
pub fn exhaust_one(b: &mut BattleState, c: CardInst) {
    b.exhaust_pile.add_top(c);
    blood_for_blood_trigger(b);
}

/// Blood for Blood 机制：每次消耗后，手牌/抽牌堆/弃牌堆中的 B4B 费用 -1。
/// TODO 3.10：核对原版挂接点（ExhaustAction/ExhaustSpecificCardAction 内联）。
pub fn blood_for_blood_trigger(b: &mut BattleState) {
    for pile in [&mut b.hand, &mut b.draw_pile, &mut b.discard_pile] {
        for c in &mut pile.group {
            if c.st().id == "Blood for Blood" && c.cost_for_turn > 0 {
                c.cost_for_turn -= 1;
            }
        }
    }
}

/// FiendFireAction：count = 打出时手牌数；全部消耗后每张一次 baseDamage。
fn fiend_fire(b: &mut BattleState, call: &RoutineCall) {
    let count = b.hand.group.len();
    // 消耗整手（每张触发消耗钩子）
    while let Some(c) = b.hand.group.pop() {
        exhaust_one(b, c);
    }
    for _ in 0..count {
        if let Some(mi) = call.target {
            b.mgr.add_bottom(GameAction::Damage {
                target: TargetRef::Monster(mi),
                info: attack_info(b, mi, sts_data::gen::CARDS[call.sid].base_damage, DamageType::Normal),
            });
        }
    }
}

/// LimitBreakAction：有力量则 ApplyPower(Strength, 当前量)——叠加即翻倍。
fn limit_break(b: &mut BattleState, call: &RoutineCall) {
    let _ = call;
    let str_amt = b
        .player
        .powers
        .iter()
        .find(|p| p.kind == crate::power::PowerKind::Strength)
        .map(|p| p.amount)
        .unwrap_or(0);
    if str_amt > 0 {
        b.mgr.add_bottom(GameAction::ApplyPower {
            target: TargetRef::Player,
            source: Some(TargetRef::Player),
            power: crate::power::Power::new(crate::power::PowerKind::Strength, str_amt),
            from_monster: false,
        });
    }
}

/// FeedAction：伤害；目标因此死亡 → increaseMaxHp(magic)。
fn feed(b: &mut BattleState, call: &RoutineCall) {
    if let Some(mi) = call.target {
        let info = attack_info(b, mi, sts_data::gen::CARDS[call.sid].base_damage, DamageType::Normal);
        b.exec_damage(TargetRef::Monster(mi), info);
        if b.monsters[mi].creature.is_dying {
            b.player.increase_max_hp(call.amount);
        }
    }
}

/// InfernalBladeAction：随机攻击/技能/能力卡进手（0 费）——TODO 4.1 RNG（占位不动）。
fn infernal_blade(_b: &mut BattleState, _call: &RoutineCall) {
    // TODO 4.1：cardRng 从职业色+无色池抽卡
}

/// ModifyDamageAction：按 uuid 找到卡实例，misc += amount（Rampage 永久成长）。
fn modify_damage(b: &mut BattleState, call: &RoutineCall) {
    for pile in [&mut b.discard_pile, &mut b.hand, &mut b.draw_pile] {
        if let Some(c) = pile.group.iter_mut().find(|c| c.uuid == call.card_uuid) {
            c.misc += call.amount;
            return;
        }
    }
}
