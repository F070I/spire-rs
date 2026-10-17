//! 统一 UI 组件入口。
//!
//! 结构：
//! - ui_core：宽度/面板/列表/选中态等通用原语
//! - ui/map：地图全图渲染 + 拓扑测试
//! - ui/battle：战斗界面
//! - ui/panels：篝火/商店/事件/牌库/药水/遗物/结算

pub mod battle;
pub mod map;
pub mod panels;

pub use super::ui_core::panel_title;
