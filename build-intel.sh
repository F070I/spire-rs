#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "需要 macOS 构建环境（本地 Xcode 或 GitHub Actions macOS runner）" >&2
  exit 2
fi

# 使用可预测的输出路径，避免不同 Xcode / CI 主机 DerivedData 路径变化。
DEST="$PWD/.build/Release"
rm -rf "$DEST"
xcodebuild \
  -project AutoSnakeSaverIntel.xcodeproj \
  -target AutoSnakeSaverIntel \
  -configuration Release \
  -sdk macosx \
  ARCHS=x86_64 \
  ONLY_ACTIVE_ARCH=NO \
  MACOSX_DEPLOYMENT_TARGET=12.0 \
  CODE_SIGNING_ALLOWED=NO \
  CONFIGURATION_BUILD_DIR="$DEST" \
  build

SAVER="$DEST/AutoSnakeSaverIntel.saver"
if [[ ! -d "$SAVER" ]]; then
  echo "构建未生成预期文件：$SAVER" >&2
  exit 3
fi

EXE="$SAVER/Contents/MacOS/AutoSnakeSaverIntel"
if [[ ! -f "$EXE" ]]; then
  echo "屏保缺少原生可执行文件：$EXE" >&2
  exit 4
fi

lipo -archs "$EXE" | grep -qw x86_64
plutil -lint "$SAVER/Contents/Info.plist"
test -f "$SAVER/Contents/Resources/AutoSnakeWeb/index.html"
test -f "$SAVER/Contents/Resources/AutoSnakeWeb/js/main.js"

mkdir -p dist
rm -rf dist/AutoSnakeSaverIntel.saver dist/AutoSnakeSaverIntel-macOS12-Intel.zip
/usr/bin/ditto "$SAVER" dist/AutoSnakeSaverIntel.saver
/usr/bin/ditto -c -k --keepParent --sequesterRsrc dist/AutoSnakeSaverIntel.saver dist/AutoSnakeSaverIntel-macOS12-Intel.zip

echo "✅ 已编译：dist/AutoSnakeSaverIntel-macOS12-Intel.zip"
