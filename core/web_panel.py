#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""本地 Web 管理面板（仅监听 127.0.0.1，默认 http://127.0.0.1:8765）。

安全设计：
- 只绑定 127.0.0.1；Host 头校验（防 DNS rebinding）
- 写操作（POST）必须携带 X-Panel-Token（面板启动令牌，同源页面可读，跨站脚本拿不到）
- 不提供任何文件读取、命令执行接口；launchctl 探测只读且仅 macOS
- 页面/日志/接口不返回密码、token、Cookie；账号标识脱敏
- 手动执行与定时任务共用 signin_core 的同一把锁，不会并发重复签到
"""

import base64
import hmac
import json
import os
import re
import subprocess
import sys
import threading
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import yujia_signin as y
import signin_core as core

MAX_BODY = 64 * 1024
LOOPBACK = ("127.0.0.1", "localhost", "::1")


def web_password():
    return os.environ.get("YUJIA_WEB_PASSWORD") or ""


def lan_mode():
    """是否开启局域网访问（绑定非回环地址）。"""
    return (os.environ.get("YUJIA_WEB_BIND") or "127.0.0.1") not in LOOPBACK


def _tail_log(lines=200):
    lines = max(10, min(500, int(lines)))
    try:
        with open(y.LOG_PATH, "r", encoding="utf-8", errors="replace") as f:
            return y.sanitize_text("".join(f.readlines()[-lines:]))
    except OSError as e:
        return "（无法读取日志：%s）" % e


class Handler(BaseHTTPRequestHandler):
    server_version = "YujiaPanel/2.0"

    # ---------- 基础 ----------
    def _json(self, obj, status=200):
        body = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.end_headers()
        self.wfile.write(body)

    def _html(self, text):
        body = text.encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.end_headers()
        self.wfile.write(body)

    def _host_ok(self):
        host = (self.headers.get("Host") or "").split(":")[0]
        if host in LOOPBACK:
            return True
        # 局域网模式已有口令保护（防 DNS rebinding 的作用由口令接管）
        return lan_mode() and bool(web_password())

    def _auth_ok(self):
        """局域网模式必须通过 Basic Auth；本机模式不要求。"""
        pwd = web_password()
        if not pwd:
            return True
        header = self.headers.get("Authorization") or ""
        if not header.startswith("Basic "):
            return False
        try:
            userpass = base64.b64decode(header[6:].strip()).decode("utf-8")
        except (ValueError, UnicodeDecodeError):
            return False
        user, _, passwd = userpass.partition(":")
        user_ok = hmac.compare_digest(user.encode("utf-8"),
                                      os.environ.get("YUJIA_WEB_USER", "panel").encode("utf-8"))
        pass_ok = hmac.compare_digest(passwd.encode("utf-8"), pwd.encode("utf-8"))
        return user_ok and pass_ok

    def _deny_auth(self):
        body = json.dumps({"error": "需要登录（局域网访问模式）"}, ensure_ascii=False).encode("utf-8")
        self.send_response(401)
        self.send_header("WWW-Authenticate", 'Basic realm="yujia-panel"')
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _token_ok(self):
        return (self.headers.get("X-Panel-Token") or "") == core._panel_token()

    def _body(self):
        n = int(self.headers.get("Content-Length") or 0)
        if n <= 0 or n > MAX_BODY:
            return None
        try:
            return json.loads(self.rfile.read(n).decode("utf-8"))
        except (ValueError, UnicodeDecodeError):
            return None

    def log_message(self, fmt, *args):
        pass  # 不写访问日志，避免噪音与敏感信息

    # ---------- 路由 ----------
    def do_GET(self):
        if not self._auth_ok():
            return self._deny_auth()
        if not self._host_ok():
            return self._json({"error": "forbidden host"}, 403)
        parsed = urllib.parse.urlparse(self.path)
        path, qs = parsed.path, urllib.parse.parse_qs(parsed.query)
        if path == "/" or path == "/index.html":
            return self._html(PAGE.replace("__PORT__", self.server.server_address[1].__str__()))
        if path == "/api/health":
            return self._json({"ok": True, "service": "yujia-panel"})
        if path == "/api/state":
            return self._json(core.state_snapshot(include_token=True))
        if path == "/api/history":
            limit = int((qs.get("limit") or ["20"])[0])
            rows = core.query_history(account=(qs.get("account") or [None])[0],
                                      status=(qs.get("status") or [None])[0],
                                      limit=max(1, min(200, limit)))
            return self._json({"rows": rows})
        if path == "/api/logs":
            return self._json({"text": _tail_log(int((qs.get("lines") or ["200"])[0]))})
        if path == "/api/export":
            csv = core.export_history_csv()
            body = csv.encode("utf-8-sig")
            self.send_response(200)
            self.send_header("Content-Type", "text/csv; charset=utf-8")
            self.send_header("Content-Disposition",
                             "attachment; filename=yujia-history.csv")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(body)
            return
        if path == "/api/accounts":
            return self._json({"accounts": core.state_snapshot().get("accounts", [])})
        if path == "/api/areas":
            uid = (qs.get("user_id") or [""])[0].strip()
            if not uid:
                return self._json({"error": "user_id required"}, 400)
            try:
                return self._json({"areas": y.get_areas(uid)})
            except (y.NetworkError, y.SSLCheckError) as e:
                return self._json({"error": y.sanitize_text(str(e))[:200]}, 502)
        return self._json({"error": "not found"}, 404)

    def do_POST(self):
        if not self._auth_ok():
            return self._deny_auth()
        if not self._host_ok():
            return self._json({"error": "forbidden host"}, 403)
        if not self._token_ok():
            return self._json({"error": "invalid panel token（请刷新页面）"}, 403)
        path = urllib.parse.urlparse(self.path).path
        data = self._body()
        if data is None:
            return self._json({"error": "invalid body"}, 400)
        try:
            return self._dispatch_post(path, data)
        except (y.NetworkError, y.SSLCheckError) as e:
            return self._json({"error": y.sanitize_text(str(e))[:250]}, 502)

    def _dispatch_post(self, path, data):
        if path == "/api/run":
            name = (data.get("account") or "").strip() or None
            if name:
                names = [a["name"] for a in y.load_accounts()]
                if name not in names:
                    return self._json({"error": "未知账号"}, 400)
            if core.lock_probe():
                return self._json({"running": True,
                                   "detail": core.read_run_state()}, 409)
            source = "web"
            if name:
                threading.Thread(target=_run_names, args=([name], source),
                                 daemon=True).start()
            else:
                threading.Thread(target=_run_names, args=(None, source),
                                 daemon=True).start()
            return self._json({"started": True, "source": source})
        if path == "/api/verify":
            name = (data.get("name") or "").strip()
            acc = next((a for a in y.load_accounts() if a["name"] == name), None)
            if not acc:
                return self._json({"error": "未知账号"}, 400)
            try:
                tp = y.total_points(acc["secrets"].get("AccessToken", ""))
            except y.SSLCheckError as e:
                return self._json({"ok": False, "status": core.ST_SSL,
                                   "message": y.sanitize_text(str(e)).split("\n")[0][:150]})
            except y.NetworkError as e:
                return self._json({"ok": False, "status": core.ST_NETWORK,
                                   "message": y.sanitize_text(str(e))[:150]})
            if tp:
                return self._json({"ok": True, "status": "valid",
                                   "coins": tp.get("TOTAL"), "streak_days": tp.get("MCOUNT")})
            return self._json({"ok": False, "status": core.ST_LOGIN_EXPIRED,
                               "message": "无法读取金币（登录态可能失效），可尝试刷新 token 或重新 login"})
        if path == "/api/accounts/add":
            user_id = (data.get("user_id") or "").strip()
            password = data.get("password") or ""
            area_id = (data.get("area_id") or "").strip()
            name = (data.get("name") or "").strip() or user_id
            note = (data.get("note") or "").strip()
            if not user_id or not password or not area_id:
                return self._json({"error": "账号、密码、区域均必填"}, 400)
            accounts = y.load_accounts()
            if any(a["name"] == name for a in accounts):
                return self._json({"error": "账号名已存在，如需更新请先删除再加"}, 400)
            result, resp = y.do_login(user_id, password, area_id)
            if not result:
                msg = y.sanitize_text(json.dumps(resp, ensure_ascii=False))[:200]
                return self._json({"error": "登录失败：%s" % msg}, 401)
            public, secrets = result
            accounts.append({"name": name, "public": public, "secrets": secrets,
                             "note": note, "enabled": True})
            y.save_accounts(accounts)
            return self._json({"ok": True, "name": name,
                               "user": public.get("UserName", ""),
                               "emp_no": public.get("EmpNo", "")})
        if path == "/api/accounts/update":
            name = (data.get("name") or "").strip()
            accounts = y.load_accounts()
            acc = next((a for a in accounts if a["name"] == name), None)
            if not acc:
                return self._json({"error": "未知账号"}, 400)
            if "note" in data:
                acc["note"] = str(data.get("note") or "").strip()
            if "enabled" in data:
                acc["enabled"] = bool(data.get("enabled"))
            if data.get("area_id"):
                acc["public"]["AreaID"] = str(data.get("area_id")).strip()
            y.save_accounts(accounts)
            return self._json({"ok": True})
        if path == "/api/accounts/delete":
            name = (data.get("name") or "").strip()
            rc = core.delete_account(name)
            if rc == 0:
                return self._json({"ok": True, "note": "登录凭证已删除，历史签到记录保留"})
            return self._json({"error": "未知账号"}, 400)
        if path == "/api/refresh-token":
            name = (data.get("name") or "").strip()
            accounts = y.load_accounts()
            acc = next((a for a in accounts if a["name"] == name), None)
            if not acc:
                return self._json({"error": "未知账号"}, 400)
            ok, resp = y.refresh_token(acc)
            if ok:
                y.save_accounts(accounts)
                return self._json({"ok": True})
            return self._json({"ok": False,
                               "error": y.sanitize_text(json.dumps(resp, ensure_ascii=False))[:200]})
        if path == "/api/schedule":
            hhmm = str(data.get("time") or "").strip()
            ok, msg = core.set_schedule(hhmm)
            if not ok:
                return self._json({"error": msg}, 400)
            effective_time = "%02d:%02d" % core.signin_time()
            plist_synced = _update_plist_time(effective_time)
            return self._json({"ok": True, "message": msg, "plist_synced": plist_synced,
                               "signin_time": effective_time,
                               "next_due": core.next_due_text()})
        if path == "/api/paused":
            if not core.set_paused(bool(data.get("paused"))):
                return self._json({"error": "保存失败（记录数据库不可写）"}, 500)
            return self._json({"ok": True, "paused": core.is_paused()})
        return self._json({"error": "not found"}, 404)


def _run_names(names, source):
    try:
        core.run_accounts(names, source=source)
    except Exception as e:
        y.log("Web 手动执行异常：%s" % e)


def _update_plist_time(hhmm, platform=None):
    """launchd 模式下把新时刻同步进 plist 并重载每日 Agent（尽力而为）。
    即使失败，meta 里的面板设置仍是调度器读取的最终值。"""
    if (platform or sys.platform) != "darwin":
        return None
    plist = os.path.expanduser("~/Library/LaunchAgents/com.yujia.signin.plist")
    if not os.path.exists(plist):
        return None
    try:
        with open(plist, "r", encoding="utf-8") as f:
            text = f.read()
        pattern = r"(<key>\s*SIGNIN_TIME\s*</key>\s*<string>)[^<]*(</string>)"
        new, count = re.subn(pattern,
                             lambda m: m.group(1) + hhmm + m.group(2), text, count=1)
        if count == 0:
            return False
        if new == text:
            return True  # plist 已经是目标时间，不是同步失败
        tmp = plist + ".tmp"
        with open(tmp, "w", encoding="utf-8") as f:
            f.write(new)
        os.replace(tmp, plist)
        subprocess.run(["launchctl", "unload", plist], capture_output=True, timeout=10)
        subprocess.run(["launchctl", "load", plist], capture_output=True, timeout=10)
        y.log("已同步 launchd 计划时间为 %s 并重载 Agent" % hhmm)
        return True
    except (OSError, subprocess.SubprocessError) as e:
        y.log("同步 launchd 计划时间失败（面板设置仍然生效）：%s" % e)
        return None


# ---------------------------------------------------------------- 页面

PAGE = r"""<!DOCTYPE html>
<html lang="zh-CN"><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark">
<title>寓+ · 自动签到控制台</title>
<style>
:root{
  color-scheme:dark;--bg:#0a1020;--panel:#111b2e;--panel2:#152239;--line:#263750;
  --text:#e8effa;--muted:#91a3bc;--accent:#72d6ff;--accent2:#7c9cff;
  --ok:#5ee0a0;--warn:#ffc66d;--err:#ff7d91;--shadow:0 18px 48px #02061755;
}
*{box-sizing:border-box}
body{margin:0;min-height:100vh;background:radial-gradient(ellipse at 8% -12%,#17395a 0,transparent 35%),radial-gradient(ellipse at 100% 0%,#252451 0,transparent 30%),var(--bg);color:var(--text);font:14px/1.55 -apple-system,BlinkMacSystemFont,"PingFang SC","Microsoft YaHei",sans-serif}
button,input,select{font:inherit}
button{display:inline-flex;align-items:center;justify-content:center;gap:7px;padding:8px 12px;border:1px solid #3a4d69;border-radius:10px;background:#1a2940;color:var(--text);cursor:pointer;transition:background .15s,border-color .15s,transform .15s}
button:hover{background:#243955;border-color:#6caed3}button:active{transform:translateY(1px)}button:disabled{opacity:.48;cursor:not-allowed;transform:none}
button.primary{background:linear-gradient(135deg,#83e1ff,#8ba7ff);border-color:transparent;color:#071426;font-weight:750}
button.primary:hover{filter:brightness(1.07)}button.danger{color:var(--err);border-color:#754052;background:#2a1b2a}button.ghost{background:transparent}
input,select{width:100%;min-width:0;padding:9px 11px;border:1px solid #30425c;border-radius:10px;background:#0c1628;color:var(--text);outline:none}
input:focus,select:focus{border-color:var(--accent);box-shadow:0 0 0 3px #72d6ff1c}
label{display:block;margin:0 0 5px;color:var(--muted);font-size:12px}
a{color:inherit;text-decoration:none}
.wrap{width:min(1240px,100%);margin:0 auto;padding:26px 22px 40px}
.topbar{display:flex;align-items:center;justify-content:space-between;gap:16px;margin-bottom:24px}
.brand{display:flex;align-items:center;gap:13px}.brandmark{display:grid;place-items:center;width:44px;height:44px;border-radius:14px;background:linear-gradient(135deg,#7de2ff,#8a9eff);color:#071426;font-size:23px;font-weight:900;box-shadow:0 8px 28px #72d6ff25}
h1{margin:0;font-size:21px;letter-spacing:.2px;font-weight:760}.subtitle{margin-top:3px;color:var(--muted);font-size:12px}.topmeta{display:flex;align-items:center;gap:9px;flex-wrap:wrap;justify-content:flex-end}
.chip{display:inline-flex;align-items:center;gap:6px;padding:5px 9px;border:1px solid #33445e;border-radius:999px;color:#b7c7dc;background:#101a2c;font-size:11px}.dot{width:6px;height:6px;border-radius:50%;background:var(--ok);box-shadow:0 0 0 3px #5ee0a014}
.banner{display:flex;align-items:flex-start;gap:10px;padding:12px 14px;margin:9px 0;border:1px solid #855d2c;border-radius:12px;background:#392b1c;color:#ffe0a6;overflow-wrap:anywhere}.banner:before{content:"!";display:grid;place-items:center;flex:0 0 20px;height:20px;border:1px solid currentColor;border-radius:50%;font-size:12px;font-weight:800}.banner.blue{border-color:#355c91;background:#162e4d;color:#b9e7ff}.banner.blue:before{content:"↻"}
.section{margin-top:24px}.section-head{display:flex;align-items:center;justify-content:space-between;gap:12px;margin:0 0 11px}.section-head h2{margin:0;font-size:14px;font-weight:730;letter-spacing:.2px}.section-head .hint{color:var(--muted);font-size:12px}
.cards{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:12px}.card{position:relative;min-width:0;overflow:hidden;padding:15px 16px;border:1px solid #253650;border-radius:15px;background:linear-gradient(145deg,#142139ee,#10192aee);box-shadow:var(--shadow)}.card:after{content:"";position:absolute;right:-26px;top:-35px;width:95px;height:95px;border-radius:50%;background:#72d6ff0a;pointer-events:none}.card .k{color:var(--muted);font-size:12px}.card .v{position:relative;z-index:1;margin-top:8px;font-size:17px;font-weight:700;overflow-wrap:anywhere}.card .subv{margin-top:5px;color:var(--muted);font-size:11px}.card .v .pill{font-size:11px}
.pill{display:inline-flex;align-items:center;gap:4px;padding:3px 8px;border:1px solid #42516a;border-radius:999px;color:#c7d2e3;background:#1a2940;font-size:11px;white-space:nowrap}.pill.ok{border-color:#28694f;color:var(--ok);background:#12352b}.pill.blue{border-color:#315e87;color:#9ddcff;background:#152f4b}.pill.warn{border-color:#74572f;color:var(--warn);background:#382c1c}.pill.err{border-color:#754051;color:var(--err);background:#38202d}.pill.mut{color:#9bacc2;background:#172236}
.toolbar{display:flex;align-items:center;gap:9px;flex-wrap:wrap}.schedulebar{display:grid;grid-template-columns:minmax(150px,1fr) auto auto auto;align-items:end;gap:10px;padding:14px;border:1px solid #263750;border-radius:15px;background:#111c2eee}.schedulebar .timefield{max-width:220px}.inline-note{align-self:center;color:var(--muted);font-size:12px;min-width:120px;overflow-wrap:anywhere}
.panel{border:1px solid #253650;border-radius:15px;background:#101a2bee;overflow:hidden;box-shadow:var(--shadow)}.panel-pad{padding:15px}.table-wrap{overflow-x:auto;-webkit-overflow-scrolling:touch}table{width:100%;min-width:790px;border-collapse:separate;border-spacing:0}th,td{padding:11px 12px;border-bottom:1px solid #23334a;text-align:left;vertical-align:middle;font-size:12px;white-space:nowrap}th{position:sticky;top:0;z-index:1;background:#142139;color:#91a3bc;font-weight:650}tbody tr{background:#101a2b;transition:background .12s}tbody tr:hover{background:#17253b}tbody tr:last-child td{border-bottom:0}td.wrapcell{white-space:normal;min-width:170px;max-width:300px;overflow-wrap:anywhere}.account-name{font-weight:700}.small{color:var(--muted);font-size:11px}.muted{color:var(--muted)}.actions{display:flex;align-items:center;gap:5px;flex-wrap:wrap}.actions button{padding:5px 8px;border-radius:8px;font-size:11px}
.form-panel{display:none;margin-top:12px;padding:16px;border:1px solid #30415b;border-radius:14px;background:#111c2e}.form-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:12px}.form-actions{display:flex;align-items:center;gap:9px;flex-wrap:wrap;margin-top:13px}
.filter{display:flex;align-items:center;gap:8px}.filter select{width:auto;min-width:140px;padding:7px 9px}.logbox{max-height:330px;overflow:auto;margin:0;padding:14px;border-radius:0 0 15px 15px;background:#080f1d;color:#a8bdd6;font:11px/1.7 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;white-space:pre-wrap;overflow-wrap:anywhere}.event-list{display:grid;gap:6px;padding:14px}.event{padding:8px 10px;border:1px solid #263750;border-radius:9px;background:#111d30;color:#c0cee0;font-size:11px;overflow-wrap:anywhere}.empty{padding:24px 12px;text-align:center;color:var(--muted)}.footer{display:flex;justify-content:space-between;gap:12px;flex-wrap:wrap;margin-top:24px;color:#71849e;font-size:11px}
@media(max-width:950px){.cards{grid-template-columns:repeat(2,minmax(0,1fr))}.schedulebar{grid-template-columns:1fr 1fr}.schedulebar .timefield{max-width:none}.inline-note{grid-column:1/-1}}
@media(max-width:600px){.wrap{padding:17px 12px 28px}.topbar{align-items:flex-start;flex-direction:column}.topmeta{justify-content:flex-start}.cards{gap:8px}.card{padding:12px}.card .v{font-size:15px}.schedulebar{grid-template-columns:1fr 1fr}.schedulebar .timefield{grid-column:1/-1}.form-grid{grid-template-columns:1fr}.section-head{align-items:flex-start;flex-direction:column}.section-head .filter{width:100%}.filter select{flex:1}.toolbar button{flex:1}}
</style></head><body><main class="wrap">
<header class="topbar">
  <div class="brand"><div class="brandmark">寓</div><div><h1>自动签到控制台</h1><div class="subtitle">寓+ · 本地运行 · 账号凭证不在面板展示</div></div></div>
  <div class="topmeta"><span class="chip"><span class="dot"></span>管理服务</span><span class="chip" id="modeHint">正在读取运行模式…</span></div>
</header>
<div id="warnings" aria-live="polite"></div><div id="runningBanner"></div>
<section class="section"><div class="section-head"><h2>运行概览</h2><span class="hint" id="updatedAt">等待首次刷新</span></div><div class="cards" id="cards"></div></section>
<section class="section"><div class="section-head"><h2>计划与控制</h2><span class="hint">计划修改立即保存，不需要手动编辑配置文件</span></div>
  <div class="schedulebar">
    <div class="timefield"><label for="f_time">每日签到时间</label><input id="f_time" type="time" step="60" value="09:05"></div>
    <button class="primary" id="btnSchedule" type="button">保存计划</button>
    <button id="btnPause" type="button" data-action="pause">暂停定时签到</button>
    <a href="/api/export" download><button type="button" class="ghost">导出 CSV</button></a>
    <div class="inline-note" id="schedMsg" aria-live="polite">当前计划以面板保存值为准。</div>
  </div>
  <div class="toolbar" style="margin-top:12px"><button class="primary" id="btnRun" data-action="run-all" type="button">▶ 立即执行全部账号</button><button id="btnToggleAdd" type="button">＋ 添加账号</button><button id="btnRefresh" type="button">↻ 刷新状态</button></div>
  <div class="form-panel" id="addForm"><div class="section-head"><h2>添加账号</h2><span class="hint">密码只用于登录换取 token，不会回显</span></div>
    <div class="form-grid">
      <div><label for="f_name">账号备注名（可选）</label><input id="f_name" autocomplete="off" placeholder="默认使用账号 ID"></div>
      <div><label for="f_note">备注</label><input id="f_note" autocomplete="off" placeholder="例如：主账号"></div>
      <div><label for="f_uid">账号 ID / 工号</label><input id="f_uid" autocomplete="username" placeholder="输入后加载区域"></div>
      <div><label for="f_pwd">密码</label><input id="f_pwd" type="password" autocomplete="new-password" placeholder="仅本次登录使用"></div>
      <div><label for="f_area">签到区域</label><select id="f_area"><option value="">先填账号 ID 后加载</option></select></div>
    </div><div class="form-actions"><button class="primary" id="btnAdd" type="button">保存并登录</button><button id="btnCancelAdd" type="button">取消</button><span class="small" id="addMsg" aria-live="polite"></span></div>
  </div>
</section>
<section class="section"><div class="section-head"><h2>账号列表 <span class="muted" id="accCount">0</span></h2><span class="hint">定时任务只处理可自动重试的账号；其他错误会在这里提示</span></div>
  <div class="panel"><div class="table-wrap"><table id="accTable"><thead><tr><th>账号</th><th>标识</th><th>区域</th><th>金币</th><th>连续天数</th><th>今日状态</th><th>最近执行</th><th>操作</th></tr></thead><tbody><tr><td colspan="8" class="empty">正在加载…</td></tr></tbody></table></div></div>
</section>
<section class="section"><div class="section-head"><h2>执行历史</h2><div class="filter"><label for="histFilter">筛选结果</label><select id="histFilter"><option value="">全部结果</option><option value="success">成功</option><option value="already_checked_in">今日已签到</option><option value="login_expired">登录失效</option><option value="network_error">网络错误</option><option value="ssl_error">证书错误</option><option value="api_error">接口错误</option><option value="verification_failed">未核验</option><option value="scheduler_error">调度错误</option></select></div></div>
  <div class="panel"><div class="table-wrap"><table id="histTable"><thead><tr><th>时间</th><th>账号</th><th>来源</th><th>结果</th><th>核验</th><th>金币变化</th><th>信息</th></tr></thead><tbody><tr><td colspan="7" class="empty">正在加载…</td></tr></tbody></table></div></div>
</section>
<section class="section"><div class="section-head"><h2>风险事件</h2><span class="hint">最近 12 条</span></div><div class="panel"><div id="events" class="event-list"><div class="empty">暂无事件</div></div></div></section>
<section class="section"><div class="section-head"><h2>运行日志</h2><button id="btnLogs" type="button">刷新日志</button></div><div class="panel"><pre id="logs" class="logbox">加载中…</pre></div></section>
<footer class="footer"><span>仅用于本机管理 · 不展示账号密码与 token</span><span>页面每 15 秒刷新状态；关闭浏览器不影响后台任务</span></footer>
</main>
<script>
let TOKEN="";
const STATUS={success:["ok","签到成功"],already_checked_in:["blue","今日已签到"],login_expired:["warn","登录失效"],network_error:["err","网络错误"],ssl_error:["err","证书错误"],api_error:["err","接口错误"],verification_failed:["warn","未核验"],scheduler_error:["err","调度错误"],busy:["mut","任务运行中"],valid:["ok","有效"]};
const esc=v=>String(v===null||v===undefined?"":v).replace(/[&<>"']/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]));
const unk=v=>(v===null||v===undefined||v==="")?"未知":v;
const tag=(c,t,extra="")=>`<span class="pill ${esc(c)}">${esc(t)}${extra?` · ${esc(extra)}`:""}</span>`;
const pill=(s,extra="")=>{const[c,t]=STATUS[s]||["mut",s||"未知"];return tag(c,t,extra)};
async function api(path,opt){const r=await fetch(path,opt);if(r.status===401){alert("需要登录（局域网访问模式）");location.reload();throw new Error("401")};const j=await r.json().catch(()=>({}));if(!r.ok)throw Object.assign(new Error(j.error||("HTTP "+r.status)),{data:j});return j}
async function post(path,body){return api(path,{method:"POST",headers:{"Content-Type":"application/json","X-Panel-Token":TOKEN},body:JSON.stringify(body||{})})}
function parseEvents(raw){try{return JSON.parse(raw||"[]")}catch(_){return []}}
function stPill(a){if(!a.enabled)return tag("mut","已停用");if(a.today_status)return pill(a.today_status,a.today_verified?"已核验":"");if(a.last_status==="login_expired")return pill("login_expired");return tag("mut","未签到")}
function spark(series){if(!series||series.length<2)return `<span class="small">数据不足</span>`;const w=150,h=34,vals=series.map(p=>Number(p.coins)||0),min=Math.min(...vals),max=Math.max(...vals);const pts=vals.map((v,i)=>((i/(vals.length-1))*w).toFixed(1)+","+(h-4-((v-min)/((max-min)||1))*(h-8)).toFixed(1)).join(" ");return `<svg width="${w}" height="${h}" viewBox="0 0 ${w} ${h}" role="img" aria-label="金币趋势"><polyline fill="none" stroke="#72d6ff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" points="${pts}"/></svg><div class="small">${esc(series[0].date)} ${esc(vals[0])} → ${esc(series[series.length-1].date)} ${esc(vals[vals.length-1])}</div>`}
function schedText(s){const sc=s.scheduler||{};if(sc.mode==="launchd")return sc.launchd_loaded?"launchd 已加载":"launchd 未加载";if(sc.mode==="docker-or-other")return "容器 / 循环模式";return "未知运行模式"}
function schedClass(s){const sc=s.scheduler||{};if(sc.mode==="launchd")return sc.launchd_loaded?"ok":"err";if(sc.mode==="docker-or-other")return "ok";return "warn"}
async function loadState(){let s;try{s=await api("/api/state")}catch(e){return};TOKEN=s.panel_token||TOKEN;document.getElementById("modeHint").textContent=s.web_lan_mode?"局域网模式 · 已启用口令保护":"仅本机访问 · http://127.0.0.1:__PORT__";
const w=s.warnings||[];document.getElementById("warnings").innerHTML=w.map(x=>`<div class="banner">${esc(x)}</div>`).join("");
const rs=s.run_state;document.getElementById("runningBanner").innerHTML=(rs&&rs.running&&!rs.stale)?`<div class="banner blue">任务执行中：${rs.current_account?"正在处理 "+esc(rs.current_account):"准备中"}（来源 ${esc(rs.source)}，${esc(rs.started_at)}）</div>`:"";
const en=(s.accounts||[]).filter(a=>a.enabled),signed=en.filter(a=>a.signed_today).length,srcText={panel:"面板计划",env:"环境变量",default:"默认值"},ct=s.counts_today||{},sc=s.scheduler||{};
const failCount=Object.entries(ct).filter(([k])=>!["success","already_checked_in"].includes(k)).reduce((n,[,v])=>n+(Number(v)||0),0);
const retryText=sc.retry_blocked?"等待人工处理":(sc.retry_count?`${sc.retry_count} 次 · ${sc.next_retry_at?"下次 "+sc.next_retry_at:"待检查"}`:"正常");
document.getElementById("cards").innerHTML=[
["今日签到",`${signed} / ${en.length} 已签`,signed===en.length&&en.length?"ok":signed?"blue":"mut",`${(s.accounts||[]).length} 个账号`],
["今日结果",`成功 ${Number(ct.success)||0} · 已签 ${Number(ct.already_checked_in)||0}`,"",`失败 ${failCount} 次`],
["计划时刻",`${unk(sc.signin_time)} · ${srcText[sc.schedule_source]||"未知来源"}`,"",sc.paused?"定时已暂停":"按计划自动检查"],
["下次执行",unk(sc.next_due),sc.retry_blocked?"warn":"",sc.retry_count?`重试：${retryText}`:(sc.due_now?"已到计划时间":"等待计划时间")],
["调度服务",schedText(s),schedClass(s),sc.mode==="launchd"?(sc.launchd_loaded?"后台任务已加载":"请重新安装后台任务"):"循环服务"],
["定时状态",sc.paused?"已暂停":"运行中",sc.paused?"warn":"ok",sc.retry_blocked?"有账号需要人工处理":"手动执行不受暂停影响"],
["近 14 天金币",spark(s.coins_series),"","每日最后一次成功记录",true],
["Web 面板",s.web_running?"在线":"状态未知",s.web_running?"ok":"warn",s.web_lan_mode?"局域网口令保护":"仅本机访问"]
].map(([k,v,c,sub,isHtml])=>`<div class="card"><div class="k">${esc(k)}</div><div class="v">${c?tag(c,v):(isHtml?v:esc(v))}</div><div class="subv">${esc(sub||"")}</div></div>`).join("");
if(document.activeElement!==document.getElementById("f_time"))document.getElementById("f_time").value=sc.signin_time||"09:05";
document.getElementById("btnPause").textContent=sc.paused?"恢复定时签到":"暂停定时签到";document.getElementById("btnPause").dataset.paused=sc.paused?"true":"false";document.getElementById("accCount").textContent=(s.accounts||[]).length;
document.querySelector("#accTable tbody").innerHTML=(s.accounts||[]).map(a=>`<tr><td><div class="account-name">${esc(a.name)}</div>${a.note?`<div class="small">${esc(a.note)}</div>`:""}${!a.enabled?`<div class="small">已停用</div>`:""}</td><td class="small">${esc(unk(a.user_id_masked))}</td><td>${esc(unk(a.area_id))}</td><td>${esc(unk(a.coins))}</td><td>${esc(unk(a.streak_days))}</td><td>${stPill(a)}</td><td>${a.last_status?pill(a.last_status):tag("mut","暂无记录")}<div class="small">${esc(unk(a.last_run_at))}</div></td><td><div class="actions"><button data-action="run-one" data-name="${esc(a.name)}">执行</button><button data-action="verify" data-name="${esc(a.name)}">验证</button><button data-action="edit" data-name="${esc(a.name)}" data-note="${esc(a.note||"")}" data-area="${esc(a.area_id||"")}">编辑</button><button data-action="toggle" data-name="${esc(a.name)}" data-enabled="${a.enabled?"false":"true"}">${a.enabled?"停用":"启用"}</button><button class="danger" data-action="delete" data-name="${esc(a.name)}">删除</button></div></td></tr>`).join("")||`<tr><td colspan="8" class="empty">还没有账号。点击“添加账号”开始配置。</td></tr>`;
const btn=document.getElementById("btnRun");btn.disabled=!!(rs&&rs.running&&!rs.stale);btn.textContent=btn.disabled?"任务运行中…":"▶ 立即执行全部账号";document.getElementById("updatedAt").textContent="更新于 "+esc(s.now);
const events=(s.recent_runs||[]).flatMap(r=>parseEvents(r.events).map(e=>`<div class="event">${esc(r.finished_at||r.started_at)} · ${esc(r.account_name)} · ${esc(e.type)}：${esc(e.detail||"")}</div>`)).slice(0,12);document.getElementById("events").innerHTML=events.join("")||`<div class="empty">暂无风险事件</div>`;
}
async function setSchedule(){const t=document.getElementById("f_time").value.trim(),m=document.getElementById("schedMsg");if(!t){m.textContent="请选择有效的时间";return};const b=document.getElementById("btnSchedule");b.disabled=true;try{const j=await post("/api/schedule",{time:t});m.textContent=j.message+(j.plist_synced===false?"；面板计划已保存，但旧后台配置未匹配，请运行维护脚本修复。":"");await loadState()}catch(e){m.textContent="保存失败："+e.message}finally{b.disabled=false}}
async function togglePause(){const b=document.getElementById("btnPause"),target=b.dataset.paused!=="true";b.disabled=true;try{await post("/api/paused",{paused:target});await loadState()}catch(e){alert(e.message)}finally{b.disabled=false}}
async function loadHistory(){const f=document.getElementById("histFilter").value;let j;try{j=await api("/api/history?limit=20"+(f?"&status="+encodeURIComponent(f):""))}catch(e){return};document.querySelector("#histTable tbody").innerHTML=(j.rows||[]).map(r=>{const ev=r.verified?"已核验":(r.status==="success"?"已核验":"未核验/不适用");return `<tr><td>${esc(r.finished_at||r.started_at)}</td><td>${esc(r.account_name)}</td><td>${esc(r.source||"")}</td><td>${pill(r.status)}${r.error_type?`<div class="small">${esc(r.error_type)}</div>`:""}</td><td class="small">${esc(ev)}</td><td class="small">${esc(unk(r.score_before))} → ${esc(unk(r.score_after))}<div class="small">连续 ${esc(unk(r.streak_days))} 天</div></td><td class="wrapcell">${esc((r.message||"").slice(0,160))}</td></tr>`}).join("")||`<tr><td colspan="7" class="empty">没有符合条件的执行记录</td></tr>`}
async function loadLogs(){try{const j=await api("/api/logs?lines=200");document.getElementById("logs").textContent=j.text}catch(e){document.getElementById("logs").textContent=String(e)}}
async function runAll(){const b=document.getElementById("btnRun");b.disabled=true;try{await post("/api/run");await loadState();await loadHistory()}catch(e){alert(e.message)}finally{b.disabled=false}}
async function runOne(n){try{await post("/api/run",{account:n});await loadState();await loadHistory()}catch(e){alert(e.message)}}
async function verify(n){try{const j=await post("/api/verify",{name:n});alert(j.ok?`登录态有效：金币 ${unk(j.coins)}，连续 ${unk(j.streak_days)} 天`:`验证失败[${j.status}]：${j.message||j.error}`)}catch(e){alert(e.message)}await loadState()}
async function toggleEn(n,en){try{await post("/api/accounts/update",{name:n,enabled:en});await loadState()}catch(e){alert(e.message)}}
async function del(n){if(!confirm(`删除账号「${n}」？\n登录凭证将被删除；历史签到记录保留。`))return;try{await post("/api/accounts/delete",{name:n});await loadState();await loadHistory()}catch(e){alert(e.message)}}
async function editAccount(n,note,area){const nextNote=prompt("修改备注（留空即清空）",note||"");if(nextNote===null)return;const nextArea=prompt("修改签到区域 AreaID",area||"");if(nextArea===null)return;try{await post("/api/accounts/update",{name:n,note:nextNote,area_id:nextArea.trim()});await loadState()}catch(e){alert(e.message)}}
function toggleAdd(){const f=document.getElementById("addForm");f.style.display=f.style.display==="block"?"none":"block";if(f.style.display==="block")document.getElementById("f_uid").focus()}
document.getElementById("f_uid").addEventListener("change",async e=>{const uid=e.target.value.trim();if(!uid)return;const sel=document.getElementById("f_area");sel.innerHTML="<option>加载中…</option>";try{const j=await api("/api/areas?user_id="+encodeURIComponent(uid));sel.innerHTML=(j.areas||[]).map(a=>`<option value="${esc(a.AreaID)}">${esc(a.Name)}</option>`).join("")||"<option value=''>未获取到</option>"}catch(err){sel.innerHTML="<option value=''>加载失败</option>"}});
async function addAccount(){const m=document.getElementById("addMsg"),b=document.getElementById("btnAdd");m.textContent="正在登录并保存…";b.disabled=true;try{const j=await post("/api/accounts/add",{name:document.getElementById("f_name").value,note:document.getElementById("f_note").value,user_id:document.getElementById("f_uid").value,password:document.getElementById("f_pwd").value,area_id:document.getElementById("f_area").value});m.textContent=`添加成功：${j.user||j.name}（工号 ${j.emp_no||"?"}）`;document.getElementById("f_pwd").value="";document.getElementById("f_uid").value="";document.getElementById("f_name").value="";document.getElementById("f_note").value="";toggleAdd();await loadState()}catch(e){m.textContent="添加失败："+e.message}finally{b.disabled=false}}
function clickAction(e){const b=e.target.closest("button[data-action]");if(!b)return;const a=b.dataset.action,n=b.dataset.name;if(a==="run-all")runAll();else if(a==="run-one")runOne(n);else if(a==="verify")verify(n);else if(a==="toggle")toggleEn(n,b.dataset.enabled==="true");else if(a==="delete")del(n);else if(a==="edit")editAccount(n,b.dataset.note,b.dataset.area);else if(a==="pause")togglePause()}
document.addEventListener("click",clickAction);document.getElementById("btnSchedule").addEventListener("click",setSchedule);document.getElementById("btnToggleAdd").addEventListener("click",toggleAdd);document.getElementById("btnCancelAdd").addEventListener("click",toggleAdd);document.getElementById("btnAdd").addEventListener("click",addAccount);document.getElementById("btnRefresh").addEventListener("click",()=>{loadState();loadHistory()});document.getElementById("btnLogs").addEventListener("click",loadLogs);document.getElementById("histFilter").addEventListener("change",loadHistory);
loadState();loadHistory();loadLogs();setInterval(loadState,15000);
</script></body></html>"""


def build_server(port=None):
    """创建面板服务器。默认仅绑定 127.0.0.1；
    局域网访问需同时设置 YUJIA_WEB_BIND（如 0.0.0.0）与 YUJIA_WEB_PASSWORD，缺密码拒绝启动。"""
    if port is None:
        port = int(os.environ.get("YUJIA_WEB_PORT", "8765"))
    bind = os.environ.get("YUJIA_WEB_BIND") or "127.0.0.1"
    if bind not in LOOPBACK and not web_password():
        raise SystemExit("拒绝启动：YUJIA_WEB_BIND=%s 为非本机回环地址，"
                         "必须同时设置 YUJIA_WEB_PASSWORD（浏览器访问口令）" % bind)
    core.init_storage()
    httpd = ThreadingHTTPServer((bind, port), Handler)
    httpd.daemon_threads = True
    return httpd


def serve(args=None, foreground=True):
    port = int(os.environ.get("YUJIA_WEB_PORT", "8765"))
    for a in (args or []):
        if a.startswith("--port="):
            port = int(a.split("=", 1)[1])
    httpd = build_server(port)
    addr = httpd.server_address[0]
    y.log("Web 管理面板已启动：http://%s:%d %s" % (
        addr, port,
        "（局域网模式，密码保护）" if lan_mode() else "（仅监听本机回环地址）"))
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        httpd.server_close()
    return 0


if __name__ == "__main__":
    import sys
    sys.exit(serve(sys.argv[1:]))
