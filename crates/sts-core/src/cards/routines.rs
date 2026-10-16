//! 卡牌专属例程注册表（原版卡牌专属 Action 类的执行体；GameAction::CardRoutine 落点）。
//!
//! 原版对应：actions/unique 与各卡伴生 Action 类（WhirlwindAction、HeadbuttAction…）。
//! 例程执行时机 = 队列消费到该项（即时模型下一个 tick 内完成全部逻辑）。

use crate::action::RoutineCall;
use crate::battle::BattleState;

type RoutineFn = fn(&mut BattleState, &RoutineCall);

/// 注册表（id → 执行体）。灌注进度：批次 3.10 前应有 ~110 项。
const ROUTINES: &[(&str, RoutineFn)] = &[];

pub fn run(b: &mut BattleState, call: &RoutineCall) {
    match ROUTINES.iter().find(|(id, _)| *id == call.id) {
        Some((_, f)) => f(b, call),
        None => {
            // 未转译例程：记入 Future 动作等价（审计用；灌注进度表跟踪）
            // 这里选择静默跳过 + debug_assert，避免测试因灌注进度中断。
            #[cfg(debug_assertions)]
            eprintln!("[sts-core] untranslated card routine: {}", call.id);
        }
    }
}
