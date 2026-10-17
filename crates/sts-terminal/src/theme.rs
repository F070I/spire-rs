//! 集中式主题配置（docs UI 规范 §三）：低饱和彩色 + 纯黑白两套。
//!
//! 规则：
//! - 颜色不作为区分节点/状态/操作的唯一方式（符号 + 线型同时携带信息）；
//! - 所有场景共用本配置，禁止页面自定义配色；
//! - 主题切换不改游戏状态。

/// ANSI SGR 参数序列（零依赖，无 crossterm）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    /// 前景 SGR 码（如 31 = 红）。None = 默认前景。
    pub fg: Option<u8>,
    /// 背景码。None = 默认背景。
    pub bg: Option<u8>,
    /// 加粗（终端下常为亮色）。
    pub bold: bool,
    /// 反显（黑白主题的选中态）。
    pub reverse: bool,
}

impl Style {
    pub const fn new(fg: Option<u8>) -> Self {
        Self {
            fg,
            bg: None,
            bold: false,
            reverse: false,
        }
    }

    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub const fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// 包裹文本：返回带转义序列的字符串。
    pub fn paint(&self, text: &str) -> String {
        if self.fg.is_none() && !self.bold && !self.reverse {
            return text.to_string();
        }
        let mut codes = String::new();
        if self.bold {
            codes.push_str("1;");
        }
        if self.reverse {
            codes.push_str("7;");
        }
        if let Some(fg) = self.fg {
            codes.push_str(&fg.to_string());
            codes.push(';');
        }
        format!("\x1b[{}m{}\x1b[0m", codes.trim_end_matches(';'), text)
    }
}

/// 语义角色 → 样式。两套主题实现同一接口。
pub trait Theme {
    fn name(&self) -> &'static str;

    // —— 通用 ——
    fn normal(&self) -> Style;
    fn dim(&self) -> Style; // 弱化信息（牌堆计数、提示）
    fn header(&self) -> Style; // 面板标题
    fn selected(&self) -> Style; // 选中态
    fn disabled(&self) -> Style; // 不可用项

    // —— 游戏状态 ——
    fn hp(&self) -> Style;
    fn block(&self) -> Style;
    fn energy(&self) -> Style;
    fn gold(&self) -> Style;

    // —— 节点类型（符号不变，仅色相区分）——
    fn node_monster(&self) -> Style;
    fn node_elite(&self) -> Style;
    fn node_rest(&self) -> Style;
    fn node_shop(&self) -> Style;
    fn node_event(&self) -> Style;
    fn node_treasure(&self) -> Style;
    fn node_boss(&self) -> Style;

    // —— 地图路径 ——
    fn edge_visited(&self) -> Style;
    fn edge_available(&self) -> Style;
    fn edge_future(&self) -> Style;

    // —— 战斗 ——
    fn enemy(&self) -> Style;
    fn enemy_intent_attack(&self) -> Style;
    fn enemy_intent_other(&self) -> Style;
    fn card_attack(&self) -> Style;
    fn card_skill(&self) -> Style;
    fn card_power(&self) -> Style;
    fn card_unplayable(&self) -> Style;
    fn log(&self) -> Style;
}

/// 低饱和彩色主题（暗底，256 色低饱和区 240-245 + 少量 130 系降饱和）。
pub struct ColorTheme;

impl Theme for ColorTheme {
    fn name(&self) -> &'static str {
        "color"
    }

    fn normal(&self) -> Style {
        Style::new(Some(250))
    }
    fn dim(&self) -> Style {
        Style::new(Some(243))
    }
    fn header(&self) -> Style {
        Style::new(Some(252)).bold()
    }
    fn selected(&self) -> Style {
        Style::new(Some(15)).bold()
    }
    fn disabled(&self) -> Style {
        Style::new(Some(240))
    }

    fn hp(&self) -> Style {
        Style::new(Some(174)) // 低饱和红
    }
    fn block(&self) -> Style {
        Style::new(Some(152)) // 低饱和青
    }
    fn energy(&self) -> Style {
        Style::new(Some(222)) // 低饱和琥珀
    }
    fn gold(&self) -> Style {
        Style::new(Some(179))
    }

    fn node_monster(&self) -> Style {
        Style::new(Some(250))
    }
    fn node_elite(&self) -> Style {
        Style::new(Some(216)).bold()
    }
    fn node_rest(&self) -> Style {
        Style::new(Some(151))
    }
    fn node_shop(&self) -> Style {
        Style::new(Some(179))
    }
    fn node_event(&self) -> Style {
        Style::new(Some(146))
    }
    fn node_treasure(&self) -> Style {
        Style::new(Some(222))
    }
    fn node_boss(&self) -> Style {
        Style::new(Some(174)).bold()
    }

    fn edge_visited(&self) -> Style {
        Style::new(Some(240))
    }
    fn edge_available(&self) -> Style {
        Style::new(Some(15)).bold()
    }
    fn edge_future(&self) -> Style {
        Style::new(Some(238))
    }

    fn enemy(&self) -> Style {
        Style::new(Some(174))
    }
    fn enemy_intent_attack(&self) -> Style {
        Style::new(Some(209)).bold()
    }
    fn enemy_intent_other(&self) -> Style {
        Style::new(Some(146))
    }
    fn card_attack(&self) -> Style {
        Style::new(Some(209))
    }
    fn card_skill(&self) -> Style {
        Style::new(Some(152))
    }
    fn card_power(&self) -> Style {
        Style::new(Some(183))
    }
    fn card_unplayable(&self) -> Style {
        Style::new(Some(240))
    }
    fn log(&self) -> Style {
        Style::new(Some(245))
    }
}

/// 纯黑白主题：高对比文本 + 反显选中 + 符号/线型携带全部语义。
pub struct MonoTheme;

impl Theme for MonoTheme {
    fn name(&self) -> &'static str {
        "mono"
    }

    fn normal(&self) -> Style {
        Style::new(None)
    }
    fn dim(&self) -> Style {
        Style::new(None) // 单色终端无弱化，靠符号
    }
    fn header(&self) -> Style {
        Style::new(None).bold()
    }
    fn selected(&self) -> Style {
        Style::new(None).reverse().bold()
    }
    fn disabled(&self) -> Style {
        Style::new(None) // 不可用项由 × 前缀标注
    }

    fn hp(&self) -> Style {
        Style::new(None).bold()
    }
    fn block(&self) -> Style {
        Style::new(None)
    }
    fn energy(&self) -> Style {
        Style::new(None).bold()
    }
    fn gold(&self) -> Style {
        Style::new(None)
    }

    fn node_monster(&self) -> Style {
        Style::new(None)
    }
    fn node_elite(&self) -> Style {
        Style::new(None).bold()
    }
    fn node_rest(&self) -> Style {
        Style::new(None)
    }
    fn node_shop(&self) -> Style {
        Style::new(None)
    }
    fn node_event(&self) -> Style {
        Style::new(None)
    }
    fn node_treasure(&self) -> Style {
        Style::new(None)
    }
    fn node_boss(&self) -> Style {
        Style::new(None).bold()
    }

    fn edge_visited(&self) -> Style {
        Style::new(None)
    }
    fn edge_available(&self) -> Style {
        Style::new(None).bold()
    }
    fn edge_future(&self) -> Style {
        Style::new(None) // 单色下虚线线型区分
    }

    fn enemy(&self) -> Style {
        Style::new(None).bold()
    }
    fn enemy_intent_attack(&self) -> Style {
        Style::new(None).bold()
    }
    fn enemy_intent_other(&self) -> Style {
        Style::new(None)
    }
    fn card_attack(&self) -> Style {
        Style::new(None).bold()
    }
    fn card_skill(&self) -> Style {
        Style::new(None)
    }
    fn card_power(&self) -> Style {
        Style::new(None).bold()
    }
    fn card_unplayable(&self) -> Style {
        Style::new(None) // × 前缀标注
    }
    fn log(&self) -> Style {
        Style::new(None)
    }
}

/// 当前激活主题（全局选择；切换不改游戏状态）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    Color,
    Mono,
}

pub fn theme(kind: ThemeKind) -> &'static dyn Theme {
    match kind {
        ThemeKind::Color => &ColorTheme,
        ThemeKind::Mono => &MonoTheme,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_themes_implement_all_roles() {
        for kind in [ThemeKind::Color, ThemeKind::Mono] {
            let t = theme(kind);
            // 冒烟：全部接口可调用
            assert!(!t.name().is_empty());
            let _ = t.normal().paint("x");
            let _ = t.selected().paint("y");
            let _ = t.node_boss().paint("B");
        }
    }

    #[test]
    fn mono_selected_is_reverse() {
        let s = MonoTheme.selected();
        assert!(s.reverse, "黑白主题选中态必须反显");
    }

    #[test]
    fn color_uses_low_saturation_range() {
        // 低饱和约束：主体色在 240-245（灰阶）与降饱和色相
        assert_eq!(ColorTheme.dim().fg, Some(243));
        assert_eq!(ColorTheme.edge_future().fg, Some(238));
    }
}
