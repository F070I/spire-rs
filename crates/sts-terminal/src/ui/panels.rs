//! 其他场景面板（docs UI 规范 §六）：篝火/商店/事件/牌库/药水栏——统一列表详情风格。

use crate::ui_core::{list_item, panel_title, separator};
use crate::theme::Theme;

/// 篝火选项（Rest/Smith 固定 + 遗物选项占位）。
pub fn render_campfire(theme: &dyn Theme, can_smith: bool, width: usize) -> Vec<String> {
    let mut out = vec![
        panel_title(theme, "篝火", width),
        list_item(theme, 0, "休息（回复 30% 最大生命）", true, true),
    ];
    out.push(list_item(theme, 1, "锻造（升级一张牌）", true, can_smith));
    out
}

/// 商店视图（商品列表：卡/遗物/药水/移除服务）。
pub fn render_shop(
    theme: &dyn Theme,
    gold: i32,
    items: &[(&str, i32)],
    width: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    out.push(panel_title(theme, "商店", width));
    out.push(format!("  金币 {}", theme.gold().paint(&gold.to_string())));
    out.push(separator(theme, width));
    for (i, (name, price)) in items.iter().enumerate() {
        let afford = gold >= *price;
        out.push(list_item(
            theme,
            i,
            &format!("{} — {}金", name, price),
            false,
            afford,
        ));
    }
    out
}

/// 事件视图（标题 + 正文 + 编号选项）。
pub fn render_event(
    theme: &dyn Theme,
    title: &str,
    body: &str,
    options: &[String],
    width: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    out.push(panel_title(theme, title, width));
    for line in body.lines() {
        out.push(format!("  {}", theme.normal().paint(line)));
    }
    out.push(separator(theme, width));
    for (i, o) in options.iter().enumerate() {
        out.push(list_item(theme, i, o, true, true));
    }
    out
}

/// 牌库视图（排序 + 计数；单卡详情走 InspectCard）。
pub fn render_deck(theme: &dyn Theme, deck: &[sts_core::battle::CardInst], width: usize) -> Vec<String> {
    let mut out = Vec::new();
    out.push(panel_title(theme, &format!("牌库（{} 张）", deck.len()), width));
    let mut sorted: Vec<&sts_core::battle::CardInst> = deck.iter().collect();
    sorted.sort_by_key(|c| (c.eff_cost(), c.st().id));
    for c in sorted {
        let text = match c.st().kind {
            sts_data::gen::CardType::Attack => {
                format!("{} 攻{} 费{}", c.st().id, c.eff_damage(), c.eff_cost())
            }
            sts_data::gen::CardType::Skill => {
                format!("{} 块{} 费{}", c.st().id, c.eff_block(), c.eff_cost())
            }
            _ => format!("{} 费{}", c.st().id, c.eff_cost()),
        };
        let text = if c.upgraded {
            format!("{}+", text)
        } else {
            text
        };
        out.push(format!("  {}", text));
    }
    out
}

/// 药水栏（3 槽）。
pub fn render_potions(theme: &dyn Theme, potions: &[Option<&'static str>], width: usize) -> Vec<String> {
    let mut out = Vec::new();
    out.push(panel_title(theme, "药水", width));
    for (i, p) in potions.iter().enumerate() {
        let text = p.map(|id| id.to_string()).unwrap_or_else(|| "空".to_string());
        out.push(list_item(theme, i, &text, false, p.is_some()));
    }
    out
}

/// 遗物栏。
pub fn render_relics(theme: &dyn Theme, relics: &[&'static str], width: usize) -> Vec<String> {
    let mut out = Vec::new();
    out.push(panel_title(theme, "遗物", width));
    if relics.is_empty() {
        out.push(theme.dim().paint("  （无）"));
    }
    for (i, r) in relics.iter().enumerate() {
        out.push(list_item(theme, i, r, false, true));
    }
    out
}

/// 死亡/胜利结算。
pub fn render_game_over(theme: &dyn Theme, victory: bool, floor: i64, gold: i32, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let title = if victory { "胜利" } else { "死亡" };
    out.push(panel_title(theme, title, width));
    out.push(format!("  到达层级 {}", floor));
    out.push(format!("  金币 {}", gold));
    out
}
