# AutoSnakeSaver — macOS Monterey Intel source

这是 **Intel x86_64** 的原生屏保源码工程，最低部署目标 macOS 12.0（适用于 Monterey 12.7）。当前构建环境不是 macOS，没有 Apple SDK / Xcode，因此交付的是可在 Mac 上构建的源码工程，**不是已编译的 `.saver` 二进制**。

## 构建

1. 把本目录复制到 Intel Mac（macOS 12.7，安装完整 Xcode（或者使用 GitHub Actions 云编译，详见 README_GITHUB_BUILD.md））。
2. 在 Terminal 执行：`./build-intel.sh`。
3. 若构建成功，产物位于 `dist/AutoSnakeSaverIntel.saver`。
4. 安装到 `~/Library/Screen Savers/`，然后在系统设置的屏幕保护程序中选择它。首次本地未签名构建可能触发 macOS 安全提示；只对自己编译、确认来源的构建执行手动允许。

## 注意

- 目标架构明确为 `x86_64`，不包含 arm64。
- 使用系统 WebKit 渲染共享的 v21 HTML/JS 核心；原生容器负责 ScreenSaverView 生命周期和设置面板。
- 这个源码包需要在真实 Intel Mac 上完成首次编译、安装、预览、睡眠/唤醒验证后，才能称为已验证的原生屏保。
