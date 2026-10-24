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
        headers = {}
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
                headers={"Content-Type": "application/json",
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


if __name__ == "__main__":
    unittest.main(verbosity=2)
