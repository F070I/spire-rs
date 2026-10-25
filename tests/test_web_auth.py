#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Web 面板局域网访问模式的验收测试。
运行：python tests/test_web_auth.py -v
"""
import base64
import json
import os
import sys
import tempfile
import threading
import unittest
import urllib.error
import urllib.request
from unittest import mock

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, BASE)
sys.path.insert(0, os.path.join(BASE, "tests"))

import yujia_signin as y      # noqa: E402
import signin_core as core    # noqa: E402
import web_panel              # noqa: E402


class TestLanAccess(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp(prefix="yjweb-")
        y.CONFIG_DIR = self.dir
        y.CONFIG_PATH = os.path.join(self.dir, "config.json")
        y.LOG_PATH = os.path.join(self.dir, "signin.log")
        self._saved = {k: os.environ.get(k) for k in
                       ("YUJIA_WEB_BIND", "YUJIA_WEB_PASSWORD", "YUJIA_WEB_USER")}

    def tearDown(self):
        for k, v in self._saved.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v

    def test_loopback_default_no_password_needed(self):
        os.environ.pop("YUJIA_WEB_BIND", None)
        os.environ.pop("YUJIA_WEB_PASSWORD", None)
        httpd = web_panel.build_server(0)
        self.assertEqual(httpd.server_address[0], "127.0.0.1")
        httpd.server_close()

    def test_lan_without_password_refuses_to_start(self):
        os.environ["YUJIA_WEB_BIND"] = "0.0.0.0"
        os.environ.pop("YUJIA_WEB_PASSWORD", None)
        with self.assertRaises(SystemExit):
            web_panel.build_server(0)

    def _start_lan(self):
        os.environ["YUJIA_WEB_BIND"] = "0.0.0.0"
        os.environ["YUJIA_WEB_PASSWORD"] = "secret-pw"
        os.environ["YUJIA_WEB_USER"] = "panel"
        httpd = web_panel.build_server(0)
        port = httpd.server_address[1]
        t = threading.Thread(target=httpd.serve_forever, daemon=True)
        t.start()
        return httpd, port

    @staticmethod
    def _get(url, auth=None):
        # Simulate a LAN browser's Host header; local loopback access is intentionally passwordless.
        headers = {"Host": "192.168.10.40"}
        if auth:
            headers["Authorization"] = "Basic " + base64.b64encode(
                auth.encode()).decode()
        return urllib.request.urlopen(urllib.request.Request(url, headers=headers),
                                      timeout=5)

    def test_lan_requires_basic_auth(self):
        httpd, port = self._start_lan()
        try:
            with self.assertRaises(urllib.error.HTTPError) as cm:
                self._get("http://127.0.0.1:%d/api/state" % port)
            self.assertEqual(cm.exception.code, 401)
            self.assertIsNotNone(cm.exception.headers.get("WWW-Authenticate"))
            # 错误口令
            with self.assertRaises(urllib.error.HTTPError) as cm:
                self._get("http://127.0.0.1:%d/api/state" % port, auth="panel:wrong")
            self.assertEqual(cm.exception.code, 401)
            # 正确口令
            with self._get("http://127.0.0.1:%d/api/state" % port,
                           auth="panel:secret-pw") as r:
                state = json.loads(r.read().decode())
            self.assertTrue(state["web_lan_mode"])
            self.assertTrue(any("局域网" in w for w in state["warnings"]))
        finally:
            httpd.shutdown()
            httpd.server_close()

    def test_lan_post_still_needs_panel_token(self):
        """局域网口令只解决 GET 可读；写操作仍需面板令牌（双重保护）。"""
        httpd, port = self._start_lan()
        try:
            req = urllib.request.Request(
                "http://127.0.0.1:%d/api/run" % port, data=b"{}", method="POST",
                headers={"Content-Type": "application/json", "Host": "192.168.10.40",
                         "Authorization": "Basic " + base64.b64encode(
                             b"panel:secret-pw").decode()})
            with self.assertRaises(urllib.error.HTTPError) as cm:
                urllib.request.urlopen(req, timeout=5)
            self.assertEqual(cm.exception.code, 403)
        finally:
            httpd.shutdown()
            httpd.server_close()

    def test_web_running_probe_survives_lan_auth(self):
        """局域网模式下健康自检应判定面板存活（回归：401 曾被误判为未运行）。"""
        httpd, port = self._start_lan()
        os.environ["YUJIA_WEB_PORT"] = str(port)
        try:
            self.assertTrue(core._web_running())
        finally:
            os.environ.pop("YUJIA_WEB_PORT", None)
            httpd.shutdown()
            httpd.server_close()


class TestAccountAddResponse(unittest.TestCase):
    """端到端验证：寓+登录失败不是局域网认证失败，成功时仍可保存账号。"""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="yj-add-")
        self.orig_paths = (y.CONFIG_DIR, y.CONFIG_PATH, y.LOG_PATH)
        self.orig_env = {k: os.environ.get(k) for k in ("YUJIA_WEB_BIND", "YUJIA_WEB_PASSWORD")}
        y.CONFIG_DIR = self.tmp.name
        y.CONFIG_PATH = os.path.join(self.tmp.name, "config.json")
        y.LOG_PATH = os.path.join(self.tmp.name, "signin.log")
        os.environ.pop("YUJIA_WEB_BIND", None)
        os.environ.pop("YUJIA_WEB_PASSWORD", None)
        self.httpd = web_panel.build_server(0)
        self.port = self.httpd.server_address[1]
        self.thread = threading.Thread(target=self.httpd.serve_forever, daemon=True)
        self.thread.start()
        with urllib.request.urlopen(self.url("/api/state"), timeout=5) as r:
            self.token = json.load(r)["panel_token"]

    def tearDown(self):
        self.httpd.shutdown()
        self.httpd.server_close()
        self.thread.join(timeout=2)
        y.CONFIG_DIR, y.CONFIG_PATH, y.LOG_PATH = self.orig_paths
        for k, v in self.orig_env.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
        self.tmp.cleanup()

    def url(self, path):
        return "http://127.0.0.1:%s%s" % (self.port, path)

    def add_account(self):
        payload = json.dumps({"name": "testaccount", "user_id": "test-id", "password": "not-real-password",
                              "area_id": "GL"}).encode("utf-8")
        request = urllib.request.Request(self.url("/api/accounts/add"), data=payload, method="POST",
                  headers={"Content-Type": "application/json", "X-Panel-Token": self.token})
        try:
            with urllib.request.urlopen(request, timeout=5) as r:
                return r.status, json.load(r)
        except urllib.error.HTTPError as e:
            return e.code, json.loads(e.read().decode("utf-8"))

    def test_upstream_login_failure_is_422_with_reason_and_no_account(self):
        with mock.patch.object(y, "do_login", return_value=(None, {"Code": "fail", "Message": "账号或密码不正确"})):
            status, body = self.add_account()
        self.assertEqual(status, 422)
        self.assertEqual(body["error_type"], "account_login_failed")
        self.assertIn("账号或密码不正确", body["error"])
        self.assertNotIn("局域网", body["error"])
        self.assertEqual(y.load_accounts(), [])

    def test_upstream_failure_without_message_explained(self):
        with mock.patch.object(y, "do_login", return_value=(None, {})):
            status, body = self.add_account()
        self.assertEqual(status, 422)
        self.assertIn("服务器没有提供具体原因", body["error"])

    def test_successful_login_saves_account(self):
        public = {"UserID": "test-id", "UserName": "Mock Person", "EmpNo": "E123", "AreaID": "GL"}
        secrets = {"AccessToken": "unit-test-token", "RefreshToken": "unit-test-refresh"}
        with mock.patch.object(y, "do_login", return_value=((public, secrets), {"Code": "ok"})):
            status, body = self.add_account()
        self.assertEqual(status, 200)
        self.assertTrue(body["ok"])
        self.assertEqual(len(y.load_accounts()), 1)
        self.assertEqual(y.load_accounts()[0]["public"]["UserID"], "test-id")

    def test_save_failure_is_not_reported_as_login_or_lan_error(self):
        public = {"UserID": "test-id", "AreaID": "GL"}
        with mock.patch.object(y, "do_login", return_value=((public, {"AccessToken": "dummy"}), {"Code": "ok"})), \
             mock.patch.object(y, "save_accounts", side_effect=PermissionError("read-only")):
            status, body = self.add_account()
        self.assertEqual(status, 500)
        self.assertEqual(body["error_type"], "account_save_failed")
        self.assertIn("保存账号失败", body["error"])
        self.assertNotIn("局域网", body["error"])

    def test_frontend_does_not_blindly_replace_401_with_lan_error(self):
        page = web_panel.PAGE
        self.assertNotIn('if(r.status===401){alert("需要登录', page)
        self.assertIn('j.error==="需要登录（局域网访问模式）"', page)


if __name__ == "__main__":
    unittest.main(verbosity=2)
