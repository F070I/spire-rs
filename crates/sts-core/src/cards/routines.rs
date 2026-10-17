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
use crate::damage::DamageInfo;
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
    // actions/unique/VampireDamageAllEnemiesAction：全体伤害 + 实扣血吸血
    ("VampireDamageAllEnemiesAction", vampire_all),
    // actions/unique/ExhumeAction：消耗堆取 1 回手（选择屏 TODO 批次 6，占位取最新）
    ("ExhumeAction", exhume),
    // actions/unique/SecondWindAction（BlockPerNonAttackAction）：消耗非攻击，每张 block
    ("SecondWindAction", second_wind),
    // actions/unique/ExhaustAllNonAttackAction
    ("ExhaustAllNonAttackAction", exhaust_all_non_attack),
    // actions/unique/AttackDamageRandomEnemyAction：随机敌人 amount 伤
    ("AttackDamageRandomEnemyAction", attack_random),
    // actions/unique/EscapePlanAction：抽到技能 → +amount 格挡
    ("EscapePlanAction", escape_plan),
    // actions/unique/ExpertiseAction：抽牌直到手牌 = amount
    ("ExpertiseAction", expertise),
    // actions/unique/CalculatedGambleAction：弃整手 → 抽同数
    ("CalculatedGambleAction", calculated_gamble),
    // actions/unique/SkewerAction：X 费——每点能量一段 amount 伤
    ("SkewerAction", skewer),
    // actions/unique/MalaiseAction：X 费——敌人 Weak +amount 与力量 -amount
    ("MalaiseAction", malaise),
    // actions/unique/HeelHookAction：伤害 + 本回合有弃牌 → 抽 1 回 1 能
    ("HeelHookAction", heel_hook),
    // actions/unique/FinisherAction：本回合每张攻击 → 一次 amount 伤
    ("FinisherAction", finisher),
    // actions/unique/FlechetteAction：手牌技能数 × amount 伤
    ("FlechetteAction", flechettes),
    // actions/unique/ApplyBulletTimeAction：手牌全部 0 费
    ("ApplyBulletTimeAction", bullet_time),
    // actions/unique/BladeFuryAction（Storm of Steel 复用）：弃整手 → 每张 +1 飞刀
    ("BladeFuryAction", blade_fury),
    // actions/unique/SetupAction：手牌选 1 置抽牌堆顶（选择屏 TODO 批次 6）
    ("SetupAction", setup),
    // actions/unique/CatalystAction：毒层数 ×amount（2/3）
    ("CatalystAction", catalyst),
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
    let id = c.st().id;
    b.exhaust_pile.add_top(c);
    b.after_card_exhausted(id, 1);
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

/// SecondWindAction：消耗全部非攻击牌，每张获得 block 格挡。
fn second_wind(b: &mut BattleState, call: &RoutineCall) {
    let n = b
        .hand
        .group
        .iter()
        .filter(|c| c.st().kind != sts_data::gen::CardType::Attack)
        .count();
    let mut i = 0;
    while i < b.hand.group.len() {
        if b.hand.group[i].st().kind != sts_data::gen::CardType::Attack {
            let c = b.hand.group.remove(i);
            exhaust_one(b, c);
        } else {
            i += 1;
        }
    }
    if n > 0 {
        b.player.add_block(call.amount * n as i32);
    }
}

/// ExhaustAllNonAttackAction：消耗全部非攻击牌。
fn exhaust_all_non_attack(b: &mut BattleState, _call: &RoutineCall) {
    let mut i = 0;
    while i < b.hand.group.len() {
        if b.hand.group[i].st().kind != sts_data::gen::CardType::Attack {
            let c = b.hand.group.remove(i);
            exhaust_one(b, c);
        } else {
            i += 1;
        }
    }
}

/// AttackDamageRandomEnemyAction：随机存活敌人 amount 伤（cardRandomRng TODO 4.1 语义对齐：原版用该流）。
fn attack_random(b: &mut BattleState, call: &RoutineCall) {
    let alive: Vec<usize> = b
        .monsters
        .iter()
        .enumerate()
        .filter(|(_, m)| !m.creature.is_dying && !m.creature.is_dead)
        .map(|(i, _)| i)
        .collect();
    if alive.is_empty() {
        return;
    }
    let mi = if alive.len() == 1 {
        alive[0]
    } else {
        let idx = b.rngs.card_random.random_int_inclusive(alive.len() as i32 - 1) as usize;
        alive[idx]
    };
    b.exec_damage(
        TargetRef::Monster(mi),
        DamageInfo::pure(true, call.amount, DamageType::Normal),
    );
}

/// EscapePlanAction：抽到技能 → GainBlock(amount)。
fn escape_plan(b: &mut BattleState, call: &RoutineCall) {
    if let Some(top) = b.hand.group.last() {
        if top.st().kind == sts_data::gen::CardType::Skill {
            b.mgr.add_bottom(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: call.amount,
            });
        }
    }
}

/// ExpertiseAction：抽牌直到手牌数 = amount（不足则抽差值）。
fn expertise(b: &mut BattleState, call: &RoutineCall) {
    let deficit = call.amount - b.hand.group.len() as i32;
    if deficit > 0 {
        b.mgr.add_bottom(GameAction::DrawCard {
            amount: deficit,
            end_turn_draw: false,
        });
    }
}

/// CalculatedGambleAction：弃整手（每张计数）→ 抽同数。
fn calculated_gamble(b: &mut BattleState, call: &RoutineCall) {
    let _ = call;
    let n = b.hand.group.len() as i32;
    b.total_discarded_this_turn += n;
    while let Some(c) = b.hand.group.pop() {
        b.discard_pile.add_top(c);
    }
    if n > 0 {
        b.mgr.add_bottom(GameAction::DrawCard { amount: n, end_turn_draw: false });
    }
}

/// SkewerAction：X 费——energyOnUse 段，每段 amount 伤（amount2 = 能量）。
fn skewer(b: &mut BattleState, call: &RoutineCall) {
    for _ in 0..call.amount2.max(0) {
        if let Some(mi) = call.target {
            b.mgr.add_bottom(GameAction::Damage {
                target: TargetRef::Monster(mi),
                info: attack_info(b, mi, call.amount, DamageType::Normal),
            });
        }
    }
    b.energy = (b.energy - call.amount2).max(0);
}

/// MalaiseAction：X 费——目标 Weak +amount2？原版：施加 Weak 与力量损失各 X。amount2 = 能量。
fn malaise(b: &mut BattleState, call: &RoutineCall) {
    let x = call.amount2.max(0);
    if let Some(mi) = call.target {
        b.mgr.add_bottom(GameAction::ApplyPower {
            target: TargetRef::Monster(mi),
            source: Some(TargetRef::Player),
            power: crate::power::Power::new(crate::power::PowerKind::Weak, x),
            from_monster: false,
        });
        b.mgr.add_bottom(GameAction::ApplyPower {
            target: TargetRef::Monster(mi),
            source: Some(TargetRef::Player),
            power: crate::power::Power::new(crate::power::PowerKind::Strength, -x),
            from_monster: false,
        });
    }
    b.energy = (b.energy - x).max(0);
}

/// HeelHookAction：伤害；本回合 totalDiscardedThisTurn > 0 → 抽 1 + 回 1 能。
fn heel_hook(b: &mut BattleState, call: &RoutineCall) {
    if let Some(mi) = call.target {
        b.mgr.add_bottom(GameAction::Damage {
            target: TargetRef::Monster(mi),
            info: attack_info(b, mi, sts_data::gen::CARDS[call.sid].base_damage, DamageType::Normal),
        });
        if b.total_discarded_this_turn > 0 {
            b.mgr.add_bottom(GameAction::DrawCard { amount: 1, end_turn_draw: false });
            b.mgr.add_bottom(GameAction::GainEnergy { amount: 1 });
        }
    }
}

/// FinisherAction：本回合每张攻击 → 一次 amount 伤（多段）。
fn finisher(b: &mut BattleState, call: &RoutineCall) {
    let n = b.attacks_played_this_turn;
    for _ in 0..n {
        if let Some(mi) = call.target {
            b.mgr.add_bottom(GameAction::Damage {
                target: TargetRef::Monster(mi),
                info: attack_info(b, mi, call.amount, DamageType::Normal),
            });
        }
    }
}

/// FlechetteAction：手牌技能数 × amount 伤（一次 DamageInfo 多段动作？原版单段 count×dmg
/// ——FlechetteAction: count = 技能数，damage 输出 = info × count 单次结算）。
fn flechettes(b: &mut BattleState, call: &RoutineCall) {
    let skills = b
        .hand
        .group
        .iter()
        .filter(|c| c.st().kind == sts_data::gen::CardType::Skill)
        .count() as i32;
    if let Some(mi) = call.target {
        b.mgr.add_bottom(GameAction::Damage {
            target: TargetRef::Monster(mi),
            info: attack_info(b, mi, call.amount * skills, DamageType::Normal),
        });
    }
}

/// ApplyBulletTimeAction：手牌全部 setCostForTurn(0)（NoDraw 由卡牌 apply 段处理）。
fn bullet_time(b: &mut BattleState, _call: &RoutineCall) {
    for c in &mut b.hand.group {
        c.cost_for_turn = 0;
    }
}

/// BladeFuryAction（Storm of Steel）：弃整手 → 每张 +1 Shiv 进手。
fn blade_fury(b: &mut BattleState, call: &RoutineCall) {
    let _ = call;
    let n = b.hand.group.len() as i32;
    b.total_discarded_this_turn += n;
    while let Some(c) = b.hand.group.pop() {
        b.discard_pile.add_top(c);
    }
    for _ in 0..n {
        b.mgr.add_bottom(GameAction::MakeTempCardInHand {
            sid: crate::cards::sid_of("Shiv"),
            upgraded: call.upgraded,
            count: 1,
        });
    }
}

/// SetupAction：手牌选 1 置抽牌堆顶（选择屏 TODO 批次 6；占位移除手牌顶）。
fn setup(b: &mut BattleState, _call: &RoutineCall) {
    if let Some(c) = b.hand.group.pop() {
        b.draw_pile.add_top(c);
    }
}

/// CatalystAction：目标毒层数 ×amount（2/3）。
fn catalyst(b: &mut BattleState, call: &RoutineCall) {
    if let Some(mi) = call.target {
        let cur = b.monsters[mi]
            .creature
            .powers
            .iter()
            .find(|p| p.kind == crate::power::PowerKind::Poison)
            .map(|p| p.amount)
            .unwrap_or(0);
        if cur > 0 {
            let add = cur * (call.amount - 1);
            b.mgr.add_bottom(GameAction::ApplyPower {
                target: TargetRef::Monster(mi),
                source: Some(TargetRef::Player),
                power: crate::power::Power::new(crate::power::PowerKind::Poison, add),
                from_monster: false,
            });
        }
    }
}

/// InfernalBladeAction：随机攻击/技能/能力卡进手（0 费）——TODO 4.1 RNG（占位不动）。
fn infernal_blade(_b: &mut BattleState, _call: &RoutineCall) {
    // TODO 4.1：cardRng 从职业色+无色池抽卡
}

/// VampireDamageAllEnemiesAction（Reaper）：全体伤害；每目标实扣血（lastDamageTaken）合计回复。
fn vampire_all(b: &mut BattleState, call: &RoutineCall) {
    let base = sts_data::gen::CARDS[call.sid].base_damage;
    let matrix = damage_matrix(b, base, DamageType::Normal);
    let mut heal_total = 0;
    for (mi, dmg) in matrix {
        let alive = mi < b.monsters.len()
            && !b.monsters[mi].creature.is_dying
            && !b.monsters[mi].creature.is_dead;
        if alive {
            b.exec_damage(
                TargetRef::Monster(mi),
                DamageInfo::pure(false, dmg, DamageType::Normal),
            );
            heal_total += b.monsters[mi].creature.last_damage_taken;
        }
    }
    if heal_total > 0 {
        b.player.heal(heal_total);
    }
}

/// ExhumeAction：消耗堆取 1 回手（TODO 批次 6 选择屏；占位取最新消耗的卡）。
fn exhume(b: &mut BattleState, call: &RoutineCall) {
    let _ = call;
    if let Some(c) = b.exhaust_pile.group.pop() {
        b.hand.add_top(c);
    }
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
