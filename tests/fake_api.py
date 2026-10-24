#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""可编程假 API（进程内线程，端口随机），用于不打真实接口的端到端测试。

token 行为约定：
  TOK_<userID>  正常 token（登录返回 TOK_+userID，各账号互不相同）
  TOK_BAD       认证接口 401，RefreshToken2 正常换发 TOK_NEW
  TOK_DEAD      401，RefreshToken2 拒绝，AutoUserLogin2 成功换 TOK_NEW2
  TOK_LOST      401，刷新与自动登录全部拒绝（→ login_expired）
  TOK_429/403   AddDetail 直接返回对应状态码（限流/拒绝）
"""
import datetime
import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

USER_TPL = {"ID": "UID-{u}", "Name": "测试{u}", "EmpNo": "E-{u}", "AreaID": "GL",
            "UserID": "{u}", "AutoLoginToken": "ALT-{u}", "CheckCode": "CC-{u}"}


class FakeAPI:
    def __init__(self):
        self.counts = {}
        self.add_mode = "ok"        # ok | already | api_error | garbage
        self.signed_today = False   # SignPoints 账本中是否有今日记录
        self.login_ok = True
        self.server = None
        self.thread = None

    # ---- 生命周期 ----
    def start(self):
        api = self
        today = datetime.date.today().isoformat()

        class H(BaseHTTPRequestHandler):
            def _send(self, code, obj):
                b = json.dumps(obj).encode("utf-8")
                self.send_response(code)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(b)))
                self.end_headers()
                self.wfile.write(b)

            def _bump(self, k):
                api.counts[k] = api.counts.get(k, 0) + 1

            def _tok(self):
                return (self.headers.get("Authorization") or "")[7:]

            def do_GET(self):
                t = self._tok()
                if "SignTotalPoints" in self.path:
                    self._bump("total_points")
                    return self._send(200, {"Code": "ok",
                                            "Data": [{"TOTAL": "100", "MCOUNT": "5"}]})
                if "SignPoints" in self.path:
                    self._bump("sign_points")
                    data = []
                    if api.signed_today:
                        data = [{"Time": today + " 09:05:00", "Explain": "每日签到",
                                 "Count": "5"}]
                    return self._send(200, {"Code": "ok", "Data": data})
                if "LoginAreaID" in self.path:
                    return self._send(200, {"Code": "ok",
                                            "Data": [{"AreaID": "GL", "Name": "觀瀾園區"}]})
                return self._send(404, {"Code": "error"})

            def do_POST(self):
                n = int(self.headers.get("Content-Length") or 0)
                body = self.rfile.read(n)
                t = self._tok()
                p = self.path
                if p.endswith("AutoUserLogin2"):
                    self._bump("auto")
                    if t == "TOK_LOST":
                        return self._send(200, {"Code": "error", "Message": "auto rejected"})
                    d = dict((k, v.format(u="rec")) for k, v in USER_TPL.items())
                    d["AccessToken"], d["RefreshToken"] = "TOK_NEW2", "RT_NEW2"
                    return self._send(200, {"Code": "ok", "Data": d})
                if p.endswith("UserLogin2"):
                    self._bump("login")
                    if not api.login_ok:
                        return self._send(200, {"Code": "error", "Message": "wrong password"})
                    try:
                        u = json.loads(body).get("userID", "u")
                    except ValueError:
                        u = "u"
                    d = dict((k, v.format(u=u)) for k, v in USER_TPL.items())
                    d["AccessToken"], d["RefreshToken"] = "TOK_" + u, "RT_" + u
                    return self._send(200, {"Code": "ok", "Data": d})
                if p.endswith("RefreshToken2"):
                    self._bump("refresh")
                    if t in ("TOK_DEAD", "TOK_LOST"):
                        return self._send(200, {"Code": "error", "Message": "refresh rejected"})
                    return self._send(200, {"Code": "ok", "Data": {
                        "AccessToken": "TOK_NEW", "RefreshToken": "RT_NEW",
                        "ExpiresIn": 7200}})
                # 注意：AutoUserLogin2 必须在 UserLogin2 之前判断（后缀包含关系）
                if p.endswith("AddDetail"):
                    self._bump("add_detail")
                    if t == "TOK_429":
                        return self._send(429, {"Code": "error", "Message": "too many"})
                    if t == "TOK_403":
                        return self._send(403, {"Code": "error", "Message": "forbidden"})
                    if t in ("TOK_BAD", "TOK_DEAD", "TOK_LOST"):
                        return self._send(401, {"Code": "error", "Message": "unauthorized"})
                    if api.add_mode == "ok":
                        api.signed_today = True
                        return self._send(200, {"Code": "ok", "Data": {"Points": "5"}})
                    if api.add_mode == "already":
                        return self._send(200, {"Code": "fail",
                                                "Message": "今日已签到，请明日再来"})
                    if api.add_mode == "api_error":
                        return self._send(200, {"Code": "fail", "Message": "Invalid store"})
                    if api.add_mode == "garbage":
                        return self._html_garbage()
                    api.signed_today = True
                    return self._send(200, {"Code": "ok", "Data": {"Points": "5"}})
                return self._send(404, {"Code": "error"})

            def _html_garbage(self):
                b = b"<html>gateway error</html>"
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.send_header("Content-Length", str(len(b)))
                self.end_headers()
                self.wfile.write(b)

            def log_message(self, *a):
                pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), H)
        self.server.daemon_threads = True
        self.port = self.server.server_address[1]
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def stop(self):
        if self.server:
            self.server.shutdown()
            self.server.server_close()
            self.thread.join(timeout=3)
            self.server = None

    @property
    def base_url(self):
        return "http://127.0.0.1:%d/api/" % self.port
