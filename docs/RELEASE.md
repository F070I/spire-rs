# 构建与发行（docs/08 执行版）

三平台发行操作手册。核心层（sts-data/sts-core/sts-interface）零外部依赖、
零平台 API——任何能编 Rust 的目标平台都可直接构建；终端层（sts-terminal）
为零依赖 ANSI 转义实现，同样无平台代码。

## 产物与体积

| 目标 | 命令 | 产物 | 实测体积 |
|---|---|---|---|
| Windows x86_64 | `cargo build --release -p sts-terminal` | `target/release/sts.exe` | ~0.64 MB |
| macOS Intel | `cargo build --release -p sts-terminal`（在 macOS 上） | `target/release/sts` | 同级 |
| macOS Apple Silicon | 同上（aarch64 原生） | 同上 | 同级 |
| Linux x86_64 | 同上 | 同上 | 同级 |

体积预算（<20MB）余量极大：release profile 已配 `lto="fat"` +
`codegen-units=1` + `strip=true` + `panic="abort"`（workspace Cargo.toml）。

## 交叉验证（在 Windows 开发机上）

无 macOS/Linux SDK 无法交叉**链接**，但可以交叉**编译检查**（类型/平台
API 校验，足以验证核心层可移植性）：

```bash
rustup target add x86_64-apple-darwin aarch64-apple-darwin x86_64-unknown-linux-gnu
cargo check  --workspace --all-targets --target x86_64-apple-darwin
cargo check  --workspace --all-targets --target aarch64-apple-darwin
cargo check  --workspace --all-targets --target x86_64-unknown-linux-gnu
```

三平台全部通过 = 无平台 API 泄漏（本项目 UI 走 stdin/stdout + ANSI，
核心层无 `#[cfg(windows)]` 类分支）。

## 正式发行（各平台原生构建）

- **Windows**：本机 `cargo build --release`（或配 GitHub Actions
  `windows-latest`，MSVC 或 GNU 工具链均可——本机工具链注意：
  gnu 缺 dlltool 时 crossterm 类依赖不可用，但本项目零依赖不受影响）。
- **macOS**：`rustup target add x86_64-apple-darwin`（Intel）或
  `aarch64-apple-darwin`（M 系列），双架构各 build 一次；或
  `cargo build --release --target aarch64-apple-darwin` 后 `lipo` 合并
  universal binary（可选）。
- **Linux**：`cargo build --release`；如需完全静态可换
  `x86_64-unknown-linux-musl` 目标。

## 冒烟测试（每平台发行前）

```bash
echo "q" | ./sts          # 启动即退出，无 panic
cargo test --workspace    # 184 测试全过（含 RNG 基线/拓扑守护/双主题等价）
```

## 打包清单（发行 zip）

- 可执行文件（sts / sts.exe）
- README（用法：`sts` 直接运行；行命令见启动帮助）
- LICENSE（workspace 声明 MIT）

## 已知平台注意

- Windows Git Bash 下 `link.exe` 冲突仅影响含 C 依赖的 crate
  （如 crossterm/parking_lot）——本项目已全部移除该类依赖。
- 终端需支持 UTF-8 与 ANSI 转义（Windows Terminal / iTerm2 / 主流
  Linux 终端均满足；老式 conhost 建议配合 `chcp 65001`）。
