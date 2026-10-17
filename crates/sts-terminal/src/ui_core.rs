//! 通用 UI 组件（docs UI 规范 §三）：显示宽度、面板、列表、选中态。
//!
//! 规则：同类组件全场景一致；宽度计算含中文全宽；裁切不丢语义。

use crate::theme::{Style, Theme};

/// 单字符显示宽度（终端等宽网格）：CJK/全宽 = 2，其余 = 1。
/// 覆盖常用范围：CJK 统一表意、扩展 A、兼容表意、全角标点、Hangul。
pub fn char_width(c: char) -> usize {
    let u = c as u32;
    if (0x1100..=0x115F).contains(&u) // Hangul Jamo
        || (0x2E80..=0xA4CF).contains(&u) // CJK 部首~Yi
        || (0xA960..=0xA97F).contains(&u) // Hangul Jamo Extended-A
        || (0xAC00..=0xD7A3).contains(&u) // Hangul 音节
        || (0xF900..=0xFAFF).contains(&u) // CJK 兼容表意
        || (0xFE30..=0xFE4F).contains(&u) // CJK 兼容形式
        || (0xFF00..=0xFF60).contains(&u) // 全角形式
        || (0xFFE0..=0xFFE6).contains(&u) // 全角符号
        || (0x20000..=0x3FFFD).contains(&u)
    {
        2
    } else {
        1
    }
}

/// 字符串显示宽度。
pub fn str_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

/// 按显示宽度裁切（不丢内容语义——超出部分省略号）。
pub fn truncate(s: &str, max_width: usize) -> String {
    if str_width(s) <= max_width {
        return s.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = char_width(c);
        if w + cw > max_width.saturating_sub(1) {
            break;
        }
        out.push(c);
        w += cw;
    }
    out.push('…');
    out
}

/// 左对齐填充到指定显示宽度。
pub fn pad_left(s: &str, width: usize) -> String {
    let w = str_width(s);
    if w >= width {
        truncate(s, width).to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - w))
    }
}

/// 右对齐填充。
pub fn pad_right(s: &str, width: usize) -> String {
    let w = str_width(s);
    if w >= width {
        truncate(s, width).to_string()
    } else {
        format!("{}{}", " ".repeat(width - w), s)
    }
}

/// 列表项渲染：选中态前缀 + 编号 + 文本（统一全场景）。
pub fn list_item(theme: &dyn Theme, idx: usize, text: &str, selected: bool, enabled: bool) -> String {
    let prefix = match (selected, enabled) {
        (true, _) => theme.selected().paint("▸ "),
        (false, true) => "  ".to_string(),
        (false, false) => theme.disabled().paint("× "),
    };
    let num = format!("{:>2}. ", idx + 1);
    let body = if enabled {
        if selected {
            theme.selected().paint(text)
        } else {
            theme.normal().paint(text)
        }
    } else {
        theme.disabled().paint(text)
    };
    format!("{}{}{}", prefix, num, body)
}

/// 分隔线（细）。
pub fn separator(theme: &dyn Theme, width: usize) -> String {
    theme.dim().paint(&"─".repeat(width))
}

/// 面板标题行：`── 标题 ──…`。
pub fn panel_title(theme: &dyn Theme, title: &str, width: usize) -> String {
    let t = theme.header().paint(title);
    let used = str_width(title) + 4;
    if used >= width {
        return t;
    }
    format!("── {} {}", t, theme.dim().paint(&"─".repeat(width - used)))
}

/// 状态键值对（"HP 72/80" 形式），统一间距。
pub fn kv(theme: &dyn Theme, key: &str, val: &str, val_style: Style) -> String {
    format!(
        "{} {}",
        theme.dim().paint(key),
        val_style.paint(val)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::MonoTheme;

    #[test]
    fn cjk_width_is_two() {
        assert_eq!(char_width('中'), 2);
        assert_eq!(char_width('a'), 1);
        assert_eq!(char_width('→'), 1);
        assert_eq!(str_width("打击"), 4);
        assert_eq!(str_width("Strike"), 6);
    }

    #[test]
    fn truncate_keeps_cjk_boundary() {
        let t = truncate("打击伤害", 5);
        // 5 宽只容 "打击"（4）+ "…"(1)
        assert_eq!(str_width(&t), 5);
        assert!(t.starts_with("打击"));
    }

    #[test]
    fn pad_mixed_width() {
        assert_eq!(str_width(&pad_left("HP", 6)), 6);
        assert_eq!(str_width(&pad_left("生命", 6)), 6);
    }

    #[test]
    fn list_item_marks_disabled() {
        let t = &MonoTheme;
        let sel = list_item(t, 0, "A", true, true);
        assert!(sel.contains('▸'));
        let dis = list_item(t, 1, "B", false, false);
        assert!(dis.contains('×'));
    }
}
