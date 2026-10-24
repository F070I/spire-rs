#!/bin/bash
# Explicit user-selected migration: backup old destination first, don't destroy source.
set -euo pipefail
if [ "$#" -ne 1 ] || [ ! -d "$1" ]; then
    echo "没有选择有效的旧项目目录" >&2; exit 2
fi
SOURCE="$(cd "$1" && pwd)"
TARGET="$HOME/Library/Application Support/yujia-signin"
BACKEND="$(cd "$(dirname "$0")" && pwd)"
if [ "$SOURCE" = "$TARGET" ]; then echo '数据已在标准应用目录，无需迁移'; exit 0; fi
if [ ! -f "$SOURCE/config.json" ] && [ ! -f "$SOURCE/records.db" ]; then
    echo '旧目录缺少 config.json 或 records.db，未操作任何数据' >&2; exit 2
fi
# Stop only a legacy no-hup scheduler positively identified as coming from the chosen folder.
if [ -f "$SOURCE/daily.pid" ]; then
    OLD_PID="$(tr -cd '0-9' < "$SOURCE/daily.pid" || true)"
    if [ -n "$OLD_PID" ] && /bin/kill -0 "$OLD_PID" 2>/dev/null; then
        OLD_CMD="$(/bin/ps -p "$OLD_PID" -o command= 2>/dev/null || true)"
        OLD_CWD="$(/usr/sbin/lsof -a -p "$OLD_PID" -d cwd -Fn 2>/dev/null | /usr/bin/sed -n 's/^n//p' | /usr/bin/head -n 1)"
        if [[ "$OLD_CMD" == *run_daily.py* || "$OLD_CMD" == *yujia_signin.py* ]]; then
            if [[ "$OLD_CMD" == *"$SOURCE"* || "$OLD_CWD" == "$SOURCE" ]]; then
                /bin/kill -TERM "$OLD_PID" 2>/dev/null || true
            fi
        fi
    fi
fi
"$BACKEND/uninstall_launchd.sh"
mkdir -p "$TARGET"
chmod 700 "$TARGET"
BACKUP="$TARGET/backup-before-import-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP"
chmod 700 "$BACKUP"
for name in config.json records.db; do
    # Only copy ordinary files, not symlinks that may point elsewhere.
    if [ -f "$SOURCE/$name" ] && [ ! -L "$SOURCE/$name" ]; then
        if [ -e "$TARGET/$name" ]; then /bin/cp -p "$TARGET/$name" "$BACKUP/$name"; fi
        /bin/cp -p "$SOURCE/$name" "$TARGET/$name.importing"
        /bin/chmod 600 "$TARGET/$name.importing"
        /bin/mv -f "$TARGET/$name.importing" "$TARGET/$name"
    fi
done
# Database WAL can hold newer transactions; back up and migrate with SQLite to avoid truncation.
# If source records.db-wal exists, preserve alongside database before restart.
for name in records.db-wal records.db-shm; do
    if [ -f "$SOURCE/$name" ] && [ ! -L "$SOURCE/$name" ]; then
        [ ! -f "$TARGET/$name" ] || /bin/cp -p "$TARGET/$name" "$BACKUP/$name"
        /bin/cp -p "$SOURCE/$name" "$TARGET/$name"
        /bin/chmod 600 "$TARGET/$name"
    fi
done
# Panel token is a per-install local anti-CSRF secret; fresh one is generated automatically.
/bin/rm -f "$TARGET/panel_token"
echo "已导入账号与历史；原文件保留不动，新目录旧数据已单独备份。"
