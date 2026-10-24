#!/bin/bash
# Idempotent self-contained installation for a GUI .app (no system Python required).
set -euo pipefail
BACKEND="$(cd "$(dirname "$0")" && pwd)"
RESOURCES="$(cd "$BACKEND/.." && pwd)"
BIN="$RESOURCES/runtime/YujiaService/YujiaService"
DATA="$HOME/Library/Application Support/yujia-signin"
AGENTS="$HOME/Library/LaunchAgents"
DAILY="$AGENTS/com.yujia.signin.plist"
WEB="$AGENTS/com.yujia.signinweb.plist"
UID_NUM="$(id -u)"
PORT=8765

if [ ! -x "$BIN" ]; then
    echo "应用内部缺少签到运行程序，请重新安装完整 App。" >&2
    exit 1
fi
mkdir -p "$DATA" "$AGENTS"
chmod 700 "$DATA"

xml_escape() {
    printf '%s' "$1" | /usr/bin/sed -e 's/\&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/"/\&quot;/g' -e "s/'/\&apos;/g"
}
ESC_BIN="$(xml_escape "$BIN")"
ESC_DATA="$(xml_escape "$DATA")"

# Stop both previous versions that used these exact agent labels.
for label in com.yujia.signin com.yujia.signinweb; do
    /bin/launchctl bootout "gui/$UID_NUM/$label" >/dev/null 2>&1 || true
done
/bin/launchctl unload "$DAILY" >/dev/null 2>&1 || true
/bin/launchctl unload "$WEB" >/dev/null 2>&1 || true

# Only one scheduler: periodic --if-due check, which reads its actual time from DB.
# No extra calendar trigger; the database is the authoritative schedule.
cat > "$DAILY" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>Label</key><string>com.yujia.signin</string>
    <key>ProgramArguments</key><array>
        <string>$ESC_BIN</string><string>--if-due</string>
    </array>
    <key>EnvironmentVariables</key><dict>
        <key>YUJIA_CONFIG_DIR</key><string>$ESC_DATA</string>
        <key>SIGNIN_TIME</key><string>09:05</string>
    </dict>
    <key>StartInterval</key><integer>600</integer>
    <key>RunAtLoad</key><true/>
    <key>StandardOutPath</key><string>$ESC_DATA/launchd.log</string>
    <key>StandardErrorPath</key><string>$ESC_DATA/launchd.log</string>
</dict></plist>
EOF
cat > "$WEB" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>Label</key><string>com.yujia.signinweb</string>
    <key>ProgramArguments</key><array>
        <string>$ESC_BIN</string><string>--panel</string>
    </array>
    <key>EnvironmentVariables</key><dict>
        <key>YUJIA_CONFIG_DIR</key><string>$ESC_DATA</string>
        <key>YUJIA_WEB_PORT</key><string>$PORT</string>
    </dict>
    <key>KeepAlive</key><true/>
    <key>RunAtLoad</key><true/>
    <key>StandardOutPath</key><string>$ESC_DATA/web.log</string>
    <key>StandardErrorPath</key><string>$ESC_DATA/web.log</string>
</dict></plist>
EOF
chmod 600 "$DAILY" "$WEB"
/usr/bin/plutil -lint "$DAILY" "$WEB" >/dev/null
/bin/launchctl bootstrap "gui/$UID_NUM" "$DAILY"
/bin/launchctl bootstrap "gui/$UID_NUM" "$WEB"
echo "后台已安装，设置和记录保存在系统应用数据目录；可以关闭 App 窗口。"
