//! 流程外壳 —— 模式状态机与 Screen 路由（原版 CardCrawlGame / AbstractDungeon 屏幕逻辑的等价物）。
//!
//! 阶段 0 骨架：仅占位结构；批次 5.5 落地完整路由（含 previousScreen/screenSwap 返回栈、
//! DEATH 接管、TopPanel Esc 路由表等价物——依据记录 08 §1/§4）。

use crate::state::{Game, ScreenStack};
use sts_data::{GameMode, ScreenKind};

/// 当前流程状态。TODO(批次5.5)：SPLASH 阶段与转场状态机。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowPhase {
    CharSelect,
    InRun,
}

impl Game {
    pub fn phase(&self) -> FlowPhase {
        match self.mode {
            GameMode::CharSelect => FlowPhase::CharSelect,
            GameMode::Gameplay => FlowPhase::InRun,
        }
    }

    /// 打开屏幕（等价原版 openScreen：记录 previous_screen 或置 swap）。
    pub fn open_screen(&mut self, screen: ScreenKind) {
        let stack: &mut ScreenStack = &mut self.screens;
        if stack.screen != ScreenKind::None {
            stack.previous = Some(stack.screen);
        }
        stack.screen = screen;
    }

    /// 关闭当前屏幕（等价 closeCurrentScreen 的路由骨架；特例随批次 5.5 补全）。
    pub fn close_screen(&mut self) {
        let stack = &mut self.screens;
        if stack.previous == Some(stack.screen) {
            stack.previous = None;
        }
        stack.screen = match stack.previous.take() {
            Some(prev) => prev,
            None => ScreenKind::None,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_stack_roundtrip() {
        let mut g = Game::new();
        g.open_screen(ScreenKind::Map);
        g.open_screen(ScreenKind::MasterDeckView);
        assert_eq!(g.screens.screen, ScreenKind::MasterDeckView);
        g.close_screen();
        assert_eq!(g.screens.screen, ScreenKind::Map);
        assert_eq!(g.screens.previous, None);
        g.close_screen();
        assert_eq!(g.screens.screen, ScreenKind::None);
    }
}
