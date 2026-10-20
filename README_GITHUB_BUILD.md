# AutoSnakeSaver v21 — GitHub 无 Xcode 云编译（Intel Mac / macOS 12.7）

你自己的 Mac **不需要安装 Xcode**。GitHub Actions 会在云端的 Intel Mac 构建机编译 `AutoSnakeSaverIntel.saver`。

## 最省事的方法：使用 GitHub Desktop 上传

1. 在 GitHub 登录，创建**空仓库**：仓库名建议 `AutoSnakeSaver-Intel`；不必勾选添加 README/gitignore。
2. 将这个目录原样解压在电脑上，务必保留隐藏文件夹 `.github/workflows/`。
3. 用 [GitHub Desktop](https://desktop.github.com/) 的 **File → Add local repository** 选择这个目录。如果提示不是 Git 仓库，点击 **create a repository here**。
4. 在 GitHub Desktop 提交全部文件（Commit to main），点击 **Publish repository**。如果之前已经在 GitHub 建好同名空仓库，按 GitHub Desktop 提示连接远程后 Push。可以建 Public 或 Private；私有仓库的 macOS Actions 消耗账户配额。
5. 到 GitHub 仓库网页 → **Actions** → **Build Intel macOS Screen Saver**。首次启用工作流时按提示启用，然后点 **Run workflow**（默认推送也会自动触发构建）。
6. 等待出现绿色对勾；点开那次运行 → 页面下方 **Artifacts** → 下载 `AutoSnakeSaver-v21-Intel-macOS12`。
7. 下载的 Actions 外层 ZIP 解压后，还有一个 `AutoSnakeSaverIntel-macOS12-Intel.zip`；再解压可得到 `AutoSnakeSaverIntel.saver`。
8. Mac 上安装到当前用户目录：
   `mkdir -p "$HOME/Library/Screen Savers"`
   `cp -R AutoSnakeSaverIntel.saver "$HOME/Library/Screen Savers/"`
9. 打开 **系统偏好设置 → 桌面与屏幕保护程序 → 屏幕保护程序**，选择 AutoSnakeSaverIntel 并测试。

## 无 GitHub Desktop 方法

也可以先在 GitHub 网站新建仓库，再通过 **Add file → Upload files** 上传项目内的所有文件及其子目录。一定要确认仓库网页中存在 `.github/workflows/build-macos-intel.yml`（如果隐藏目录没有传上去，工作流不会启动）。

## 问题定位

- **找不到 Actions 工作流**：检查仓库根目录是否有 `.github/workflows/build-macos-intel.yml`，且已提交到默认分支。
- **红叉失败**：进入运行记录 → `build-intel` → 找第一条 `error:`，复制日志给我。云端有 Xcode，不必在自己的 Mac 安装。
- **无法安装/无法预览**：这次构建产物未签名、未公证，只建议自己测试；首次可能触发 macOS 安全提示。只对你自己确认来源的文件进行手动授权，不要为此关闭整个系统的 Gatekeeper。
- **编译成功不等于兼容性验证**：需要在 macOS 12.7 的 Intel 真机上验证实际屏保预览、设置、休眠与唤醒。

目标：x86_64、macOS 最低部署目标 12.0，实际主要测试目标 12.7。不是 arm64 / Apple Silicon 版。
