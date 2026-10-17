//! 卡牌效果分发（批次 3 内容灌注的落点）。
//!
//! 结构对应原版 cards/{red,green,blue,purple,colorless,...} 包：
//! - 每个颜色模块 `effects()` 内 match 卡牌规范 ID → 动作序列；
//! - `handled_ids()` 列出已转译卡（灌注进度核对 + G3 冒烟测试范围）；
//! - 未转译卡回落 `generic_effects()`（通用攻击/防御/能力占位），由灌注进度表跟踪；
//! - 卡牌专属例程（原版卡牌 Action 类）在 `routines.rs` 注册执行。

pub mod builder;
pub mod blue;
pub mod colorless;
pub mod simple;
pub mod purple;
pub mod green;
pub mod red;
pub mod routines;

use crate::action::{GameAction, RoutineCall, TargetRef};
use crate::battle::{BattleState, CardInst};
use crate::cards::builder::EffectBuilder;
use crate::damage::{self, DamageInfo};
use crate::power::DamageType;
use sts_data::gen::{self, CardType};

/// 全部已转译卡 ID（由各颜色模块合并；G3 冒烟测试按此遍历）。
pub fn handled_ids() -> Vec<&'static str> {
    let mut v = Vec::new();
    v.extend_from_slice(red::HANDLED);
    v.extend_from_slice(green::HANDLED);
    v.extend_from_slice(blue::HANDLED);
    v.extend_from_slice(purple::HANDLED);
    v.extend_from_slice(colorless::HANDLED);
    v.extend_from_slice(simple::HANDLED_CURSES);
    v.extend_from_slice(simple::HANDLED_STATUS);
    v.extend_from_slice(simple::HANDLED_TEMP);
    v.extend_from_slice(simple::HANDLED_OPTIONS);
    v
}

/// 分发入口（consume_card_queue_item 调用）。
pub fn effects(b: &mut BattleState, card: &CardInst, target: Option<usize>) -> Vec<GameAction> {
    let mut eb = EffectBuilder::new();
    let id = card.st().id;
    let handled = match card.st().color_dir {
        "red" => red::HANDLED.contains(&id),
        "green" => green::HANDLED.contains(&id),
        "blue" => blue::HANDLED.contains(&id),
        "purple" => purple::HANDLED.contains(&id),
        "colorless" => colorless::HANDLED.contains(&id),
        "curses" => simple::HANDLED_CURSES.contains(&id),
        "status" => simple::HANDLED_STATUS.contains(&id),
        "tempCards" => simple::HANDLED_TEMP.contains(&id),
        // 批次 3.3-3.4 逐色接入
        _ => false,
    };
    if handled {
        match card.st().color_dir {
            "red" => red::effects(b, card, target, &mut eb),
            "green" => green::effects(b, card, target, &mut eb),
            "blue" => blue::effects(b, card, target, &mut eb),
            "purple" => purple::effects(b, card, target, &mut eb),
            "colorless" => colorless::effects(b, card, target, &mut eb),
            "curses" => simple::effects_curses(b, card, target, &mut eb),
            "status" => simple::effects_status(b, card, target, &mut eb),
            "tempCards" => simple::effects_temp(b, card, target, &mut eb),
            "optionCards" => simple::effects_option(b, card, target, &mut eb),
            _ => unreachable!("handled 表与分发分支不同步"),
        }
    } else {
        generic_effects(b, card, target, &mut eb);
    }
    eb.finish()
}

/// 未转译卡的通用行为（与原版不等价，灌注进度跟踪；G3 门后应只剩特殊卡）。
pub fn generic_effects(b: &mut BattleState, card: &CardInst, target: Option<usize>, eb: &mut EffectBuilder) {
    let st = card.st();
    match st.kind {
        CardType::Attack => {
            if card.eff_damage() > 0 {
                if let Some(mi) = target {
                    let info = DamageInfo::from_card(
                        &b.player.powers,
                        b.player_stance,
                        &b.monsters[mi].creature.powers,
                        card.eff_damage(),
                        DamageType::Normal,
                    );
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info,
                    });
                }
            }
        }
        CardType::Skill if card.eff_block() > 0 => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        _ => {}
    }
}

/// 例程执行入口（execute_action 的 CardRoutine 分支）。
pub fn run_routine(b: &mut BattleState, call: &RoutineCall) {
    routines::run(b, call);
}

/// 按规范 ID 查卡表索引（生成临时卡用）。
pub fn sid_of(id: &str) -> usize {
    sts_data::gen::CARDS
        .iter()
        .position(|c| c.id == id)
        .unwrap_or_else(|| panic!("card id not in table: {id}"))
}

/// 高频辅助：对目标的攻击 DamageInfo（打出瞬间重算，记录 09 §1 阶段C）。
pub fn attack_info(
    b: &BattleState,
    target_monster: usize,
    base: i32,
    ty: DamageType,
) -> DamageInfo {
    DamageInfo::from_card(
        &b.player.powers,
        b.player_stance,
        &b.monsters[target_monster].creature.powers,
        base,
        ty,
    )
}

/// 高频辅助：全体敌人伤害矩阵（DamageInfo.createDamageMatrix，记录 03 §4a）。
pub fn damage_matrix(b: &BattleState, base: i32, ty: DamageType) -> Vec<(usize, i32)> {
    b.monsters
        .iter()
        .enumerate()
        .filter(|(_, m)| !m.creature.is_dying && !m.creature.is_dead)
        .map(|(i, m)| {
            (
                i,
                damage::info_apply_powers(
                    &b.player.powers,
                    true,
                    &m.creature.powers,
                    b.player_stance,
                    base,
                    ty,
                ),
            )
        })
        .collect()
}

#[allow(dead_code)]
fn _st_guard(_s: &'static gen::CardStatic) {}
