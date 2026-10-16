//! sts-terminal：文字终端 UI 与可执行入口。
//!
//! 阶段 0：仅入口骨架（版本信息与文档指引）。终端框架随批次 6.2 落地：
//! - 输入/raw mode/颜色：crossterm（跨平台统一层，docs/08 §2）
//! - 布局：docs/04 §3 的固定区域设计
//! - 平台差异全部收敛于本 crate 的 term 模块，核心层零平台 API。

fn main() {
    println!("Slay the Spire — Text Edition (Rust)");
    println!("版本: {}", env!("CARGO_PKG_VERSION"));
    println!("阶段 0（设计与盘点）已完成；游戏实现按 docs/05 批次推进。");
    println!("项目文档: docs/00-项目总规范.md");
}
