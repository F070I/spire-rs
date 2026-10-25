"""Real HTTP integration tests for login, schedules and the embedded dashboard.
Uses local FakeAPI; no credential, external internet or real check-in needed.
"""
import datetime
import json
import os
import sys
import pathlib
import tempfile
import threading
import unittest
import urllib.error
import urllib.request
from unittest import mock

import yujia_signin as y
import signin_core as core
import run_daily
import web_panel
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from fake_api import FakeAPI


class PanelIntegration(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="yujia-panel-regression-")
        self.saved = (y.CONFIG_DIR, y.CONFIG_PATH, y.LOG_PATH, y.BASE_URL)
        self.saved_env = {name: os.environ.get(name) for name in
                          ("YUJIA_WEB_BIND", "YUJIA_WEB_PASSWORD", "SIGNIN_TIME")}
        for name in ("YUJIA_WEB_BIND", "YUJIA_WEB_PASSWORD", "SIGNIN_TIME"):
            os.environ.pop(name, None)
        y.CONFIG_DIR = self.tmp.name
        y.CONFIG_PATH = os.path.join(self.tmp.name, "config.json")
        y.LOG_PATH = os.path.join(self.tmp.name, "signin.log")
        self.upstream = FakeAPI()
        self.upstream.start()
        y.BASE_URL = self.upstream.base_url
        self.panel = web_panel.build_server(0)
        self.thread = threading.Thread(target=self.panel.serve_forever, daemon=True)
        self.thread.start()
        self.base = "http://127.0.0.1:%d" % self.panel.server_address[1]
        _, state = self.request("/api/state")
        self.token = state["panel_token"]

    def tearDown(self):
        self.panel.shutdown()
        self.panel.server_close()
        self.thread.join(timeout=3)
        self.upstream.stop()
        y.CONFIG_DIR, y.CONFIG_PATH, y.LOG_PATH, y.BASE_URL = self.saved
        for k, v in self.saved_env.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
        self.tmp.cleanup()

    def request(self, path, data=None):
        headers = {}
        if data is not None:
            headers = {"Content-Type": "application/json", "X-Panel-Token": self.token}
        req = urllib.request.Request(self.base + path, headers=headers,
                                     data=json.dumps(data).encode() if data is not None else None)
        try:
            with urllib.request.urlopen(req, timeout=6) as response:
                return response.status, json.load(response)
        except urllib.error.HTTPError as e:
            return e.code, json.loads(e.read().decode())

    def test_real_login_pipeline_area_auth_and_persistence(self):
        code, regions = self.request("/api/areas?user_id=real-flow")
        self.assertEqual(code, 200)
        self.assertEqual(regions["areas"][0]["AreaID"], "GL")
        code, result = self.request("/api/accounts/add", {
            "name": "MacAccount", "user_id": "real-flow", "password": "sample-only",
            "area_id": "GL", "note": "从 App 添加"})
        self.assertEqual((code, result.get("ok")), (200, True))
        self.assertEqual(self.upstream.counts["login"], 1)
        accs = y.load_accounts()
        self.assertEqual(len(accs), 1)
        self.assertEqual(accs[0]["public"]["AreaID"], "GL")
        self.assertEqual(accs[0]["secrets"]["AccessToken"], "TOK_real-flow")
        code, state = self.request("/api/state")
        self.assertEqual(len(state["accounts"]), 1)
        self.assertNotIn("TOK_real-flow", json.dumps(state))

    def test_actual_upstream_reject_stays_account_error_without_persist(self):
        self.upstream.login_ok = False
        code, err = self.request("/api/accounts/add", {
            "user_id": "real-flow", "password": "bad", "area_id": "GL"})
        self.assertEqual(code, 422)
        self.assertEqual(err.get("error_type"), "account_login_failed")
        self.assertIn("wrong password", err["error"])
        self.assertNotIn("局域网", err["error"])
        self.assertEqual(y.load_accounts(), [])

    def test_new_schedule_prevents_old_nine_oclock_execution(self):
        code, response = self.request("/api/schedule", {"time": "14:37"})
        self.assertEqual(code, 200)
        self.assertEqual(response["signin_time"], "14:37")
        self.assertEqual(core.signin_time(), (14, 37))
        with mock.patch.object(run_daily, "_now", return_value=datetime.datetime(2026, 10, 25, 9, 6)), \
             mock.patch.object(core, "pending_accounts", side_effect=AssertionError("Early execution!")):
            self.assertEqual(run_daily.mode_if_due(), 0)
        code, state = self.request("/api/state")
        self.assertEqual(state["scheduler"]["schedule_source"], "panel")
        self.assertEqual(state["scheduler"]["signin_time"], "14:37")


    def test_periodic_launch_agent_needs_no_reload_when_plan_changes(self):
        """No launchctl restart on schedule edit; SQLite is authoritative."""
        plist = pathlib.Path(self.tmp.name) / "com.yujia.signin.plist"
        plist.write_text("<plist><key>StartInterval</key><integer>600</integer>"
                         "<key>SIGNIN_TIME</key><string>09:05</string></plist>")
        with mock.patch.object(web_panel.os.path, "expanduser", return_value=str(plist)), \
             mock.patch.object(web_panel.subprocess, "run", side_effect=AssertionError("Agent restarted!")):
            self.assertTrue(web_panel._update_plist_time("14:37", platform="darwin"))
        self.assertIn("09:05", plist.read_text())  # unused fallback, not a second trigger

    def test_json_body_rejects_non_object_cleanly(self):
        req = urllib.request.Request(self.base + "/api/accounts/add", data=b"[]",
                      method="POST", headers={"Content-Type": "application/json",
                                              "X-Panel-Token": self.token})
        with self.assertRaises(urllib.error.HTTPError) as e:
            urllib.request.urlopen(req, timeout=6)
        self.assertEqual(e.exception.code, 400)

    def test_web_page_contains_complete_new_ui_no_scripts_or_secrets_leaked(self):
        with urllib.request.urlopen(self.base + "/", timeout=6) as r:
            html = r.read().decode()
        self.assertIn("data-view=\"accounts\"", html)
        self.assertIn("id=\"accountOverlay\"", html)
        self.assertIn("id=\"f_area_manual\"", html)
        self.assertIn('j.error==="需要登录（局域网访问模式）"', html)
        self.assertNotIn("prompt(\"修改备注", html)
        self.assertNotIn("TOK_real-flow", html)
        self.assertNotIn("__PORT__", html)


if __name__ == "__main__":
    unittest.main()
