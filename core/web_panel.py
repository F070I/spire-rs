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
        try:
            n = int(self.headers.get("Content-Length") or 0)
        except ValueError:
            return None
        if n <= 0 or n > MAX_BODY:
            return None
        try:
            data = json.loads(self.rfile.read(n).decode("utf-8"))
            return data if isinstance(data, dict) else None
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
                # 寓+业务登录失败与面板 Basic Auth 无关，绝不能复用 HTTP 401。
                # 401 仅留给 _deny_auth，否则 Web 前端会误弹「局域网访问需要登录」。
                fields = resp if isinstance(resp, dict) else {}
                reason = next((fields.get(k) for k in ("Message", "Msg", "message", "msg", "Error")
                               if isinstance(fields.get(k), str) and fields.get(k).strip()), "")
                code = str(fields.get("Code") or fields.get("code") or "").strip()
                reason = y.sanitize_text(reason).strip()[:180]
                code = re.sub(r"[^a-zA-Z0-9._-]", "", code)[:40]
                detail = reason or "服务器没有提供具体原因"
                if code and code.lower() != "ok":
                    detail += "（代码 %s）" % code
                return self._json({"error": "寓+账号登录失败：%s。请检查账号、密码和签到区域。" % detail,
                                   "error_type": "account_login_failed"}, 422)
            public, secrets = result
            accounts.append({"name": name, "public": public, "secrets": secrets,
                             "note": note, "enabled": True})
            try:
                y.save_accounts(accounts)
            except (OSError, ValueError) as exc:
                y.log("Web 添加账号保存失败：%s" % y.sanitize_text(str(exc)))
                return self._json({"error": "寓+登录成功，但保存账号失败。请检查应用数据目录的写入权限。",
                                   "error_type": "account_save_failed"}, 500)
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
        # Independent app uses StartInterval + SQLite as its single schedule source.
        # There is NO fixed 09:05 calendar event to update. Reloading the agents
        # would unnecessarily interrupt the always-on panel and risks launchd errors.
        if "<key>StartInterval</key>" in text and "<key>StartCalendarInterval</key>" not in text:
            return True
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
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="color-scheme" content="light dark">
<title>寓+签到 · 控制台</title>
<style>
:root{font-family:-apple-system,BlinkMacSystemFont,"SF Pro Display","PingFang SC",sans-serif;color-scheme:light;--bg:#f5f5f7;--side:#f8f8fa;--card:#fff;--ink:#212126;--secondary:#74747e;--line:#e9e9ee;--blue:#1477e9;--blue-soft:#eef5ff;--green:#24936c;--green-soft:#eaf8f0;--amber:#a96718;--amber-soft:#fff6e6;--red:#be4b4a;--red-soft:#fff0f0;--shadow:0 2px 16px rgba(24,27,38,.035);--radius:18px}
*{box-sizing:border-box}[hidden]{display:none!important}html{scroll-behavior:smooth}body{margin:0;min-height:100vh;color:var(--ink);background:var(--bg);font-size:13px;line-height:1.55}button,input,select{font:inherit}button{cursor:pointer}button:disabled{opacity:.5;cursor:not-allowed}a{color:inherit;text-decoration:none}h1,h2,h3,p{margin-top:0}::selection{background:#cbe0ff}*:focus-visible{outline:3px solid #89bfff;outline-offset:2px}
.shell{display:grid;grid-template-columns:225px minmax(0,1fr);min-height:100vh}.sidebar{position:sticky;top:0;display:flex;flex-direction:column;align-self:start;height:100vh;padding:27px 15px 18px;background:var(--side);border-right:1px solid var(--line)}.identity{display:flex;align-items:center;gap:12px;padding:4px 10px 30px}.appmark{display:grid;place-items:center;width:43px;height:43px;border-radius:13px;background:linear-gradient(155deg,#73b4ff,#1676e6);color:#fff;font-size:21px;font-weight:720;box-shadow:0 6px 15px #4387ef2a}.appname{font-size:15px;font-weight:740;letter-spacing:-.5px}.appcaption{color:var(--secondary);font-size:11px;margin-top:1px}.nav-label{padding:0 14px;margin:13px 0 8px;font-size:10px;font-weight:700;color:#9696a0;letter-spacing:.1em;text-transform:uppercase}.nav{display:grid;gap:3px}.nav-item{display:flex;gap:13px;align-items:center;width:100%;padding:11px 14px;background:transparent;color:#60606a;border:0;border-radius:11px;text-align:left;font-size:13px;font-weight:530;transition:background .15s}.nav-item:hover{background:#e9e9ed}.nav-item.active{background:#e7f0fe;color:#136fdb;font-weight:680}.nav-icon{display:grid;place-items:center;width:19px;height:19px;flex:0 0 19px}.nav-icon svg{width:18px;height:18px;stroke:currentColor;stroke-width:1.7;fill:none;stroke-linecap:round;stroke-linejoin:round}.nav-count{margin-left:auto;font-size:11px;opacity:.75}.sidebar-foot{margin-top:auto;border-top:1px solid var(--line);padding:15px 12px 3px}.service-line{display:flex;gap:8px;align-items:center;color:var(--secondary);font-size:12px}.service-dot{display:inline-block;width:7px;height:7px;border-radius:50%;background:var(--green)}.service-dot.off{background:var(--amber)}.side-note{color:#a1a1ab;font-size:11px;margin-top:9px}
.workspace{min-width:0}.topbar{height:70px;display:flex;justify-content:space-between;align-items:center;gap:16px;padding:0 clamp(22px,4vw,56px);border-bottom:1px solid var(--line);background:rgba(255,255,255,.56)}.crumb{font-size:12px;color:var(--secondary)}.top-actions{display:flex;align-items:center;gap:9px}.top-status{font-size:11px;color:var(--secondary);margin-right:6px}.main{width:100%;max-width:1320px;margin:0 auto;padding:36px clamp(22px,4vw,56px) 65px}.page-heading{display:flex;gap:20px;justify-content:space-between;align-items:flex-end;margin-bottom:25px}.eyebrow{font-size:11px;color:var(--blue);font-weight:720;letter-spacing:.075em;margin-bottom:6px}.page-heading h1{font-size:29px;line-height:1.16;letter-spacing:-1px;font-weight:730;margin:0 0 8px}.lede{font-size:13px;color:var(--secondary);margin:0}.heading-buttons{display:flex;align-items:center;gap:8px;flex-wrap:wrap}
.btn{display:inline-flex;gap:7px;align-items:center;justify-content:center;white-space:nowrap;min-height:36px;padding:8px 14px;border:1px solid #dfdfe5;background:var(--card);border-radius:10px;color:#474751;font-weight:590;box-shadow:0 1px 1px #17171a08;transition:background .15s,border-color .15s,transform .1s}.btn:hover{background:#f9faff;border-color:#c8d7ec}.btn:active{transform:scale(.986)}.btn.primary{background:var(--blue);border-color:var(--blue);color:white;box-shadow:0 2px 6px #1678e628}.btn.primary:hover{background:#0066d8}.btn.negative{color:var(--red);border-color:#f0d0d0}.btn.quiet{background:transparent;box-shadow:none;border-color:transparent}.btn.small{padding:5px 10px;min-height:29px;font-size:11px}.btn.icon-btn{min-width:36px;padding:7px;font-size:18px;line-height:1}.btn:disabled{transform:none;box-shadow:none}
.view{animation:appear .2s ease-out}@keyframes appear{from{opacity:.6;transform:translateY(4px)}to{opacity:1;transform:none}}.surface{border:1px solid var(--line);background:var(--card);border-radius:var(--radius);box-shadow:var(--shadow)}.surface-pad{padding:23px}.section-title{display:flex;justify-content:space-between;align-items:center;gap:12px;margin:30px 0 13px}.section-title h2{margin:0;font-size:15px;letter-spacing:-.24px;font-weight:700}.muted{color:var(--secondary)}.small{font-size:11px;color:var(--secondary)}.empty{color:var(--secondary);text-align:center;padding:30px}.hero{display:grid;grid-template-columns:1.55fr 1fr;gap:14px}.hero-primary{padding:29px 30px;background:linear-gradient(130deg,#fff 47%,#f0f6ff 100%);min-height:215px}.hero-kicker{color:var(--secondary);font-size:12px}.hero-number{margin:10px 0 4px;line-height:1.1;font-size:42px;letter-spacing:-1.7px;font-weight:740}.hero-number span{font-size:19px;font-weight:570;color:#9797a0}.hero-sub{color:var(--secondary);font-size:12px}.hero-footer{display:flex;gap:12px;align-items:center;flex-wrap:wrap;margin-top:29px}.hero-side{display:flex;flex-direction:column;justify-content:space-between;gap:12px;padding:27px 28px;background:var(--card)}.light-label{color:var(--secondary);font-size:11px}.next-time{font-weight:680;font-size:24px;letter-spacing:-.7px;margin:4px 0}.progress{height:6px;background:#edf0f4;border-radius:20px;overflow:hidden;margin-top:15px}.progress-bar{height:100%;width:0;background:#329a77;border-radius:20px;transition:width .4s}.stat-row{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:14px;margin-top:14px}.stat{padding:20px 22px;min-width:0}.stat-label{font-size:11px;color:var(--secondary)}.stat-value{font-size:21px;font-weight:710;letter-spacing:-.7px;margin-top:7px;overflow-wrap:anywhere}.stat-detail{font-size:11px;color:var(--secondary);margin-top:3px}.stat-value .pill{font-size:12px}.overview-cols{display:grid;grid-template-columns:minmax(0,1.23fr) minmax(260px,.77fr);gap:14px;margin-top:15px}.section-in{display:flex;align-items:center;justify-content:space-between;gap:14px;margin-bottom:17px}.section-in h2{font-size:14px;font-weight:700;margin:0}.trend{height:103px;display:flex;justify-content:center;align-items:center}.trend svg{width:100%;height:100%;max-height:100px}.timeline-item{display:flex;gap:12px;padding:13px 0;border-top:1px solid var(--line)}.timeline-item:first-child{border-top:0}.timeline-dot{flex:0 0 8px;width:8px;height:8px;background:#a4a9b4;border-radius:100%;margin-top:7px}.timeline-dot.good{background:var(--green)}.timeline-dot.bad{background:var(--red)}.timeline-copy{min-width:0;flex:1}.timeline-title{font-size:12px;font-weight:600;overflow-wrap:anywhere}.timeline-sub{font-size:11px;color:var(--secondary);margin-top:2px}.warning-list{display:grid;gap:9px;margin-bottom:15px}.banner{padding:12px 14px;background:var(--amber-soft);color:#835b21;border:1px solid #f8e9cd;border-radius:12px;font-size:12px;overflow-wrap:anywhere}.banner.blue{color:#185baf;background:var(--blue-soft);border-color:#dfebfd}.banner.error{color:var(--red);background:var(--red-soft);border-color:#f6dada}
.table-scroll{width:100%;overflow:auto;-webkit-overflow-scrolling:touch}table{width:100%;border-collapse:collapse;min-width:680px}th{padding:15px 16px;text-align:left;color:#8f8f99;font-size:11px;font-weight:590;border-bottom:1px solid var(--line);white-space:nowrap;background:#fdfdfe}td{padding:14px 16px;text-align:left;border-bottom:1px solid #f0f0f3;vertical-align:middle;font-size:12px;overflow-wrap:anywhere}tr:last-child td{border-bottom:0}tbody tr:hover{background:#fcfcfe}.account-line{display:flex;align-items:center;gap:11px}.avatar{display:grid;place-items:center;width:34px;height:34px;border-radius:11px;background:#eaf3ff;color:#3279cc;font-weight:720;font-size:13px;flex:none}.account-name{font-weight:670;font-size:12px}.action-cell{white-space:nowrap}.actions{display:flex;gap:4px;align-items:center}.pill{display:inline-flex;align-items:center;gap:6px;white-space:nowrap;padding:4px 9px;border-radius:8px;font-size:11px;font-weight:610;background:#f1f1f4;color:#787883}.pill.ok{color:var(--green);background:var(--green-soft)}.pill.blue{color:var(--blue);background:var(--blue-soft)}.pill.warn{color:var(--amber);background:var(--amber-soft)}.pill.err{color:var(--red);background:var(--red-soft)}.pill.mut{color:#7d7d89;background:#f1f1f4}.account-list-mini{display:grid}.mini-acc{display:flex;align-items:center;gap:11px;padding:12px 0;border-top:1px solid var(--line)}.mini-acc:first-child{border-top:0}.mini-acc .pill{margin-left:auto}
.settings-grid{display:grid;grid-template-columns:minmax(0,1.1fr) minmax(270px,.9fr);gap:16px}.form-row{display:grid;gap:6px;margin-bottom:20px}.form-row label{font-size:12px;color:#565660;font-weight:630}.field{width:100%;min-width:0;padding:10px 12px;height:40px;background:#fcfcfd;border:1px solid #dedee5;border-radius:10px;color:var(--ink);outline:0;transition:border .15s,box-shadow .15s}.field:focus{border-color:#78adf6;box-shadow:0 0 0 3px #3b91fb1b}.field::placeholder{color:#b5b5c0}.field[type=time]{max-width:170px;font-weight:650;font-size:17px}.form-note{font-size:11px;color:var(--secondary);margin-top:7px}.setting-line{display:flex;justify-content:space-between;align-items:center;gap:10px;padding:17px 0;border-top:1px solid var(--line)}.setting-line:first-child{border-top:0}.setting-name{font-size:13px;font-weight:650}.setting-detail{color:var(--secondary);font-size:11px;margin-top:4px}.setting-msg{font-size:11px;margin-top:13px;min-height:17px;color:var(--secondary)}.info-pair{display:flex;justify-content:space-between;gap:14px;font-size:12px;padding:13px 0;border-bottom:1px solid var(--line)}.info-pair:last-child{border-bottom:0}.info-pair span:last-child{font-weight:590;text-align:right;overflow-wrap:anywhere}
.filter{display:flex;align-items:center;gap:8px}.filter select{width:auto;min-width:140px}.logbox{padding:20px 21px;margin:0;color:#4c5864;background:#fbfbfc;border-radius:0 0 var(--radius) var(--radius);max-height:530px;overflow:auto;font:11px/1.8 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;white-space:pre-wrap;overflow-wrap:anywhere}.footnote{margin-top:34px;font-size:11px;color:#a3a3ad;display:flex;justify-content:space-between;gap:15px;flex-wrap:wrap}
.overlay{position:fixed;z-index:20;inset:0;background:rgba(23,28,39,.35);backdrop-filter:blur(6px);display:flex;justify-content:flex-end}.sheet{width:min(480px,100%);height:100%;overflow:auto;background:#fff;box-shadow:-14px 0 40px #17203520;padding:30px 33px;animation:slide .2s ease-out}@keyframes slide{from{transform:translateX(20px);opacity:.85}to{transform:none;opacity:1}}.sheet-head{display:flex;justify-content:space-between;align-items:center;gap:10px;margin-bottom:24px}.sheet-head h2{font-size:21px;letter-spacing:-.5px;margin:0}.sheet-copy{font-size:12px;color:var(--secondary);margin:-12px 0 27px}.sheet-actions{display:flex;gap:9px;align-items:center;margin-top:27px}.sheet-msg{font-size:12px;margin-top:15px;color:var(--red);min-height:18px;overflow-wrap:anywhere}.sheet-divider{height:1px;background:var(--line);margin:24px 0}.toast{position:fixed;right:22px;bottom:22px;z-index:30;border:1px solid #dfe8f4;padding:13px 18px;max-width:min(400px,calc(100vw - 32px));border-radius:13px;background:#fff;box-shadow:0 8px 28px #1f32441a;font-size:12px;overflow-wrap:anywhere}.toast.error{border-color:#f0cdcf;color:#a33d40}.inline-error{color:var(--red);font-size:12px}
@media(max-width:1160px){.shell{grid-template-columns:190px minmax(0,1fr)}.sidebar{padding-inline:10px}.topbar{padding-inline:24px}.main{padding-inline:24px}.hero{grid-template-columns:1fr 1fr}.hero-primary{padding:23px}.hero-side{padding:23px}.stat-row{gap:9px}.stat{padding:15px}.overview-cols{grid-template-columns:1fr}}@media(max-width:780px){.shell{display:block}.sidebar{position:sticky;height:auto;top:0;z-index:10;display:block;padding:10px 16px;border-right:0;border-bottom:1px solid var(--line);background:rgba(248,248,250,.96);backdrop-filter:blur(14px)}.identity{padding:0;display:inline-flex}.appmark{width:29px;height:29px;border-radius:9px;font-size:14px}.appname{font-size:12px}.appcaption,.nav-label,.sidebar-foot{display:none}.nav{display:flex;gap:4px;overflow:auto;margin-top:9px}.nav-item{white-space:nowrap;font-size:11px;gap:5px;padding:7px 10px}.nav-icon{width:14px;height:14px;flex-basis:14px}.nav-icon svg{width:14px;height:14px}.nav-count{display:none}.topbar{height:49px;padding:0 16px}.main{padding:27px 16px 50px}.page-heading{align-items:flex-start;flex-direction:column}.page-heading h1{font-size:25px}.hero{grid-template-columns:1fr}.hero-primary{min-height:auto}.stat-row{grid-template-columns:1fr 1fr}.settings-grid{grid-template-columns:1fr}.top-status{display:none}}@media(max-width:470px){.stat-row{grid-template-columns:1fr}.heading-buttons{width:100%}.heading-buttons .btn{flex:1}.surface-pad{padding:17px}.hero-primary,.hero-side{padding:21px}.sheet{padding:24px 20px}.topbar .btn{min-height:31px;padding:5px 9px}}
@media(prefers-reduced-motion:reduce){*,*:before,*:after{animation:none!important;transition:none!important}html{scroll-behavior:auto}}
</style></head><body>
<div class="shell">
  <aside class="sidebar" aria-label="主导航">
    <div class="identity"><span class="appmark">寓</span><div><div class="appname">寓+签到</div><div class="appcaption">Sign-in Manager</div></div></div>
    <div class="nav-label">工作空间</div><nav class="nav">
      <button type="button" class="nav-item active" data-view="overview"><span class="nav-icon"><svg viewBox="0 0 24 24"><rect x="3" y="3" width="8" height="8" rx="2"/><rect x="13" y="3" width="8" height="8" rx="2"/><rect x="3" y="13" width="8" height="8" rx="2"/><rect x="13" y="13" width="8" height="8" rx="2"/></svg></span>概览</button>
      <button type="button" class="nav-item" data-view="accounts"><span class="nav-icon"><svg viewBox="0 0 24 24"><circle cx="9" cy="8" r="3"/><path d="M3 20v-1a6 6 0 0 1 12 0v1M17 11a3 3 0 0 0 0-6M17 14a5 5 0 0 1 4 5v1"/></svg></span>账号管理 <span class="nav-count" id="navAccountCount">0</span></button>
      <button type="button" class="nav-item" data-view="schedule"><span class="nav-icon"><svg viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="16" rx="3"/><path d="M7 2v6M17 2v6M3 10h18M8 15h3"/></svg></span>签到计划</button>
      <button type="button" class="nav-item" data-view="history"><span class="nav-icon"><svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="M12 7v6l4 2"/></svg></span>执行记录</button>
      <button type="button" class="nav-item" data-view="logs"><span class="nav-icon"><svg viewBox="0 0 24 24"><path d="M5 3h11l3 3v15H5zM8 10h8M8 14h8M8 18h6"/></svg></span>运行日志</button>
    </nav>
    <div class="sidebar-foot"><div class="service-line"><span class="service-dot" id="serviceDot"></span><span id="sideService">正在连接本机服务</span></div><div class="side-note">数据仅保存在你的设备中</div></div>
  </aside>
  <div class="workspace"><header class="topbar"><div class="crumb" id="crumb">工作空间 / 概览</div><div class="top-actions"><span class="top-status" id="updatedAt">正在同步</span><button class="btn small" id="btnRefresh" type="button">↻ 刷新</button></div></header>
    <main class="main">
      <div id="warnings" class="warning-list" aria-live="polite"></div><div id="runningBanner" class="warning-list" aria-live="polite"></div>
      <section class="view" id="view-overview">
        <div class="page-heading"><div><div class="eyebrow">CONTROL CENTER</div><h1>今天，一切尽在掌握。</h1><p class="lede">签到状态和运行情况，清晰呈现。</p></div><div class="heading-buttons"><button class="btn" id="btnQuickAccount" type="button">＋ 添加账号</button><button class="btn primary" id="btnRun" type="button">立即签到</button></div></div>
        <div class="hero"><div class="surface hero-primary"><div class="hero-kicker">今日签到进度</div><div class="hero-number"><strong id="heroDone">—</strong> <span>/ <span id="heroTotal">—</span> 个账号</span></div><div class="hero-sub" id="heroText">读取运行状态中…</div><div class="progress"><div class="progress-bar" id="heroProgress"></div></div><div class="hero-footer"><span class="pill" id="heroStatus">检查中</span><button class="btn small" type="button" data-go="accounts">管理账号 →</button></div></div>
        <div class="surface hero-side"><div><div class="light-label">下次计划签到</div><div class="next-time" id="nextDue">—</div><div class="small" id="planSource">每天自动检查</div></div><div><button class="btn small" type="button" data-go="schedule">管理计划 →</button></div></div></div>
        <div class="stat-row" id="cards"><div class="surface stat"><div class="stat-label">累计记录</div><div class="stat-value">—</div></div></div>
        <div class="overview-cols"><div class="surface surface-pad"><div class="section-in"><h2>最近 14 天 · 金币趋势</h2><span class="small">已记录的数据</span></div><div class="trend" id="trend">暂无趋势数据</div><div class="small" id="trendDetail"></div></div><div class="surface surface-pad"><div class="section-in"><h2>最近动态</h2><button class="btn small quiet" type="button" data-go="history">查看记录 →</button></div><div id="recentActivity"><div class="empty">暂无执行记录</div></div></div></div>
      </section>
      <section class="view" id="view-accounts" hidden>
        <div class="page-heading"><div><div class="eyebrow">ACCOUNTS</div><h1>账号管理</h1><p class="lede">管理签到账号、启停状态及登录凭证。</p></div><div class="heading-buttons"><button class="btn primary" id="btnToggleAdd" type="button">＋ 添加账号</button></div></div>
        <div class="surface"><div class="table-scroll"><table id="accTable"><thead><tr><th>账号</th><th>区域</th><th>金币 / 连续天数</th><th>今日状态</th><th>最近执行</th><th>操作</th></tr></thead><tbody><tr><td colspan="6" class="empty">正在加载…</td></tr></tbody></table></div></div>
        <p class="small" style="margin-top:14px">工号会脱敏展示。删除账号只删除登录凭证，执行历史仍然保留。</p>
      </section>
      <section class="view" id="view-schedule" hidden>
        <div class="page-heading"><div><div class="eyebrow">AUTOMATION</div><h1>签到计划</h1><p class="lede">计划保存后会自动同步，无需手动修改脚本。</p></div></div>
        <div class="settings-grid"><div class="surface surface-pad"><div class="section-in"><h2>每日自动签到</h2><span class="pill blue" id="scheduleBadge">自动</span></div>
          <div class="form-row"><label for="f_time">每天执行时间</label><input class="field" id="f_time" type="time" step="60" value="09:05"><div class="form-note">使用运行设备的本地时区。修改后立即生效。</div></div>
          <button class="btn primary" id="btnSchedule" type="button">保存计划</button><div class="setting-msg" id="schedMsg" role="status"></div>
          <div class="setting-line"><div><div class="setting-name">自动签到</div><div class="setting-detail">暂停后不会执行定时任务，手动签到仍可使用。</div></div><button class="btn" id="btnPause" type="button" data-paused="false">暂停计划</button></div>
          <div class="setting-line"><div><div class="setting-name">手动执行</div><div class="setting-detail">立即执行所有已启用账号的签到。</div></div><button class="btn" id="btnRunSchedule" type="button">立即签到</button></div>
        </div><div class="surface surface-pad"><div class="section-in"><h2>运行环境</h2></div><div class="info-pair"><span class="muted">调度服务</span><span id="schedService">—</span></div><div class="info-pair"><span class="muted">当前签到时刻</span><span id="schedEffective">—</span></div><div class="info-pair"><span class="muted">下次执行</span><span id="schedNext">—</span></div><div class="info-pair"><span class="muted">自动重试</span><span id="schedRetry">—</span></div><div class="info-pair"><span class="muted">访问模式</span><span id="modeHint">—</span></div><div class="setting-line"><div><div class="setting-name">导出执行记录</div><div class="setting-detail">保存为 CSV 文件</div></div><a class="btn" href="/api/export" download id="exportLink">导出 CSV</a></div></div></div>
      </section>
      <section class="view" id="view-history" hidden>
        <div class="page-heading"><div><div class="eyebrow">ACTIVITY</div><h1>执行记录</h1><p class="lede">每次任务的结果、状态与时间都会在这里保留。</p></div><div class="heading-buttons"><div class="filter"><select class="field" id="histFilter" aria-label="筛选结果"><option value="">全部结果</option><option value="success">签到成功</option><option value="already_checked_in">今日已签</option><option value="login_expired">登录失效</option><option value="network_error">网络错误</option><option value="ssl_error">证书错误</option><option value="api_error">接口错误</option><option value="verification_failed">核验失败</option><option value="scheduler_error">调度错误</option></select></div></div></div>
        <div class="surface"><div class="table-scroll"><table id="histTable"><thead><tr><th>时间</th><th>账号</th><th>来源</th><th>结果</th><th>核验</th><th>金币变化</th><th>信息</th></tr></thead><tbody><tr><td colspan="7" class="empty">正在加载…</td></tr></tbody></table></div></div>
        <div class="section-title"><h2>最近事件</h2><span class="small">仅展示最近 12 条</span></div><div class="surface surface-pad" id="events"><div class="empty">暂无风险事件</div></div>
      </section>
      <section class="view" id="view-logs" hidden>
        <div class="page-heading"><div><div class="eyebrow">DIAGNOSTICS</div><h1>运行日志</h1><p class="lede">遇到网络异常、签到失败时，可以在这里查看详细信息。</p></div><div class="heading-buttons"><button class="btn" id="btnLogs" type="button">↻ 刷新日志</button></div></div><div class="surface"><pre class="logbox" id="logs">加载中…</pre></div>
      </section>
      <div class="footnote"><span>寓+签到 · 个人本地管理工具</span><span>自动刷新间隔 15 秒 · 退出窗口不影响后台任务</span></div>
    </main>
  </div>
</div>
<div class="overlay" id="accountOverlay" hidden><section class="sheet" role="dialog" aria-modal="true" aria-labelledby="dialogTitle"><div class="sheet-head"><h2 id="dialogTitle">添加账号</h2><button type="button" class="btn icon-btn quiet" id="dialogClose" aria-label="关闭">×</button></div><p class="sheet-copy" id="dialogSubtitle">输入寓+账号并选择区域，登录成功后自动保存。</p>
  <div id="addFields"><div class="form-row"><label for="f_name">显示名称（可选）</label><input class="field" id="f_name" autocomplete="off" placeholder="默认使用账号 ID"></div><div class="form-row"><label for="f_uid">账号 / 工号</label><input class="field" id="f_uid" autocomplete="username" placeholder="输入账号后自动获取区域"></div><div class="form-row"><label for="f_pwd">登录密码</label><input class="field" id="f_pwd" type="password" autocomplete="new-password" placeholder="用于本次验证"></div><div class="form-row"><label for="f_area">签到区域</label><select class="field" id="f_area"><option value="">请先填写账号 ID</option></select><div class="form-note">区域列表获取失败时，可填写下方 AreaID。</div></div><div class="form-row"><label for="f_area_manual">手动输入 AreaID（可选）</label><input class="field" id="f_area_manual" placeholder="如 GL"></div><div class="form-row"><label for="f_note">备注（可选）</label><input class="field" id="f_note" placeholder="例如：主账号"></div></div>
  <div id="editFields" hidden><div class="form-row"><label for="edit_note">备注</label><input class="field" id="edit_note" placeholder="留空即清空备注"></div><div class="form-row"><label for="edit_area">签到区域 AreaID</label><input class="field" id="edit_area" placeholder="例如 GL"></div></div>
  <div class="sheet-actions"><button class="btn primary" type="button" id="dialogSave">保存并登录</button><button class="btn" type="button" id="dialogCancel">取消</button></div><div class="sheet-msg" id="addMsg" aria-live="polite"></div><p class="small" style="margin-top:26px">不在界面中显示密码或访问令牌。保存的账号数据仅供本机后台使用。</p>
</section></div>
<div class="overlay" id="confirmOverlay" hidden><section class="sheet" role="alertdialog" aria-modal="true" aria-labelledby="confirmTitle"><div class="sheet-head"><h2 id="confirmTitle">删除账号？</h2><button type="button" class="btn icon-btn quiet" id="confirmClose">×</button></div><p class="sheet-copy" id="confirmMessage">删除后登录凭证无法恢复，但历史记录将保留。</p><div class="sheet-actions"><button class="btn negative" type="button" id="confirmDelete">删除账号</button><button class="btn" type="button" id="confirmCancel">取消</button></div><div class="sheet-msg" id="confirmError"></div></section></div>
<div class="toast" id="toast" hidden role="status" aria-live="polite"></div>
<script>
"use strict";
let TOKEN="",currentState=null,dialogMode="add",editingName="",deletingName="",areaRequestId=0,toastTimer=null;
const $=id=>document.getElementById(id);
const STATUS={success:["ok","签到成功"],already_checked_in:["blue","今日已签到"],login_expired:["warn","登录失效"],network_error:["err","网络错误"],ssl_error:["err","证书错误"],api_error:["err","接口错误"],verification_failed:["warn","未核验"],scheduler_error:["err","调度错误"],busy:["mut","执行中"],valid:["ok","有效"]};
const esc=v=>String(v==null?"":v).replace(/[&<>"']/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]));
const unk=v=>(v===null||v===undefined||v==="")?"—":v;
const pill=(s,extra="")=>{const [c,t]=STATUS[s]||["mut",s||"未知"];return `<span class="pill ${c}">${esc(t)}${extra?" · "+esc(extra):""}</span>`};
function toast(msg,error=false){const t=$("toast");t.textContent=msg;t.classList.toggle("error",error);t.hidden=false;clearTimeout(toastTimer);toastTimer=setTimeout(()=>t.hidden=true,4800)}
async function api(path,opt){let r;try{r=await fetch(path,{cache:"no-store",...opt})}catch(e){throw new Error("无法连接本地管理服务，请检查后台是否运行")};const j=await r.json().catch(()=>({}));if(!r.ok){if(r.status===401&&j.error==="需要登录（局域网访问模式）"){throw new Error("管理面板需要访问口令，请在局域网浏览器中登录（本机 App 无需密码）")};throw Object.assign(new Error(j.error||("请求失败（HTTP "+r.status+"）")),{status:r.status,data:j})}return j}
async function post(path,body){if(!TOKEN)throw new Error("面板还未初始化，请刷新后重试");return api(path,{method:"POST",headers:{"Content-Type":"application/json","X-Panel-Token":TOKEN},body:JSON.stringify(body||{})})}
function showView(name){if(!["overview","accounts","schedule","history","logs"].includes(name))return;document.querySelectorAll(".view").forEach(el=>el.hidden=el.id!=="view-"+name);document.querySelectorAll(".nav-item").forEach(el=>{const yes=el.dataset.view===name;el.classList.toggle("active",yes);el.setAttribute("aria-current",yes?"page":"false")});const names={overview:"概览",accounts:"账号管理",schedule:"签到计划",history:"执行记录",logs:"运行日志"};$("crumb").textContent="工作空间 / "+names[name];if(name==="history")loadHistory();if(name==="logs")loadLogs();window.scrollTo({top:0,behavior:"instant"})}
function activityMarkup(s){const rows=(s.recent_runs||[]).slice(0,4);return rows.map(r=>{const ok=["success","already_checked_in"].includes(r.status);return `<div class="timeline-item"><span class="timeline-dot ${ok?"good":"bad"}"></span><div class="timeline-copy"><div class="timeline-title">${esc(r.account_name)} · ${esc((STATUS[r.status]||[0,r.status||"未知"])[1])}</div><div class="timeline-sub">${esc(r.finished_at||r.started_at||"—")} · ${esc(r.source||"未知来源")}</div></div></div>`}).join("")||`<div class="empty">暂无执行记录</div>`}
function makeTrend(series){if(!series||series.length<2)return `<span class="small">积累两天以上的数据后显示趋势</span>`;const w=500,h=115,vals=series.map(p=>Number(p.coins)||0);const lo=Math.min(...vals),hi=Math.max(...vals),ran=hi-lo||1;const points=vals.map((v,i)=>[12+i*(w-24)/(vals.length-1),h-14-(v-lo)/ran*(h-31)]);const path=points.map((p,i)=>(i?"L":"M")+p[0].toFixed(1)+" "+p[1].toFixed(1)).join(" ");return `<svg viewBox="0 0 ${w} ${h}" role="img" aria-label="最近金币趋势"><path d="M 0 ${h-12} H ${w}" stroke="#edf0f4" stroke-width="1"/><path d="${path}" fill="none" stroke="#398bef" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>${points.map(([x,y])=>`<circle cx="${x}" cy="${y}" r="3.2" fill="#fff" stroke="#398bef" stroke-width="2"/>`).join("")}</svg>`}
const formatDate=v=>{if(!v)return "—";return String(v).replace("T"," ").slice(0,19)};
function renderState(s){currentState=s;TOKEN=s.panel_token||TOKEN;const ac=s.accounts||[],en=ac.filter(a=>a.enabled),done=en.filter(a=>a.signed_today).length,sc=s.scheduler||{},ct=s.counts_today||{},cs=s.counts_total||{};$("navAccountCount").textContent=ac.length;
$("updatedAt").textContent="更新于 "+new Date().toLocaleTimeString("zh-CN",{hour:"2-digit",minute:"2-digit"});$("serviceDot").classList.toggle("off",sc.mode==="launchd"&&sc.launchd_loaded===false);$("sideService").textContent=sc.mode==="launchd"?(sc.launchd_loaded?"后台服务运行中":"后台服务未加载"):"管理服务已连接";
$("warnings").innerHTML=(s.warnings||[]).map(w=>`<div class="banner">${esc(w)}</div>`).join("");const rs=s.run_state;$("runningBanner").innerHTML=(rs&&rs.running&&!rs.stale)?`<div class="banner blue">任务执行中：${esc(rs.current_account||"正在准备")} · ${esc(rs.started_at||"")}</div>`:"";
$("heroDone").textContent=done;$("heroTotal").textContent=en.length;$("heroText").textContent=en.length?(done===en.length?"今天的签到任务已全部完成":"还有 "+(en.length-done)+" 个启用账号尚未完成签到"):"还没有启用的账号，添加账号即可开始";$("heroProgress").style.width=(en.length?Math.round(done/en.length*100):0)+"%";$("heroStatus").className="pill "+(sc.paused?"warn":done===en.length&&en.length?"ok":"blue");$("heroStatus").textContent=sc.paused?"计划已暂停":done===en.length&&en.length?"今日已完成":"等待执行";
$("nextDue").textContent=unk(sc.next_due);$("planSource").textContent=sc.paused?"定时任务已暂停":"计划时间 "+unk(sc.signin_time)+" · "+({panel:"面板设置",env:"环境变量",default:"默认值"}[sc.schedule_source]||"计划");
const total=Object.values(cs).reduce((n,v)=>n+(Number(v)||0),0),fails=Object.entries(ct).filter(([k])=>!["success","already_checked_in"].includes(k)).reduce((n,[,v])=>n+(Number(v)||0),0);$("cards").innerHTML=[['历史执行',String(total)+' 次','累计存档记录'],['今日异常',fails+' 次',fails?'查看执行记录定位原因':'暂无错误记录'],['调度服务',sc.mode==="launchd"?(sc.launchd_loaded?"已加载":"未加载"):"循环模式",sc.paused?"已暂停":(sc.retry_count?"重试中 · "+sc.retry_count+" 次":"自动运行")]].map(([k,v,d])=>`<div class="surface stat"><div class="stat-label">${esc(k)}</div><div class="stat-value">${esc(v)}</div><div class="stat-detail">${esc(d)}</div></div>`).join("");
$("trend").innerHTML=makeTrend(s.coins_series);const series=s.coins_series||[];$("trendDetail").textContent=series.length>=2?`${series[0].date} · ${unk(series[0].coins)} → ${series[series.length-1].date} · ${unk(series[series.length-1].coins)}`:"";$("recentActivity").innerHTML=activityMarkup(s);
$("accTable").querySelector("tbody").innerHTML=ac.map(a=>`<tr><td><div class="account-line"><div class="avatar">${esc((a.name||"?").slice(0,1))}</div><div><div class="account-name">${esc(a.name)}</div><div class="small">${esc(a.user_id_masked||"")}${a.note?" · "+esc(a.note):""}</div></div></div></td><td>${esc(unk(a.area_id))}</td><td>${esc(unk(a.coins))} <span class="small">/ ${esc(unk(a.streak_days))} 天</span></td><td>${!a.enabled?`<span class="pill mut">已停用</span>`:a.today_status?pill(a.today_status):`<span class="pill mut">未签到</span>`}</td><td><div>${a.last_status?pill(a.last_status):`<span class="small">暂无记录</span>`}</div><div class="small">${esc(formatDate(a.last_run_at))}</div></td><td class="action-cell"><div class="actions"><button class="btn small" type="button" data-action="run-one" data-idx="${ac.indexOf(a)}">执行</button><button class="btn small quiet" type="button" data-action="edit" data-idx="${ac.indexOf(a)}">编辑</button><button class="btn small quiet" type="button" data-action="toggle" data-idx="${ac.indexOf(a)}">${a.enabled?"停用":"启用"}</button><button class="btn small quiet" type="button" data-action="verify" data-idx="${ac.indexOf(a)}">验证</button><button class="btn small quiet" type="button" data-action="delete" data-idx="${ac.indexOf(a)}" style="color:#c25151">删除</button></div></td></tr>`).join("")||`<tr><td colspan="6" class="empty">暂无账号。点击右上角「添加账号」开始配置。</td></tr>`;
if(document.activeElement!==$("f_time"))$("f_time").value=sc.signin_time||"09:05";$("scheduleBadge").textContent=sc.paused?"已暂停":"正在运行";$("scheduleBadge").className="pill "+(sc.paused?"warn":"ok");$("btnPause").dataset.paused=sc.paused?"true":"false";$("btnPause").textContent=sc.paused?"恢复计划":"暂停计划";$("schedService").textContent=sc.mode==="launchd"?(sc.launchd_loaded?"launchd · 正常":"launchd · 未加载"):"循环调度";$("schedEffective").textContent=unk(sc.signin_time);$("schedNext").textContent=unk(sc.next_due);$("schedRetry").textContent=sc.retry_blocked?"等待人工处理":sc.next_retry_at?"下次 "+sc.next_retry_at:sc.retry_count?sc.retry_count+" 次":"正常";$("modeHint").textContent=s.web_lan_mode?"局域网 · 口令保护":"仅本机访问";
const events=(s.recent_runs||[]).flatMap(r=>parseEvents(r.events).map(e=>`<div class="timeline-item"><span class="timeline-dot bad"></span><div class="timeline-copy"><div class="timeline-title">${esc(r.account_name)} · ${esc(e.type)}</div><div class="timeline-sub">${esc(e.detail||"")} · ${esc(r.finished_at||r.started_at)}</div></div></div>`)).slice(0,12);$("events").innerHTML=events.join("")||`<div class="empty">近期没有风险事件</div>`;
}
function parseEvents(raw){try{const v=typeof raw==="string"?JSON.parse(raw||"[]"):raw;return Array.isArray(v)?v:[]}catch(_){return []}}
async function loadState(){try{const s=await api("/api/state");renderState(s);return true}catch(e){$("sideService").textContent="无法连接后台";$("serviceDot").classList.add("off");$("warnings").innerHTML=`<div class="banner error">${esc(e.message)}</div>`;return false}}
async function loadHistory(){try{const filter=$("histFilter").value;const s=await api("/api/history?limit=50"+(filter?"&status="+encodeURIComponent(filter):""));$("histTable").querySelector("tbody").innerHTML=(s.rows||[]).map(r=>`<tr><td>${esc(formatDate(r.finished_at||r.started_at))}</td><td>${esc(r.account_name)}</td><td>${esc(unk(r.source))}</td><td>${pill(r.status)}${r.error_type?`<div class="small">${esc(r.error_type)}</div>`:""}</td><td>${r.verified?"已核验":"—"}</td><td>${esc(unk(r.score_before))} → ${esc(unk(r.score_after))}</td><td style="max-width:280px;white-space:normal">${esc((r.message||"").slice(0,180))}</td></tr>`).join("")||`<tr><td colspan="7" class="empty">没有符合条件的记录</td></tr>`}catch(e){toast("读取历史失败："+e.message,true)}}
async function loadLogs(){try{const j=await api("/api/logs?lines=250");$("logs").textContent=j.text||"暂无日志"}catch(e){$("logs").textContent="无法读取日志："+e.message}}
async function refreshAll(){await loadState();await Promise.all([loadHistory(),loadLogs()])}
async function setSchedule(){const t=$("f_time").value.trim(),m=$("schedMsg"),b=$("btnSchedule");if(!/^([01]\d|2[0-3]):[0-5]\d$/.test(t)){m.textContent="请选择有效的签到时间";return}b.disabled=true;m.textContent="正在保存计划…";try{const r=await post("/api/schedule",{time:t});m.textContent=r.message+(r.plist_synced===false?" · 后台配置尚未同步，请点 App 顶部的启动/修复":"");toast("签到计划已保存");await loadState()}catch(e){m.textContent="保存失败："+e.message;toast(m.textContent,true)}finally{b.disabled=false}}
async function togglePause(){const b=$("btnPause");b.disabled=true;try{await post("/api/paused",{paused:b.dataset.paused!=="true"});await loadState();toast("计划状态已更新")}catch(e){toast(e.message,true)}finally{b.disabled=false}}
async function runAll(){const b=$("btnRun");b.disabled=true;$("btnRunSchedule").disabled=true;try{await post("/api/run");toast("已提交签到任务，可在概览查看状态");await loadState();await loadHistory()}catch(e){toast(e.message,true)}finally{b.disabled=false;$("btnRunSchedule").disabled=false}}
async function runOne(name){try{await post("/api/run",{account:name});toast("已提交「"+name+"」的签到任务");await loadState()}catch(e){toast(e.message,true)}}
async function verify(name){try{const j=await post("/api/verify",{name});toast(j.ok?"登录态有效 · 金币 "+unk(j.coins)+" · 连续 "+unk(j.streak_days)+" 天":"验证失败："+(j.message||j.error||j.status||"未知原因"),!j.ok)}catch(e){toast(e.message,true)}}
async function toggleEnabled(a){try{await post("/api/accounts/update",{name:a.name,enabled:!a.enabled});toast("账号状态已更新");await loadState()}catch(e){toast(e.message,true)}}
function openDialog(mode,a){dialogMode=mode;editingName=a?a.name:"";$("dialogTitle").textContent=mode==="add"?"添加账号":"编辑账号 · "+editingName;$("dialogSubtitle").textContent=mode==="add"?"输入寓+账号并选择区域，登录成功后自动保存。":"修改账号备注或签到区域。";$("addFields").hidden=mode!=="add";$("editFields").hidden=mode!=="edit";$("dialogSave").textContent=mode==="add"?"保存并登录":"保存修改";$("addMsg").textContent="";if(mode==="add"){$("f_name").value="";$("f_uid").value="";$("f_pwd").value="";$("f_note").value="";$("f_area_manual").value="";$("f_area").innerHTML=`<option value="">请先填写账号 ID</option>`;areaRequestId++}else{$("edit_note").value=a.note||"";$("edit_area").value=a.area_id||""}$("accountOverlay").hidden=false;setTimeout(()=>(mode==="add"?$("f_uid"):$("edit_note")).focus(),10)}
function closeDialog(){if($("dialogSave").disabled)return;$("accountOverlay").hidden=true;$("f_pwd").value="";areaRequestId++}
async function fetchAreas(){const uid=$("f_uid").value.trim(),sel=$("f_area"),requestId=++areaRequestId;if(!uid){sel.innerHTML=`<option value="">请先填写账号 ID</option>`;return}sel.innerHTML=`<option value="">正在获取区域…</option>`;try{const j=await api("/api/areas?user_id="+encodeURIComponent(uid));if(requestId!==areaRequestId)return;const areas=(j.areas||[]).filter(a=>a&&a.AreaID);sel.innerHTML=`<option value="">请选择区域</option>`+areas.map(a=>`<option value="${esc(a.AreaID)}">${esc(a.Name||a.AreaID)}</option>`).join("");if(areas.length===1)sel.value=String(areas[0].AreaID);if(!areas.length)$("addMsg").textContent="服务器没有返回区域，可尝试手动输入 AreaID。"}catch(e){if(requestId!==areaRequestId)return;sel.innerHTML=`<option value="">区域加载失败</option>`;$("addMsg").textContent="获取区域失败："+e.message+"。如已知 AreaID，可手动填写。"}}
async function saveDialog(){const b=$("dialogSave"),m=$("addMsg");b.disabled=true;m.textContent=dialogMode==="add"?"正在验证寓+账号…":"正在保存修改…";try{if(dialogMode==="add"){const user_id=$("f_uid").value.trim(),password=$("f_pwd").value,area_id=$("f_area_manual").value.trim()||$("f_area").value;if(!user_id||!password||!area_id)throw new Error("请填写账号、密码和签到区域");const result=await post("/api/accounts/add",{name:$("f_name").value.trim(),note:$("f_note").value.trim(),user_id,password,area_id});m.textContent="添加成功";$("dialogSave").disabled=false;closeDialog();toast("账号「"+(result.name||user_id)+"」已添加");showView("accounts")}else{const area_id=$("edit_area").value.trim();if(!area_id)throw new Error("签到区域不能为空");await post("/api/accounts/update",{name:editingName,note:$("edit_note").value.trim(),area_id});$("dialogSave").disabled=false;closeDialog();toast("账号信息已更新")}await loadState()}catch(e){m.textContent=e.message;toast("操作失败："+e.message,true)}finally{b.disabled=false}}
function confirmDelete(name){deletingName=name;$("confirmMessage").textContent="确定删除「"+name+"」吗？登录凭证将被删除，历史签到记录仍然保留。";$("confirmError").textContent="";$("confirmOverlay").hidden=false}
function closeConfirm(){if($("confirmDelete").disabled)return;$("confirmOverlay").hidden=true}
async function deleteAccount(){const b=$("confirmDelete");b.disabled=true;try{await post("/api/accounts/delete",{name:deletingName});b.disabled=false;closeConfirm();toast("账号已删除，历史记录保留");await loadState();await loadHistory()}catch(e){$("confirmError").textContent=e.message}finally{b.disabled=false}}
document.addEventListener("click",e=>{const nav=e.target.closest("[data-view]");if(nav){showView(nav.dataset.view);return}const go=e.target.closest("[data-go]");if(go){showView(go.dataset.go);return}const btn=e.target.closest("[data-action]");if(btn){const idx=Number(btn.dataset.idx),a=currentState?.accounts?.[idx];if(!a)return;switch(btn.dataset.action){case "run-one":runOne(a.name);break;case "verify":verify(a.name);break;case "toggle":toggleEnabled(a);break;case "edit":openDialog("edit",a);break;case "delete":confirmDelete(a.name);break}}});
$("btnQuickAccount").addEventListener("click",()=>openDialog("add"));$("btnToggleAdd").addEventListener("click",()=>openDialog("add"));$("dialogClose").addEventListener("click",closeDialog);$("dialogCancel").addEventListener("click",closeDialog);$("dialogSave").addEventListener("click",saveDialog);$("f_uid").addEventListener("change",fetchAreas);$("accountOverlay").addEventListener("click",e=>{if(e.target===$("accountOverlay"))closeDialog()});$("confirmDelete").addEventListener("click",deleteAccount);$("confirmClose").addEventListener("click",closeConfirm);$("confirmCancel").addEventListener("click",closeConfirm);$("confirmOverlay").addEventListener("click",e=>{if(e.target===$("confirmOverlay"))closeConfirm()});document.addEventListener("keydown",e=>{if(e.key==="Escape"){if(!$("confirmOverlay").hidden)closeConfirm();else if(!$("accountOverlay").hidden)closeDialog()}});
$("btnRefresh").addEventListener("click",refreshAll);$("btnSchedule").addEventListener("click",setSchedule);$("btnPause").addEventListener("click",togglePause);$("btnRun").addEventListener("click",runAll);$("btnRunSchedule").addEventListener("click",runAll);$("histFilter").addEventListener("change",loadHistory);$("btnLogs").addEventListener("click",loadLogs);
refreshAll();setInterval(loadState,15000);
</script></body></html>
"""


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
