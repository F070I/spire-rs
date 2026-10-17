//! sts-core：完整的游戏状态、规则、随机数管理、地图、战斗、奖励、流程。
//!
//! 硬性约束（docs/03-架构冻结.md）：
//! - 不依赖终端 UI、终端尺寸、颜色、键盘、显示布局；
//! - 不依赖任何平台专有 API；
//! - 不使用全局可变静态承载游戏状态——状态根为 [`Game`]。
//!
//! 模块按 docs/02-模块映射.md 的批次逐步落地；当前为阶段 0 骨架，
//! 各模块以 `// TODO(批次N.N)` 标注落地点。

pub mod state;
pub mod flow;
pub mod power;
pub mod stance;
pub mod damage;
pub mod creature;
pub mod action;
pub mod orb;
pub mod cards;
pub mod event;
pub mod flow4b;
pub mod metrics;
pub mod save;
pub mod map;
pub mod monsters;
pub mod battle;

pub use state::Game;
