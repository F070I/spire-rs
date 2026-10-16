//! 能力施加 / 叠层 / 递减 / 移除（ApplyPowerAction / stackPower / ReducePowerAction /
//! RemoveSpecificPowerAction 的语义核心，依据 docs/source-inventory/09 §8、03 §1E/2）。
//!
//! 时序保真：`pending_removals` 对应原版 stackPower 内 `addToTop(RemoveSpecificPowerAction)`
//! 的**延迟移除**——动作层（批次 2.3）据此入队，本层不得立即删除。

use super::{priority, Power, PowerKind};

/// 施加结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// 已施加；stacked=是否为同 ID 叠层（叠层不触发排序/onInitialApplication）
    Applied { stacked: bool },
    /// Artifact 拦截：DEBUFF 被挡，Artifact 层数已消耗（耗尽时进入 pending_removals）
    BlockedByArtifact,
    /// Ginger/Turnip 遗物拦截（无声取消；对应记录 09 §8 的 duration-=dt hack）
    BlockedByRelic,
    /// NoDrawPower 特判：目标已有同 ID → 放弃（不叠加，记录 09 §8-2a）
    NoDrawExists,
}

/// 施加上下文：来自状态树的遗物/阵营事实（批次 2.3 由 BattleState 填充）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ApplyCtx {
    pub target_is_player: bool,
    pub player_has_ginger: bool,
    pub player_has_turnip: bool,
}

/// 施加一个能力（`ApplyPowerAction.update()` 首帧逻辑的等价实现）。
///
/// `powers` 为目标的能力列表；列表顺序 = 原版遍历顺序
/// （插入序 + 每次新增时按 priority 稳定重排）。
///
/// TODO(批次 3.5)：施加者 `onApplyPower` 钩子链（Sadistic 等）、
/// Champion Belt 追加施加（relic 层接入后挂接）。
pub fn apply_power(
    powers: &mut Vec<Power>,
    to_apply: Power,
    ctx: ApplyCtx,
) -> (ApplyOutcome, Vec<PowerKind>) {
    let mut pending_removals = Vec::new();

    // Ginger（Weak 对玩家）/ Turnip（Frail 对玩家）：无声取消
    if ctx.target_is_player {
        if ctx.player_has_ginger && to_apply.kind == PowerKind::Weak {
            return (ApplyOutcome::BlockedByRelic, pending_removals);
        }
        if ctx.player_has_turnip && to_apply.kind == PowerKind::Frail {
            return (ApplyOutcome::BlockedByRelic, pending_removals);
        }
    }

    // Artifact 拦截：目标有 Artifact 且施加的是 DEBUFF → 消耗一层、能力未施加
    if super::is_debuff(to_apply.kind) {
        if let Some(art) = powers.iter_mut().find(|p| p.kind == PowerKind::Artifact) {
            // onSpecificTrigger（Artifact）：amount -= 1；耗尽 → addToTop(Remove)
            art.amount -= 1;
            if art.amount <= 0 {
                pending_removals.push(PowerKind::Artifact);
            }
            return (ApplyOutcome::BlockedByArtifact, pending_removals);
        }
    }

    // 已有同 ID（原版排除 "Night Terror" 特例——非标准实例，本层无此 kind）→ 叠层
    if powers.iter().any(|p| p.kind == to_apply.kind) {
        if to_apply.kind == PowerKind::NoDraw {
            return (ApplyOutcome::NoDrawExists, pending_removals);
        }
        if let Some(existing) = powers.iter_mut().find(|p| p.kind == to_apply.kind) {
            stack_power(existing, to_apply.amount, &mut pending_removals);
        }
        return (ApplyOutcome::Applied { stacked: true }, pending_removals);
    }

    // 新增：加入 → 按 priority 稳定排序 → onInitialApplication（TODO 3.5 钩子）
    powers.push(to_apply);
    powers.sort_by_key(|p| priority(p.kind)); // 稳定排序：同 priority 保持插入序
    (ApplyOutcome::Applied { stacked: false }, pending_removals)
}

/// `AbstractPower.stackPower`（记录 03 §1E + 各能力覆写）。
/// `pending_removals` 可能收到本能力（amount 归 0 的 Strength/Dexterity/Focus）。
pub fn stack_power(p: &mut Power, stack_amount: i32, pending_removals: &mut Vec<PowerKind>) {
    // amount == -1 哨兵：不叠加，直接 return
    if p.amount == -1 {
        return;
    }
    p.amount += stack_amount;
    match p.kind {
        // Strength/Dexterity/Focus：钳 ±999；归 0 → 延迟移除（addToTop）
        PowerKind::Strength | PowerKind::Dexterity | PowerKind::Focus => {
            p.amount = p.amount.clamp(-999, 999);
            if p.amount == 0 {
                pending_removals.push(p.kind);
            }
        }
        _ => {}
    }
}

/// `ReducePowerAction`（记录 03 §1E）：
/// 减少量 < 现有 amount → 真正递减（钳 0 不下穿）；否则 → 整体移除。
pub fn reduce_power(p: &mut Power, reduce_amount: i32) -> ReduceResult {
    if reduce_amount < p.amount {
        p.amount -= reduce_amount;
        if p.amount < 0 {
            p.amount = 0;
        }
        ReduceResult::Reduced
    } else {
        ReduceResult::Remove
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReduceResult {
    Reduced,
    Remove,
}

/// `RemoveSpecificPowerAction` 语义：目标已死/逃跑则放弃（onRemove 不触发）；
/// 否则先 `onRemove()`（副作用配对，如 DrawPower 恢复手牌上限——TODO 3.5）再移除。
pub fn remove_power(powers: &mut Vec<Power>, kind: PowerKind) -> bool {
    let Some(pos) = powers.iter().position(|p| p.kind == kind) else {
        return false;
    };
    powers.remove(pos);
    true
}

/// `atEndOfRound` 递减（回合型减益，记录 03 §2 的 justApplied 模式）。
/// 返回是否应移除（amount <= 0）。
pub fn end_of_round_tick(p: &mut Power) -> bool {
    if !super::is_turn_based_decrement(p.kind) {
        return false;
    }
    if p.just_applied {
        p.just_applied = false;
        return false;
    }
    p.amount -= 1;
    p.amount <= 0
}

/// 低级路径 `AbstractCreature.addPower`（记录 03 §2）：同 ID 叠层或 append，
/// **无钩子、无排序**——仅内部代码使用；标准施加一律走 `apply_power`。
pub fn add_power_raw(powers: &mut Vec<Power>, to_apply: Power) {
    if let Some(existing) = powers.iter_mut().find(|p| p.kind == to_apply.kind) {
        stack_power(existing, to_apply.amount, &mut Vec::new());
    } else {
        powers.push(to_apply);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_new_sorts_by_priority_stable() {
        let mut ps = vec![];
        let (o1, _) = apply_power(&mut ps, Power::new(PowerKind::Weak, 2), ApplyCtx::default());
        let (o2, _) = apply_power(&mut ps, Power::new(PowerKind::Frail, 2), ApplyCtx::default());
        assert_eq!(o1, ApplyOutcome::Applied { stacked: false });
        assert_eq!(o2, ApplyOutcome::Applied { stacked: false });
        assert_eq!(
            ps.iter().map(|p| p.kind).collect::<Vec<_>>(),
            vec![PowerKind::Frail, PowerKind::Weak]
        );
        // 同 priority 保持插入序（稳定排序）
        let mut ps2 = vec![];
        apply_power(&mut ps2, Power::new(PowerKind::Strength, 1), ApplyCtx::default());
        apply_power(&mut ps2, Power::new(PowerKind::Dexterity, 1), ApplyCtx::default());
        apply_power(&mut ps2, Power::new(PowerKind::Slow, 1), ApplyCtx::default());
        assert_eq!(
            ps2.iter().map(|p| p.kind).collect::<Vec<_>>(),
            vec![PowerKind::Strength, PowerKind::Dexterity, PowerKind::Slow]
        );
    }

    #[test]
    fn stack_existing_does_not_resort() {
        let mut ps = vec![];
        apply_power(&mut ps, Power::new(PowerKind::Strength, 5), ApplyCtx::default());
        apply_power(&mut ps, Power::new(PowerKind::Weak, 99), ApplyCtx::default());
        let (o, _) = apply_power(&mut ps, Power::new(PowerKind::Strength, 2), ApplyCtx::default());
        assert_eq!(o, ApplyOutcome::Applied { stacked: true });
        assert_eq!(ps[0].kind, PowerKind::Strength);
        assert_eq!(ps[0].amount, 7);
        assert_eq!(
            ps.iter().map(|p| p.kind).collect::<Vec<_>>(),
            vec![PowerKind::Strength, PowerKind::Weak]
        );
    }

    #[test]
    fn artifact_intercepts_debuff_only() {
        let mut ps = vec![Power::new(PowerKind::Artifact, 1)];
        let (o, pend) = apply_power(&mut ps, Power::new(PowerKind::Weak, 2), ApplyCtx::default());
        assert_eq!(o, ApplyOutcome::BlockedByArtifact);
        assert_eq!(pend, vec![PowerKind::Artifact]);
        assert_eq!(ps[0].amount, 0); // 移除由动作层执行（延迟语义）
        // BUFF（力量）不受 Artifact 拦截——原版 Strength 恒为 BUFF
        let (o2, _) = apply_power(&mut ps, Power::new(PowerKind::Strength, 3), ApplyCtx::default());
        assert_eq!(o2, ApplyOutcome::Applied { stacked: false });
        remove_power(&mut ps, PowerKind::Artifact);
        let (o3, _) = apply_power(&mut ps, Power::new(PowerKind::Weak, 2), ApplyCtx::default());
        assert_eq!(o3, ApplyOutcome::Applied { stacked: false });
    }

    #[test]
    fn ginger_turnip_block_player_debuffs() {
        let ctx = ApplyCtx {
            target_is_player: true,
            player_has_ginger: true,
            player_has_turnip: true,
        };
        let mut ps = vec![];
        assert_eq!(
            apply_power(&mut ps, Power::new(PowerKind::Weak, 2), ctx).0,
            ApplyOutcome::BlockedByRelic
        );
        assert_eq!(
            apply_power(&mut ps, Power::new(PowerKind::Frail, 2), ctx).0,
            ApplyOutcome::BlockedByRelic
        );
        assert_eq!(
            apply_power(&mut ps, Power::new(PowerKind::Vulnerable, 2), ctx).0,
            ApplyOutcome::Applied { stacked: false }
        );
        // 怪物目标不受遗物保护
        let mut ms = vec![];
        assert_eq!(
            apply_power(&mut ms, Power::new(PowerKind::Weak, 2), ApplyCtx::default()).0,
            ApplyOutcome::Applied { stacked: false }
        );
    }

    #[test]
    fn stack_sentinel_and_clamp() {
        let mut pend = vec![];
        let mut nos = Power::new(PowerKind::Artifact, -1);
        stack_power(&mut nos, 5, &mut pend);
        assert_eq!(nos.amount, -1);
        let mut s = Power::new(PowerKind::Strength, 998);
        stack_power(&mut s, 5, &mut pend);
        assert_eq!(s.amount, 999);
        let mut s0 = Power::new(PowerKind::Strength, 2);
        stack_power(&mut s0, -2, &mut pend);
        assert_eq!(s0.amount, 0);
        assert_eq!(pend, vec![PowerKind::Strength]);
    }

    #[test]
    fn reduce_threshold_semantics() {
        let mut p = Power::new(PowerKind::Vulnerable, 3);
        assert_eq!(reduce_power(&mut p, 2), ReduceResult::Reduced);
        assert_eq!(p.amount, 1);
        assert_eq!(reduce_power(&mut p, 1), ReduceResult::Remove);
        assert_eq!(reduce_power(&mut p, 99), ReduceResult::Remove);
    }

    #[test]
    fn end_of_round_just_applied_skip() {
        let mut v = Power::new_just_applied(PowerKind::Vulnerable, 2);
        assert!(!end_of_round_tick(&mut v));
        assert_eq!(v.amount, 2);
        assert!(!end_of_round_tick(&mut v));
        assert_eq!(v.amount, 1);
        assert!(end_of_round_tick(&mut v));
        // 玩家施加（无 justApplied）：当回合结束即递减
        let mut w = Power::new(PowerKind::Weak, 1);
        assert!(end_of_round_tick(&mut w));
    }

    #[test]
    fn add_power_raw_no_sort() {
        let mut ps = vec![Power::new(PowerKind::Weak, 2)];
        add_power_raw(&mut ps, Power::new(PowerKind::Frail, 2));
        assert_eq!(ps[0].kind, PowerKind::Weak);
        add_power_raw(&mut ps, Power::new(PowerKind::Weak, 1));
        assert_eq!(ps[0].amount, 3);
        assert_eq!(ps.len(), 2);
    }

    #[test]
    fn nodraw_no_stack() {
        let mut ps = vec![Power::new(PowerKind::NoDraw, 1)];
        let (o, _) = apply_power(&mut ps, Power::new(PowerKind::NoDraw, 1), ApplyCtx::default());
        assert_eq!(o, ApplyOutcome::NoDrawExists);
        assert_eq!(ps[0].amount, 1);
    }
}
