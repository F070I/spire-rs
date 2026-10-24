# 寓+签到 — 真正独立的 macOS App（构建工程）

> **这是 macOS 自动构建工程，不是已经编译的安装包。** 需在 GitHub Actions 的 macOS 机器构建后，下载完整可运行的 `.app` 压缩包。开发者也可在 Mac 上用工作流中的命令本地构建。Linux 环境不能真正编译 Cocoa + WebKit 程序；不要将此源码 ZIP 当成已可双击运行的 App。

## 用户在新 Mac 上的体验

1. 在 GitHub Actions 的对应构建中，下载 `YujiaSignIn-macOS-x86_64`（Intel）或 `YujiaSignIn-macOS-arm64`（Apple 芯片）构建产物，解压得到 `寓+签到-macOS-*.zip`。
2. 再次解压，只有一个 `寓+签到.app`；拖入 Mac 的「应用程序」，双击打开。**无需 Python、Xcode、命令行工具或旁边的源码文件夹**。
3. 打开后会自动检查 / 注册 `launchd` 的定时服务、启动本地面板。无需以管理员身份运行；关闭窗口后后台照常执行，每次重新登录系统可恢复。首次使用，在面板里直接添加账号和设置时间。
4. macOS 对未经苹果开发者账号公证的 App 可能阻止首次打开，可在 Finder 中右键 → 打开，或到系统设置 → 隐私与安全性按提示允许；这属于 Gatekeeper 限制，不是程序依赖。需要公证才能做到完全无安全提示。

## 从旧版迁移一次

从 App 顶部「寓+签到」菜单选择 **导入旧版账号和记录…**，选择旧的 `yujia-signin` 文件夹。会先停止用相同 `launchd` Label 安装的旧服务、在新目录备份旧数据后导入 `config.json` / `records.db`，保留原文件。**旧版本若曾用 `nohup python run_daily.py` 单独常驻运行，应在迁移前退出那个旧进程**，否则可能继续按 09:05 执行。

用户数据（属于正常 macOS 系统存储，对 Finder 日常使用透明）统一在：
`~/Library/Application Support/yujia-signin/`

包括 `config.json`、`records.db`、`signin.log`、`web.log`、`launchd.log`。关闭 App 或更新 App 不会删数据。复制 App 到新电脑后数据不会凭空同步；可在旧 Mac 使用导入迁移、备份方式迁移。

## 构建一次、以后所有新电脑直接复制

1. 新建私人 GitHub 仓库，将本工程**所有内容（包括 `.github/workflows/build-macos.yml`）**上传到根目录。仓库不要上传旧 `config.json`、token、数据库。
2. GitHub → Actions → `Build independent macOS app` → `Run workflow`。
3. 等两套构建完成后，在工作流详情的 **Artifacts** 下载对应架构安装包。每台新电脑只需得到那份已编译的 `.app`，不需要 GitHub、源码或编译工具。

构建流程会运行本地假 API 测试、打包 Python + 证书根、编译原生 Cocoa/WebKit 窗口、代码签名（ad-hoc），启动打包后的本地 Web 面板健康检查，再提供下载产物。流水线本身需要 macOS runner。现在提供的是**未经真正 macOS runner 实测的构建工程**，并不保证首次 CI 一定成功。

## macOS / Debian 分工

- macOS：从工作流下载完成的 `.app`；后台由系统 `launchd` 管理；图形界面负责账号、计划、日志、导入数据。
- Debian：保留仓库根目录的 `Dockerfile` 和 `docker-compose.yml`，Dockerfile 会从 `core/` 复制共用 Python 代码，`docker compose up -d --build` 即可部署（Docker 实机仍需你验证）。

## 注意

- macOS 12+，提供 Intel x86_64 和 Apple Silicon arm64 两个安装包。仅在可信设备和可信网络使用；默认只监听 127.0.0.1:8765。
- 端口 8765 不能同时被另一套旧版 Web 服务占用。若旧调度器是手工常驻启动，迁移前应停止旧进程。
- `launchd` 登录启动与 Mac 睡眠无关：电脑关机时不能签到；醒来/登录后会由调度器进行当日补跑。
- PyInstaller 捆绑的运行环境会使 `.app` 明显大于原有脚本。独立安装与不足 1MB 无法兼得。
