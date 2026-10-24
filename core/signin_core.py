#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""稳定任务入口：CLI、launchd 调度、Web 面板共用同一套执行/记录/锁/状态逻辑。

职责：
- 结果分类：success / already_checked_in / login_expired / network_error /
  ssl_error / api_error / verification_failed / scheduler_error
- 每次执行写一条历史记录（SQLite，WAL，进程崩溃不损坏；失败也记录）
- 跨进程任务锁（SQLite BEGIN IMMEDIATE，进程崩溃自动释放）
- 风控/会话事件记录（token 刷新、自动登录、401/403/429、结果无法核验等）
- 状态快照（供 Web 仪表盘与健康检查），不返回任何密码/token

复用 yujia_signin.py 的全部接口实现，不重复实现签到流程。
"""

import datetime
import json
import os
import re
import subprocess
import sqlite3
import sys
import time

import yujia_signin as y

# ---------------------------------------------------------------- 状态常量

ST_SUCCESS = "success"
ST_ALREADY = "already_checked_in"
ST_LOGIN_EXPIRED = "login_expired"
ST_NETWORK = "network_error"
ST_SSL = "ssl_error"
ST_API = "api_error"
ST_UNVERIFIED = "verification_failed"
ST_SCHED = "scheduler_error"
ST_BUSY = "busy"

STATUS_TEXT = {
    ST_SUCCESS: "签到成功",
    ST_ALREADY: "今日已签到",
    ST_LOGIN_EXPIRED: "登录态失效",
    ST_NETWORK: "网络错误",
    ST_SSL: "证书错误",
    ST_API: "接口错误",
    ST_UNVERIFIED: "结果未核验",
    ST_SCHED: "调度错误",
    ST_BUSY: "任务运行中",
}
# 只有这些失败值得换时间重试；登录失效/限流等重试无意义
RETRYABLE_STATUSES = {ST_NETWORK, ST_SCHED}
OK_STATUSES = {ST_SUCCESS, ST_ALREADY}

SIGNIN_TIME_DEFAULT = "09:05"
SCHEDULER_RETRY_LIMIT = 6
SCHEDULER_RETRY_INTERVAL_MIN = 30
SCHEDULER_RETRY_SLOW_INTERVAL_MIN = 180


def _now():
    return datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")


def _today():
    return datetime.date.today().isoformat()


def _meta_get(key):
    try:
        conn = _connect()
        try:
            row = conn.execute("SELECT value FROM meta WHERE key=?", (key,)).fetchone()
            return row["value"] if row else None
        finally:
            conn.close()
    except (sqlite3.Error, OSError):
        return None


def _meta_set(key, value):
    try:
        conn = _connect()
        try:
            if value is None:
                conn.execute("DELETE FROM meta WHERE key=?", (key,))
            else:
                conn.execute("INSERT OR REPLACE INTO meta(key,value) VALUES(?,?)", (key, value))
            conn.commit()
        finally:
            conn.close()
        return True
    except (sqlite3.Error, OSError) as e:
        y.log("写入配置 %s 失败：%s" % (key, e))
        return False


def signin_time():
    """生效签到时刻。优先级：面板设置（meta schedule_time）> 环境变量 SIGNIN_TIME > 默认 09:05。
    面板修改后调度器每次触发都会重新读取，无需重启；重装 launchd（install_launchd.sh）
    会清掉面板覆盖，改用命令行指定的时间。"""
    for source in ("meta", "env"):
        raw = _meta_get("schedule_time") if source == "meta" \
            else os.environ.get("SIGNIN_TIME")
        if raw:
            try:
                h, m = (int(x) for x in raw.strip().split(":"))
                if 0 <= h < 24 and 0 <= m < 60:
                    return h, m
            except ValueError:
                pass
    return 9, 5


def schedule_source():
    if _meta_get("schedule_time"):
        return "panel"
    if os.environ.get("SIGNIN_TIME"):
        return "env"
    return "default"


def set_schedule(hhmm):
    """面板设置计划时间（HH:MM）。返回 (ok, message)。"""
    hhmm = (hhmm or "").strip()
    ok = re.fullmatch(r"([01]?\d|2[0-3]):([0-5]\d)", hhmm)
    if not ok:
        return False, "时间格式应为 HH:MM（24 小时制），例如 09:05"
    storage_ok, _ = init_storage()
    if not storage_ok:
        return False, "保存失败：记录数据库不可写"
    if not _meta_set("schedule_time", "%02d:%02d" % (int(ok.group(1)), int(ok.group(2)))):
        return False, "保存失败：记录数据库不可写"
    clear_scheduler_retry_state()
    return True, "已保存并生效；重试状态已重置"


def clear_schedule_override():
    storage_ok, _ = init_storage()
    if not storage_ok:
        return False
    ok = _meta_set("schedule_time", None)
    if ok:
        clear_scheduler_retry_state()
    return ok


def is_paused():
    return _meta_get("paused") == "1"


def _meta_set_many(values):
    """原子更新一组 meta 项，避免调度器重启时读到半套重试状态。"""
    try:
        conn = _connect()
        try:
            with conn:
                for key, value in values.items():
                    if value is None:
                        conn.execute("DELETE FROM meta WHERE key=?", (key,))
                    else:
                        conn.execute("INSERT OR REPLACE INTO meta(key,value) VALUES(?,?)",
                                     (key, str(value)))
        finally:
            conn.close()
        return True
    except (sqlite3.Error, OSError) as e:
        y.log("写入调度状态失败：%s" % e)
        return False


_RETRY_META_KEYS = (
    "scheduler_retry_date", "scheduler_retry_count", "scheduler_next_retry_at",
    "scheduler_retry_blocked", "scheduler_retry_reason",
)


def clear_scheduler_retry_state():
    """清除自动重试状态；用于改计划、恢复定时或人工执行成功后。"""
    return _meta_set_many({key: None for key in _RETRY_META_KEYS})


def scheduler_retry_status(now=None):
    """返回持久化的重试状态。进程重启/网络断开后仍保留下一次重试时间。"""
    now = now or datetime.datetime.now()
    today = now.date().isoformat()
    if _meta_get("scheduler_retry_date") != today:
        return {"date": today, "count": 0, "limit": SCHEDULER_RETRY_LIMIT,
                "next_retry_at": None, "blocked": False, "reason": "",
                "ready": True, "interval_minutes": SCHEDULER_RETRY_INTERVAL_MIN}
    try:
        count = max(0, int(_meta_get("scheduler_retry_count") or 0))
    except (TypeError, ValueError):
        count = 0
    blocked = _meta_get("scheduler_retry_blocked") == "1"
    next_raw = _meta_get("scheduler_next_retry_at")
    next_dt = None
    if next_raw:
        try:
            next_dt = datetime.datetime.fromisoformat(next_raw)
        except (TypeError, ValueError):
            next_dt = None
    interval = (SCHEDULER_RETRY_INTERVAL_MIN if count <= SCHEDULER_RETRY_LIMIT
                else SCHEDULER_RETRY_SLOW_INTERVAL_MIN)
    ready = not blocked and (next_dt is None or now >= next_dt)
    return {"date": today, "count": count, "limit": SCHEDULER_RETRY_LIMIT,
            "next_retry_at": next_raw, "blocked": blocked,
            "reason": _meta_get("scheduler_retry_reason") or "",
            "ready": ready, "interval_minutes": interval}


def record_scheduler_result(summary, now=None):
    """持久化一次定时结果：网络错误限速重试；非网络错误等待人工处理。

    前 6 次网络类失败每 30 分钟重试，之后每 3 小时探测一次，直到当天结束。
    不因进程重启或短暂断网而丢失重试状态。
    """
    if summary.get("busy") or summary.get("no_accounts"):
        return scheduler_retry_status(now)
    now = now or datetime.datetime.now()
    today = now.date().isoformat()
    old = scheduler_retry_status(now)
    if summary.get("all_ok"):
        clear_scheduler_retry_state()
        return scheduler_retry_status(now)

    count = (old["count"] if old["date"] == today else 0) + 1
    if summary.get("retryable"):
        interval = (SCHEDULER_RETRY_INTERVAL_MIN if count <= SCHEDULER_RETRY_LIMIT
                    else SCHEDULER_RETRY_SLOW_INTERVAL_MIN)
        next_at = (now + datetime.timedelta(minutes=interval)).replace(microsecond=0)
        values = {
            "scheduler_retry_date": today,
            "scheduler_retry_count": count,
            "scheduler_next_retry_at": next_at.isoformat(sep=" "),
            "scheduler_retry_blocked": None,
            "scheduler_retry_reason": "network",
        }
    else:
        values = {
            "scheduler_retry_date": today,
            "scheduler_retry_count": count,
            "scheduler_next_retry_at": None,
            "scheduler_retry_blocked": "1",
            "scheduler_retry_reason": "non_retryable",
        }
    _meta_set_many(values)
    return scheduler_retry_status(now)


def set_paused(paused):
    storage_ok, _ = init_storage()
    if not storage_ok:
        return False
    if not _meta_set("paused", "1" if paused else None):
        return False
    if not paused:
        clear_scheduler_retry_state()
    y.log("定时签到已%s" % ("暂停（手动执行不受影响）" if paused else "恢复"))
    return True


def daily_coins_series(days=14):
    """近 N 天每日金币合计（各账号当日最后一条成功/已签记录的 score_after 求和）。
    字段缺失的记录不计入；结果可能是"未知"而非真实值。"""
    try:
        conn = _connect()
        try:
            rows = conn.execute(
                "SELECT substr(started_at,1,10) AS d, account_name, score_after, MAX(id) "
                "FROM runs WHERE status IN (?,?) AND score_after IS NOT NULL "
                "GROUP BY substr(started_at,1,10), account_name",
                (ST_SUCCESS, ST_ALREADY)).fetchall()
        finally:
            conn.close()
    except (sqlite3.Error, OSError):
        return []
    daily = {}
    for r in rows:
        try:
            daily.setdefault(r["d"], 0.0)
            daily[r["d"]] += float(r["score_after"])
        except (TypeError, ValueError):
            continue
    out = []
    for i in range(days - 1, -1, -1):
        d = (datetime.date.today() - datetime.timedelta(days=i)).isoformat()
        if d in daily:
            out.append({"date": d, "coins": daily[d]})
    return out


def export_history_csv(limit=1000):
    """导出执行历史为 CSV（utf-8-sig，Excel 可直接打开）。不含任何凭据字段。"""
    rows = query_history(limit=limit)
    lines = ["时间,账号,来源,状态,错误类别,核验,金币(前),金币(后),连续天数,重试,耗时秒,信息"]
    for r in rows:
        cells = [r["finished_at"] or r["started_at"], r["account_name"], r["source"] or "",
                 r["status"], r["error_type"] or "",
                 "是" if r["verified"] else "否",
                 r["score_before"] or "", r["score_after"] or "", r["streak_days"] or "",
                 str(r["retry_count"] or "0"), str(r["duration_seconds"] or ""),
                 (r["message"] or "").replace('"', "'").replace("\n", " ")]
        line = ",".join('"%s"' % c for c in cells)
        lines.append(line)
    return "\r\n".join(lines)


# ---------------------------------------------------------------- 存储（SQLite）

def db_path():
    return os.path.join(y.CONFIG_DIR, "records.db")


def _connect(timeout=5.0):
    conn = sqlite3.connect(db_path(), timeout=timeout)
    conn.row_factory = sqlite3.Row
    return conn


_SCHEMA = """
CREATE TABLE IF NOT EXISTS runs(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    duration_seconds REAL,
    source TEXT,
    account_name TEXT NOT NULL,
    user_id_masked TEXT,
    status TEXT NOT NULL,
    error_type TEXT,
    message TEXT,
    score_before TEXT,
    score_after TEXT,
    streak_days TEXT,
    verified INTEGER DEFAULT 0,
    retry_count INTEGER DEFAULT 0,
    events TEXT
);
CREATE INDEX IF NOT EXISTS idx_runs_account ON runs(account_name, started_at);
CREATE INDEX IF NOT EXISTS idx_runs_status ON runs(status, started_at);
CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT);
"""


def init_storage():
    """启动时建表/迁移，并对旧版 config.json 做一次性升级备份。失败返回 (False, err)。"""
    try:
        os.makedirs(y.CONFIG_DIR, exist_ok=True)
        conn = _connect()
        try:
            conn.execute("PRAGMA journal_mode=WAL")
            conn.executescript(_SCHEMA)
            conn.commit()
        finally:
            conn.close()
        _backup_config_once()
        return True, None
    except (sqlite3.Error, OSError) as e:
        y.log("存储初始化失败（记录将只写入日志文件）：%s" % e)
        return False, str(e)


def _backup_config_once():
    """升级兼容：首次以新版本运行时备份旧 config.json，绝不覆盖历史数据。"""
    try:
        if not os.path.exists(y.CONFIG_PATH):
            return
        with open(y.CONFIG_PATH, "r", encoding="utf-8") as f:
            data = json.load(f)
        if data.get("v") == 2 or data.get("marker") == "backed-up":
            return
        stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
        bak = y.CONFIG_PATH + ".upgrade-backup-%s.json" % stamp
        with open(bak, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        data["marker"] = "backed-up"
        with open(y.CONFIG_PATH, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        y.log("已备份旧版账号配置到 %s" % bak)
    except (OSError, ValueError) as e:
        y.log("config.json 升级备份失败（不影响运行）：%s" % e)


def record_run(rec):
    """写一条执行记录；失败时打日志（调度器不因记录失败而无提示退出）。"""
    try:
        conn = _connect()
        try:
            conn.execute(
                "INSERT INTO runs(started_at,finished_at,duration_seconds,source,"
                "account_name,user_id_masked,status,error_type,message,score_before,"
                "score_after,streak_days,verified,retry_count,events) "
                "VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
                (rec["started_at"], rec["finished_at"], rec["duration_seconds"],
                 rec["source"], rec["account_name"], rec["user_id_masked"],
                 rec["status"], rec["error_type"], rec["message"],
                 rec["score_before"], rec["score_after"], rec["streak_days"],
                 int(rec.get("verified") or 0), int(rec.get("retry_count") or 0),
                 json.dumps(rec.get("events") or [], ensure_ascii=False)))
            conn.commit()
        finally:
            conn.close()
        return True, None
    except (sqlite3.Error, OSError, KeyError) as e:
        y.log("执行记录写入失败：%s（该次结果：%s）" % (e, rec.get("status")))
        return False, str(e)


def query_history(account=None, status=None, limit=20):
    sql = "SELECT * FROM runs"
    cond, args = [], []
    if account:
        cond.append("account_name=?")
        args.append(account)
    if status:
        cond.append("status=?")
        args.append(status)
    if cond:
        sql += " WHERE " + " AND ".join(cond)
    sql += " ORDER BY id DESC LIMIT ?"
    args.append(int(limit))
    try:
        conn = _connect()
        try:
            return [dict(r) for r in conn.execute(sql, args).fetchall()]
        finally:
            conn.close()
    except (sqlite3.Error, OSError) as e:
        y.log("读取执行记录失败：%s" % e)
        return []


def _counts(where="", args=()):
    try:
        conn = _connect()
        try:
            rows = conn.execute(
                "SELECT status, COUNT(*) c FROM runs %s GROUP BY status" % where,
                args).fetchall()
            return {r["status"]: r["c"] for r in rows}
        finally:
            conn.close()
    except (sqlite3.Error, OSError):
        return {}


# ---------------------------------------------------------------- 任务锁

class RunLock:
    """跨进程互斥：OS 级文件锁（Windows msvcrt.locking / POSIX flock）。
    进程崩溃时操作系统自动释放；锁文件残留无影响，重新可锁。
    不占用 SQLite 写事务，避免阻塞同进程/他进程的记录写入。"""

    def __init__(self):
        self.fh = None

    def _lock_path(self):
        return os.path.join(y.CONFIG_DIR, "run.lock")

    def acquire(self):
        try:
            if sys.platform == "win32":
                import msvcrt
            else:
                import fcntl
        except ImportError as e:
            return False, str(e)
        try:
            fh = open(self._lock_path(), "a+")
            fh.seek(0)  # 锁定区域从文件头开始，保证不同句柄锁同一字节
        except OSError as e:
            return False, str(e)
        try:
            if sys.platform == "win32":
                msvcrt.locking(fh.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                fcntl.flock(fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as e:
            fh.close()
            return False, "%s: %s" % (type(e).__name__, e)
        try:
            fh.seek(0)
            fh.truncate()
            fh.write(json.dumps({"pid": os.getpid(), "started_at": _now()}))
            fh.flush()
        except OSError:
            pass
        self.fh = fh
        return True, None

    def release(self):
        if self.fh is None:
            return
        try:
            if sys.platform == "win32":
                import msvcrt
                self.fh.seek(0)
                msvcrt.locking(self.fh.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                import fcntl
                fcntl.flock(self.fh.fileno(), fcntl.LOCK_UN)
        except OSError:
            pass
        try:
            self.fh.close()
        except OSError:
            pass
        self.fh = None


def lock_probe():
    """仅探测是否有任务在运行（立即返回，不占用锁）。"""
    lock = RunLock()
    ok, err = lock.acquire()
    if ok:
        lock.release()
        return False
    return True  # 拿不到锁 = 有任务在跑


def _set_run_state(value):
    try:
        conn = _connect()
        try:
            if value is None:
                conn.execute("DELETE FROM meta WHERE key='run_state'")
            else:
                conn.execute("INSERT OR REPLACE INTO meta(key,value) VALUES('run_state',?)",
                             (json.dumps(value, ensure_ascii=False),))
            conn.commit()
        finally:
            conn.close()
    except (sqlite3.Error, OSError):
        pass  # 状态辅助信息，失败不影响主流程


def read_run_state():
    try:
        conn = _connect()
        try:
            row = conn.execute("SELECT value FROM meta WHERE key='run_state'").fetchone()
            return json.loads(row["value"]) if row else None
        finally:
            conn.close()
    except (sqlite3.Error, OSError, ValueError):
        return None


# ---------------------------------------------------------------- 执行与分类

def mask_id(uid):
    uid = str(uid or "")
    if len(uid) <= 4:
        return uid[:1] + "****"
    return uid[:2] + "*" * max(2, len(uid) - 4) + uid[-2:]


def _classify_http(code, resp):
    """AddDetail 响应 → (status, error_type, message)。
    非 ok 响应若消息未明确"已签到"，返回 NEEDS_LEDGER，由调用方用流水核验判定。"""
    if not isinstance(resp, dict) or "Code" not in resp:
        return ST_UNVERIFIED, "no_code_in_response", "响应无法确认最终状态：%s" % y.sanitize_text(str(resp))[:120]
    code_str = str(resp.get("Code", ""))
    msg = y.sanitize_text(str(resp.get("Message", "") or code_str))[:200]
    if code_str == "ok":
        return ST_SUCCESS, None, "服务端确认本次签到成功"
    if ("已签到" in msg) or ("明日" in msg):
        return ST_ALREADY, None, "服务器返回今日已签到（无法确定执行来源）：" + msg
    return "NEEDS_LEDGER", "api_business", "服务端业务错误：" + msg


def _check_ledger_today(token, events):
    """用 v1/SignPoints 流水（字段 Time/Explain/Count，来源 App 编译模板）核对今天是否有记录。"""
    try:
        code, resp = y.http_json("GET", "v1/SignPoints?oldtime=m3&page=1&size=15", token=token)
    except (y.NetworkError, y.SSLCheckError, RuntimeError) as e:
        events.append({"type": "verification_unavailable", "detail": y.sanitize_text(str(e))[:120]})
        return None
    items = resp.get("Data") if isinstance(resp, dict) else None
    if code != 200 or not isinstance(items, list):
        events.append({"type": "verification_unavailable", "detail": "SignPoints 响应异常"})
        return None
    today = _today()
    for it in items:
        if isinstance(it, dict) and today in str(it.get("Time", "")):
            return True
    return False


def _attempt_sign(acc, accounts, events, retry_box):
    """发起签到；401 时按既有机制恢复一次（RefreshToken2 → AutoUserLogin2），不再更多。"""
    token = acc["secrets"].get("AccessToken", "")
    code, resp = y.sign_in(token)
    if code != 401:
        return code, resp
    events.append({"type": "http_401", "detail": "登录态失效，尝试恢复（刷新→自动登录，各一次）"})
    ok, r1 = y.refresh_token(acc)
    if ok:
        events.append({"type": "token_refreshed", "detail": "AccessToken 已更换（登录状态变化）"})
        y.save_accounts(accounts)
    else:
        events.append({"type": "token_refresh_failed",
                       "detail": y.sanitize_text(json.dumps(r1, ensure_ascii=False))[:120]})
        ok, r2 = y.auto_login(acc)
        if ok:
            events.append({"type": "auto_login_used", "detail": "AutoUserLogin2 兜底成功（登录状态变化）"})
            y.save_accounts(accounts)
        else:
            events.append({"type": "auto_login_failed",
                           "detail": y.sanitize_text(json.dumps(r2, ensure_ascii=False))[:160]})
            return None  # 无法恢复，不再重试
    retry_box[0] += 1
    return y.sign_in(acc["secrets"].get("AccessToken", ""))


def run_account(acc, accounts, source):
    """对单个账号执行一次签到并返回记录 dict。串行调用，内部不做跨账号循环。"""
    started_dt = datetime.datetime.now()
    rec = {
        "started_at": started_dt.strftime("%Y-%m-%d %H:%M:%S"),
        "finished_at": None, "duration_seconds": None,
        "source": source, "account_name": acc["name"],
        "user_id_masked": mask_id(acc["public"].get("UserID", acc["name"])),
        "status": None, "error_type": None, "message": None,
        "score_before": None, "score_after": None, "streak_days": None,
        "verified": 0, "retry_count": 0, "events": [],
    }
    events = rec["events"]
    token = acc["secrets"].get("AccessToken", "")

    y.log("账号 [%s]（%s / 工号 %s）" % (acc["name"], acc["public"].get("UserName", "?"),
                                       acc["public"].get("EmpNo", "?")))
    # 签到前金币（尽力而为；失败不阻塞）
    try:
        tp = y.total_points(token) if token else None
    except (y.NetworkError, y.SSLCheckError, RuntimeError):
        tp = None
    if tp:
        rec["score_before"] = str(tp.get("TOTAL", "")) or None
        rec["streak_days"] = str(tp.get("MCOUNT", "")) or None

    retry_box = [0]
    try:
        result = _attempt_sign(acc, accounts, events, retry_box)
    except y.SSLCheckError as e:
        result = ("ssl", str(e))
    except y.NetworkError as e:
        result = ("net", str(e))

    rec["retry_count"] = retry_box[0]

    if result is None:
        rec["status"], rec["error_type"] = ST_LOGIN_EXPIRED, "auth_recover_failed"
        rec["message"] = "登录态失效且自动恢复未成功，请对该账号重新 login（不再自动重试）"
    elif isinstance(result, tuple) and result[0] == "ssl":
        rec["status"], rec["error_type"] = ST_SSL, "ssl_cert_verify"
        rec["message"] = y.sanitize_text(str(result[1])).split("\n")[0][:200]
    elif isinstance(result, tuple) and result[0] == "net":
        rec["status"], rec["error_type"] = ST_NETWORK, "network_unreachable"
        rec["message"] = y.sanitize_text(str(result[1]))[:200]
    else:
        code, resp = result
        if code in (403, 429):
            rec["status"], rec["error_type"] = ST_API, ("http_%d" % code)
            rec["message"] = "服务端返回 HTTP %d（限流或拒绝），已停止本次重试" % code
            events.append({"type": "http_%d" % code, "detail": "疑似限流/拒绝，按服务端要求停止重试"})
        elif code == 401:
            rec["status"], rec["error_type"] = ST_LOGIN_EXPIRED, "http_401_after_recover"
            rec["message"] = "恢复后再次收到 401，登录态仍失效"
        else:
            status, etype, msg = _classify_http(code, resp)
            rec["error_type"] = etype
            if status == ST_SUCCESS:
                rec["status"], rec["message"], rec["verified"] = ST_SUCCESS, msg, 1
                ledger = _check_ledger_today(acc["secrets"].get("AccessToken", ""), events)
                if ledger is False:
                    events.append({"type": "ledger_miss",
                                   "detail": "加签成功但流水中暂未发现今日记录（可能延迟）"})
            elif status == ST_ALREADY:
                rec["status"], rec["message"] = ST_ALREADY, msg
                ledger = _check_ledger_today(acc["secrets"].get("AccessToken", ""), events)
                if ledger is True:
                    rec["verified"] = 1
                    rec["message"] = "服务器确认今日已签到（流水中有今日记录；无法确定执行来源）"
                elif ledger is None:
                    rec["message"] = "服务器返回今日已签到（流水核验不可用；无法确定执行来源）"
            elif status == ST_UNVERIFIED:
                rec["status"], rec["message"] = ST_UNVERIFIED, msg
            else:
                # 非 ok 且消息未说明原因：用流水核验今天是否其实已签
                ledger = _check_ledger_today(acc["secrets"].get("AccessToken", ""), events)
                if ledger is True:
                    rec["status"] = ST_ALREADY
                    rec["message"] = ("服务端返回非成功响应，但流水中已有今日记录 → 今日已签到"
                                      "（无法确定执行来源）；服务端信息：" + msg)
                elif ledger is False:
                    rec["status"] = ST_API
                    rec["message"] = msg + "（流水中无今日记录，确认为真实业务错误）"
                else:
                    rec["status"] = ST_API
                    rec["message"] = msg + "（流水核验不可用，结果分类为接口错误）"

    # 签到后金币（成功/已签时刷新）
    if rec["status"] in OK_STATUSES:
        try:
            tp2 = y.total_points(acc["secrets"].get("AccessToken", ""))
        except (y.NetworkError, y.SSLCheckError, RuntimeError):
            tp2 = None
        if tp2:
            rec["score_after"] = str(tp2.get("TOTAL", "")) or None
            rec["streak_days"] = str(tp2.get("MCOUNT", "")) or None
        y.log("  %s：%s" % (STATUS_TEXT[rec["status"]], rec["message"] or ""))
        if rec["score_after"]:
            y.log("  金币总数：%s，连续签到：%s 天" % (rec["score_after"], rec["streak_days"]))
    else:
        y.log("  失败[%s]：%s" % (STATUS_TEXT.get(rec["status"], rec["status"]),
                                 rec["message"] or ""))

    finished_dt = datetime.datetime.now()
    rec["finished_at"] = finished_dt.strftime("%Y-%m-%d %H:%M:%S")
    rec["duration_seconds"] = round((finished_dt - started_dt).total_seconds(), 2)
    rec["events"] = events
    return rec


def run_accounts(names=None, source="manual"):
    """稳定任务入口：串行执行（指定或全部启用的）账号，全部写历史记录。"""
    started = _now()
    t0 = time.time()
    storage_ok, storage_err = init_storage()
    summary = {"started_at": started, "finished_at": None, "source": source,
               "results": [], "counts": {}, "all_ok": False, "retryable": False,
               "busy": False, "no_accounts": False, "storage_error": storage_err}

    accounts = y.load_accounts()
    if not accounts:
        y.log("尚未登录任何账号：请先执行  python yujia_signin.py login")
        summary["no_accounts"] = True
        summary["finished_at"] = _now()
        return summary
    if names:
        by_name = {a["name"]: a for a in accounts}
        missing = [n for n in names if n not in by_name]
        if missing:
            y.log("未知账号：%s（用 list 查看已有账号）" % "、".join(missing))
            summary["finished_at"] = _now()
            return summary
        targets = [by_name[n] for n in names]
    else:
        targets = [a for a in accounts if a.get("enabled", True)]
    if not targets:
        y.log("没有启用中的账号（用 enable 恢复或 login 添加）")
        summary["no_accounts"] = True
        summary["finished_at"] = _now()
        return summary

    lock = RunLock()
    got, lock_err = lock.acquire()
    if not got:
        y.log("已有签到任务在运行（%s），本次不重复执行" % y.sanitize_text(str(lock_err))[:60])
        summary["busy"] = True
        summary["finished_at"] = _now()
        return summary

    y.log("开始签到（来源=%s，共 %d 个账号）" % (source, len(targets)))
    _set_run_state({"running": True, "pid": os.getpid(), "source": source,
                    "started_at": started, "current_account": None, "heartbeat": time.time()})
    try:
        for acc in targets:
            _set_run_state({"running": True, "pid": os.getpid(), "source": source,
                            "started_at": started, "current_account": acc["name"],
                            "heartbeat": time.time()})
            rec = run_account(acc, accounts, source)
            record_run(rec)
            summary["results"].append(rec)
        _set_run_state(None)
    finally:
        lock.release()

    counts = {}
    for rec in summary["results"]:
        counts[rec["status"]] = counts.get(rec["status"], 0) + 1
    summary["counts"] = counts
    summary["all_ok"] = all(rec["status"] in OK_STATUSES for rec in summary["results"])
    summary["retryable"] = any(rec["status"] in RETRYABLE_STATUSES
                               for rec in summary["results"])
    text = "，".join("%s=%s" % (rec["account_name"], STATUS_TEXT.get(rec["status"],
                    rec["status"])) for rec in summary["results"])
    y.log("签到汇总：%s" % text)
    summary["finished_at"] = _now()
    summary["elapsed_seconds"] = round(time.time() - t0, 2)
    if source != "scheduled" and summary["all_ok"]:
        clear_scheduler_retry_state()
    return summary


def summary_exit_code(summary):
    if summary.get("busy"):
        return 0  # 已有任务在跑，不算失败（避免调度器重试风暴）
    if summary.get("no_accounts"):
        return 2
    return 0 if summary["all_ok"] else 1


# ---------------------------------------------------------------- 当天待签与补跑

def signed_today_accounts():
    """今天已有 success/already 记录的账号集合（任何来源都算数，避免重复执行）。"""
    try:
        conn = _connect()
        try:
            rows = conn.execute(
                "SELECT DISTINCT account_name FROM runs "
                "WHERE substr(started_at,1,10)=? AND status IN (?,?)",
                (_today(), ST_SUCCESS, ST_ALREADY)).fetchall()
            return {r["account_name"] for r in rows}
        finally:
            conn.close()
    except (sqlite3.Error, OSError):
        return set()


def pending_accounts():
    """当前应签而未签的启用账号（包含需要人工处理的失败账号）。"""
    accounts = [a for a in y.load_accounts() if a.get("enabled", True)]
    done = signed_today_accounts()
    return [a for a in accounts if a["name"] not in done]


def scheduler_pending_accounts():
    """自动调度可安全重试的账号；登录失效/限流/业务错误等待人工处理。

    若多账号中只有一个遇到网络错误，只重试该账号，避免反复请求其他已失败且
    不可自动恢复的账号。
    """
    pending = pending_accounts()
    if not pending:
        return []
    today = _today()
    try:
        conn = _connect()
        try:
            rows = conn.execute(
                "SELECT r.* FROM runs r JOIN (SELECT account_name, MAX(id) mid FROM runs "
                "WHERE substr(started_at,1,10)=? GROUP BY account_name) t ON r.id=t.mid",
                (today,)).fetchall()
            latest_today = {r["account_name"]: dict(r) for r in rows}
        finally:
            conn.close()
    except (sqlite3.Error, OSError):
        # 若历史状态不可读，保持保守：不自动重试未知状态。
        return []
    eligible = []
    for account in pending:
        last = latest_today.get(account["name"])
        if last is None or last.get("status") in RETRYABLE_STATUSES:
            eligible.append(account)
    return eligible


def set_account_enabled(name, enabled):
    accounts = y.load_accounts()
    for acc in accounts:
        if acc["name"] == name:
            acc["enabled"] = bool(enabled)
            y.save_accounts(accounts)
            y.log("账号 [%s] 已%s（数据保留）" % (name, "启用" if enabled else "停用"))
            return 0
    y.log("未知账号：%s" % name)
    return 2


def delete_account(name):
    """从配置中删除账号（登录凭证一并删除）；历史记录默认保留。"""
    accounts = y.load_accounts()
    remain = [a for a in accounts if a["name"] != name]
    if len(remain) == len(accounts):
        y.log("未知账号：%s" % name)
        return 2
    y.save_accounts(remain)
    y.log("账号 [%s] 已从配置删除；历史签到记录仍保留在 records.db" % name)
    return 0


# ---------------------------------------------------------------- 状态快照（健康）

def _launchd_state():
    if sys.platform != "darwin":
        return {"mode": "docker-or-other" if os.environ.get("YUJIA_CONFIG_DIR") else "unknown",
                "loaded": None}
    plist = os.path.expanduser("~/Library/LaunchAgents/com.yujia.signin.plist")
    loaded = None
    if os.path.exists(plist):
        try:
            r = subprocess.run(["launchctl", "list", "com.yujia.signin"],
                               capture_output=True, timeout=5)
            loaded = (r.returncode == 0)
        except (OSError, subprocess.SubprocessError):
            loaded = None
        return {"mode": "launchd", "loaded": loaded}
    return {"mode": "unknown", "loaded": None}


def _web_running():
    port = os.environ.get("YUJIA_WEB_PORT", "8765")
    try:
        import base64
        import urllib.error
        import urllib.request
        headers = {}
        pwd = os.environ.get("YUJIA_WEB_PASSWORD")
        if pwd:
            token = base64.b64encode(
                ("%s:%s" % (os.environ.get("YUJIA_WEB_USER", "panel"), pwd)).encode("utf-8")
            ).decode("ascii")
            headers["Authorization"] = "Basic " + token
        req = urllib.request.Request("http://127.0.0.1:%s/api/health" % port,
                                     headers=headers)
        with urllib.request.urlopen(req, timeout=1.5) as r:
            return r.status == 200
    except urllib.error.HTTPError as e:
        if e.code == 401:
            return True  # 有响应只是要求口令 → 面板在运行
        return False
    except Exception:
        return False


def next_due_text():
    if is_paused():
        return "已暂停"
    h, m = signin_time()
    now = datetime.datetime.now()
    due = now.replace(hour=h, minute=m, second=0, microsecond=0)
    if due <= now:
        pending = pending_accounts()
        eligible = scheduler_pending_accounts() if pending else []
        if pending and not is_paused():
            if not eligible:
                return "待人工处理"
            retry = scheduler_retry_status(now)
            if retry["blocked"]:
                return "今日自动重试已暂停"
            if retry["next_retry_at"] and not retry["ready"]:
                try:
                    return datetime.datetime.fromisoformat(
                        retry["next_retry_at"]).strftime("%Y-%m-%d %H:%M")
                except ValueError:
                    pass
            return due.strftime("%Y-%m-%d %H:%M") + "（待补跑）"
        due += datetime.timedelta(days=1)
    return due.strftime("%Y-%m-%d %H:%M")


def _latest_per_account():
    try:
        conn = _connect()
        try:
            rows = conn.execute(
                "SELECT r.* FROM runs r JOIN (SELECT account_name, MAX(id) mid FROM runs "
                "GROUP BY account_name) t ON r.id=t.mid").fetchall()
            return {r["account_name"]: dict(r) for r in rows}
        finally:
            conn.close()
    except (sqlite3.Error, OSError):
        return {}


def _today_status_per_account():
    try:
        conn = _connect()
        try:
            rows = conn.execute(
                "SELECT r.* FROM runs r JOIN (SELECT account_name, MAX(id) mid FROM runs "
                "WHERE substr(started_at,1,10)=? GROUP BY account_name) t ON r.id=t.mid",
                (_today(),)).fetchall()
            return {r["account_name"]: dict(r) for r in rows}
        finally:
            conn.close()
    except (sqlite3.Error, OSError):
        return {}


def state_snapshot(include_token=False):
    """供 Web 仪表盘 / CLI 健康检查。绝不含密码/token。"""
    storage_ok, _ = init_storage()
    accounts = y.load_accounts()
    latest = _latest_per_account()
    today = _today_status_per_account()
    done_today = signed_today_accounts()

    acc_view = []
    for a in accounts:
        last = latest.get(a["name"])
        tday = today.get(a["name"])
        acc_view.append({
            "name": a["name"],
            "user_id_masked": mask_id(a["public"].get("UserID", a["name"])),
            "emp_no": a["public"].get("EmpNo", ""),
            "area_id": a["public"].get("AreaID", ""),
            "note": a.get("note", ""),
            "enabled": bool(a.get("enabled", True)),
            "coins": (last or {}).get("score_after"),
            "streak_days": (last or {}).get("streak_days"),
            "last_run_at": (last or {}).get("finished_at") or (last or {}).get("started_at"),
            "last_status": (last or {}).get("status"),
            "last_message": (last or {}).get("message"),
            "today_status": (tday or {}).get("status"),
            "today_verified": bool((tday or {}).get("verified")),
            "signed_today": a["name"] in done_today,
        })

    recent = query_history(limit=20)
    counts_total = _counts()
    counts_today = _counts("WHERE substr(started_at,1,10)='%s'" % _today())

    run_state = read_run_state()
    if run_state and run_state.get("running") and \
            time.time() - float(run_state.get("heartbeat") or 0) > 900:
        run_state["stale"] = True  # 心跳超过 15 分钟 → 疑似中断（锁已自动释放，可重新执行）

    ld = _launchd_state()
    warnings = []
    paused = is_paused()
    if paused:
        warnings.append("定时签到已暂停（手动执行不受影响）")
    pending = pending_accounts()
    schedulable = scheduler_pending_accounts() if pending else []
    retry_state = scheduler_retry_status()
    h, m = signin_time()
    due_now = (datetime.datetime.now().hour, datetime.datetime.now().minute) >= (h, m)
    if not storage_ok:
        warnings.append("记录数据库不可写，结果只存在于日志中")
    if os.environ.get("YUJIA_SSL_INSECURE") == "1":
        warnings.append("YUJIA_SSL_INSECURE=1：已跳过 HTTPS 证书校验（醒目警告：仅限可信网络临时排错）")
    import web_panel as _wp  # 局部导入避免循环
    lan = _wp.lan_mode()
    if lan:
        warnings.append("面板处于局域网访问模式（已启用密码保护），请确保口令足够强")
    expired = [a["name"] for a in acc_view if a["last_status"] == ST_LOGIN_EXPIRED]
    if expired:
        warnings.append("登录态失效待人工处理：%s" % "、".join(expired))
    if ld["mode"] == "launchd" and ld["loaded"] is False:
        warnings.append("LaunchAgent 未加载（launchctl list 无该任务）")

    if pending and due_now and not paused:
        if schedulable:
            warnings.append("已过签到时刻，尚有 %d 个账号未签且可自动处理：%s"
                            % (len(schedulable), "、".join(a["name"] for a in schedulable)))
        else:
            blocked_names = [a["name"] for a in pending]
            warnings.append("以下账号今日出现不可自动重试的错误，需要人工检查：%s"
                            % "、".join(blocked_names))
    if retry_state["blocked"]:
        warnings.append("自动签到因不可重试错误暂停：请检查账号状态/接口错误；手动执行成功或次日会恢复")
    elif retry_state["next_retry_at"] and not retry_state["ready"]:
        warnings.append("最近一次自动签到失败；下次自动重试：%s" % retry_state["next_retry_at"])

    return {
        "now": _now(),
        "timezone": time.tzname[0] if time.tzname else "",
        "storage_ok": storage_ok,
        "accounts": acc_view,
        "recent_runs": recent,
        "counts_total": counts_total,
        "counts_today": counts_today,
        "run_state": run_state,
        "scheduler": {
            "mode": ld["mode"], "launchd_loaded": ld["loaded"],
            "signin_time": "%02d:%02d" % (h, m),
            "schedule_source": schedule_source(),
            "paused": paused,
            "due_now": due_now,
            "pending_accounts": [a["name"] for a in pending],
            "schedulable_accounts": [a["name"] for a in schedulable],
            "retry_count": retry_state["count"],
            "retry_limit": retry_state["limit"],
            "next_retry_at": retry_state["next_retry_at"],
            "retry_blocked": retry_state["blocked"],
            "retry_reason": retry_state["reason"],
            "retry_interval_minutes": retry_state["interval_minutes"],
            "next_due": next_due_text(),
        },
        "coins_series": daily_coins_series(),
        "web_running": _web_running(),
        "web_lan_mode": lan,
        "warnings": warnings,
        "status_text": STATUS_TEXT,
        "panel_token": _panel_token() if include_token else None,
    }


def _panel_token():
    """Web 面板防 CSRF 令牌：同源页面可读，跨站脚本读不到（SOP）。"""
    path = os.path.join(y.CONFIG_DIR, "panel_token")
    try:
        with open(path, "r", encoding="utf-8") as f:
            tok = f.read().strip()
        if tok:
            return tok
    except OSError:
        pass
    import uuid as _uuid
    tok = _uuid.uuid4().hex
    try:
        os.makedirs(y.CONFIG_DIR, exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write(tok)
        os.chmod(path, 0o600)
    except OSError:
        pass
    return tok
