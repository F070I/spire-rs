//! 战斗界面（docs UI 规范 §五）：固定区域、稳定对齐、全部状态可读。
//!
//! 布局（自上而下）：
//! 1. 状态栏：HP/能量/金币/层号
//! 2. 敌人区：意图（符号+数字）/HP/格挡
//! 3. 手牌：编号/名称/费用/攻防（选中态反显）
//! 4. 牌堆计数 + 可用命令提示
//!
//! 所有数值来自核心（eff_damage/eff_block/意图），UI 不重算。

use crate::ui_core::{kv, list_item, panel_title, pad_left, separator, str_width};
use crate::theme::Theme;
use sts_core::battle::BattleState;
use sts_data::gen::{CardType, CardTarget};

/// 意图显示符号 + 数值（来自核心 move 数据，不猜测）。
pub fn intent_text(m: &sts_core::battle::Monster) -> String {
    let move_val = m.script[m.move_idx.min(m.script.len() - 1)];
    match move_val {
        Some(d) => format!("⚔ {}", d), // 攻击意图 + 伤害
        None => "🛡 防御".to_string(),
    }
}

/// 战斗主视图（多行输出）。
pub fn render_battle(theme: &dyn Theme, b: &BattleState, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    out.push(panel_title(theme, "战斗", width));

    // 1. 状态栏
    let p = &b.player;
    out.push(format!(
        "{} {} {} {}",
        kv(theme, "HP", &format!("{}/{}", p.current_health, p.max_health), theme.hp()),
        kv(theme, "能量", &format!("{}/{}", b.energy, b.energy_master), theme.energy()),
        kv(theme, "格挡", &p.current_block.to_string(), theme.block()),
        kv(theme, "手牌", &b.hand.group.len().to_string(), theme.normal()),
    ));

    // 2. 敌人区
    out.push(separator(theme, width));
    for (i, m) in b.monsters.iter().enumerate() {
        if m.creature.is_dying || m.creature.is_dead {
            continue;
        }
        let intent = if m.is_attacking_intent() {
            theme.enemy_intent_attack().paint(&intent_text(m))
        } else {
            theme.enemy_intent_other().paint(&intent_text(m))
        };
        out.push(format!(
            "[敌{}] {} {} {}",
            i,
            pad_left(&m.creature.name, 12),
            kv(theme, "HP", &format!("{}/{}", m.creature.current_health, m.creature.max_health), theme.hp()),
            if m.creature.current_block > 0 {
                kv(theme, "格挡", &m.creature.current_block.to_string(), theme.block())
            } else {
                String::new()
            },
        ));
        out.push(format!("      意图: {}", intent));
    }

    // 3. 手牌
    out.push(separator(theme, width));
    for (i, c) in b.hand.group.iter().enumerate() {
        let unplayable = b.energy < c.cost_for_turn;
        let text = match c.st().kind {
            CardType::Attack => format!(
                "{} 费{} 攻{}",
                c.st().id, c.cost_for_turn, c.eff_damage()
            ),
            CardType::Skill => format!(
                "{} 费{} 块{}",
                c.st().id, c.cost_for_turn, c.eff_block()
            ),
            _ => format!("{} 费{}", c.st().id, c.cost_for_turn),
        };
        let text = if c.upgraded {
            format!("{}+", text)
        } else {
            text
        };
        let styled = if unplayable {
            theme.card_unplayable().paint(&format!("× {}", text))
        } else {
            match c.st().kind {
                CardType::Attack => theme.card_attack().paint(&text),
                CardType::Skill => theme.card_skill().paint(&text),
                _ => theme.normal().paint(&text),
            }
        };
        let needs_target = c.st().target == CardTarget::Enemy;
        let tgt = if needs_target { " [需目标]" } else { "" };
        out.push(format!("  {:>2}. {}{}", i + 1, styled, theme.dim().paint(tgt)));
    }

    // 4. 牌堆
    out.push(separator(theme, width));
    out.push(format!(
        "{} {} {}",
        kv(theme, "抽", &b.draw_pile.group.len().to_string(), theme.dim()),
        kv(theme, "弃", &b.discard_pile.group.len().to_string(), theme.dim()),
        kv(theme, "耗", &b.exhaust_pile.group.len().to_string(), theme.dim()),
    ));
    out
}

/// 奖励视图。
pub fn render_rewards(
    theme: &dyn Theme,
    gold: i32,
    cards: &[(usize, bool)],
    relic: Option<&str>,
    potion: Option<&str>,
    width: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    out.push(panel_title(theme, "战斗奖励", width));
    out.push(format!("  金币 +{}", theme.gold().paint(&gold.to_string())));
    for (i, (sid, up)) in cards.iter().enumerate() {
        let name = sts_data::gen::CARDS[*sid].id;
        let text = format!("{}{}", name, if *up { "+" } else { "" });
        out.push(list_item(theme, i, &text, true, true));
    }
    if let Some(r) = relic {
        out.push(format!("  [遗物] {}", theme.gold().paint(r)));
    }
    if let Some(p) = potion {
        out.push(format!("  [药水] {}", theme.normal().paint(p)));
    }
    out.push(theme.dim().paint("  r <n> 领卡 / r skip 跳过"));
    out
}

#[allow(dead_code)]
fn unused(s: &str) -> usize {
    str_width(s)
}
