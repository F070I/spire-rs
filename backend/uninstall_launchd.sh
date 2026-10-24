#!/bin/bash
# Stop launchd jobs, never delete user data.
set -euo pipefail
AGENTS="$HOME/Library/LaunchAgents"
UID_NUM="$(id -u)"
for label in com.yujia.signin com.yujia.signinweb; do
    /bin/launchctl bootout "gui/$UID_NUM/$label" >/dev/null 2>&1 || true
    /bin/launchctl unload "$AGENTS/$label.plist" >/dev/null 2>&1 || true
    /bin/rm -f "$AGENTS/$label.plist"
done
echo "后台已停止，用户数据和历史仍保留在 ~/Library/Application Support/yujia-signin。"
