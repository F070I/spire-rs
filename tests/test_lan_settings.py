"""Regression tests for persisted LAN toggle, access control and app-independent jobs."""
import base64
import json
import os
import tempfile
import threading
import unittest
import urllib.error
import urllib.request
from unittest import mock

import yujia_signin as y
import web_panel


class LanSettings(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="yujia-lan-test-")
        self.paths = (y.CONFIG_DIR, y.CONFIG_PATH, y.LOG_PATH)
        y.CONFIG_DIR = self.tmp.name
        y.CONFIG_PATH = os.path.join(self.tmp.name, "config.json")
        y.LOG_PATH = os.path.join(self.tmp.name, "signin.log")
        self.old_env = {key: os.environ.get(key) for key in
                        ("YUJIA_WEB_BIND", "YUJIA_WEB_PASSWORD", "YUJIA_WEB_USER", "YUJIA_MANAGED_BY_LAUNCHD")}
        for key in self.old_env:
            os.environ.pop(key, None)
        self.server = None

    def tearDown(self):
        if self.server:
            self.server.shutdown()
            self.server.server_close()
            self.thread.join(timeout=3)
        for key, value in self.old_env.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value
        y.CONFIG_DIR, y.CONFIG_PATH, y.LOG_PATH = self.paths
        self.tmp.cleanup()

    def start(self):
        self.server = web_panel.build_server(0)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.base = "http://127.0.0.1:%d" % self.server.server_address[1]
        _, self.state = self.req("/api/state")
        self.token = self.state["panel_token"]

    def req(self, path, payload=None, auth=None, lan_host=False):
        headers = {}
        if lan_host:
            headers["Host"] = "192.168.20.44"
        if auth:
            headers["Authorization"] = "Basic " + base64.b64encode(auth.encode()).decode()
        if payload is not None:
            headers.update({"Content-Type": "application/json", "X-Panel-Token": self.token})
        request = urllib.request.Request(self.base + path,
                     data=json.dumps(payload).encode() if payload is not None else None, headers=headers)
        try:
            with urllib.request.urlopen(request, timeout=5) as response:
                body = response.read()
                if response.headers.get("Content-Type", "").startswith("application/json"):
                    body = json.loads(body)
                return response.status, body
        except urllib.error.HTTPError as e:
            return e.code, json.loads(e.read())

    def test_default_disabled_and_password_required(self):
        self.start()
        self.assertEqual(self.server.server_address[0], "127.0.0.1")
        self.assertEqual(self.req("/api/network")[1]["enabled"], False)
        status, reply = self.req("/api/network", {"enabled": True, "password": ""})
        self.assertEqual(status, 400)
        self.assertIn("密码", reply["error"])
        status, reply = self.req("/api/network", {"enabled": True, "password": "a"})
        self.assertEqual(status, 400)
        self.assertFalse(os.path.exists(web_panel._network_path()))

    def test_saved_settings_survive_restart_and_app_local_unaffected(self):
        self.start()
        password = "safe-passcode-for-this-app"
        status, res = self.req("/api/network", {"enabled": True, "password": password})
        self.assertEqual(status, 200)
        self.assertTrue(res["restart_required"])
        with open(web_panel._network_path(), encoding="utf-8") as settings_file:
            disk = settings_file.read()
        self.assertNotIn(password, disk)
        self.assertEqual(os.stat(web_panel._network_path()).st_mode & 0o777, 0o600)
        self.assertFalse(self.server.network_config["enabled"])
        self.server.shutdown(); self.server.server_close(); self.thread.join(timeout=2)
        self.server = None
        self.start()
        self.assertEqual(self.server.server_address[0], "0.0.0.0")
        self.assertEqual(self.req("/api/state")[0], 200)  # native app needs no Basic auth
        self.assertEqual(self.req("/api/network", lan_host=True)[0], 401)
        self.assertEqual(self.req("/api/network", auth="panel:incorrect", lan_host=True)[0], 401)
        self.assertEqual(self.req("/api/network", auth="panel:" + password, lan_host=True)[0], 200)
        self.assertEqual(self.req("/api/accounts/add", {"user_id": "fake", "password": "fake"},
                                  auth="panel:" + password, lan_host=True)[0], 400)
        self.assertEqual(self.req("/api/network", {"enabled": False},
                                  auth="panel:" + password, lan_host=True)[0], 403)
        self.assertEqual(self.req("/api/state", lan_host=True)[0], 401)
        status, res = self.req("/api/network", {"enabled": False})
        self.assertEqual(status, 200)
        self.assertFalse(res["enabled"])
        self.assertTrue(web_panel._read_network()["password_hash"])
        self.server.shutdown(); self.server.server_close(); self.thread.join(timeout=2)
        self.server = None
        self.start()
        self.assertEqual(self.server.server_address[0], "127.0.0.1")

    def test_docker_environment_remains_authoritative(self):
        os.environ["YUJIA_WEB_BIND"] = "0.0.0.0"
        os.environ["YUJIA_WEB_PASSWORD"] = "dockerpassword"
        self.start()
        self.assertTrue(self.req("/api/network")[1]["managed_externally"])
        status, data = self.req("/api/network", {"enabled": False, "password": "anything"})
        self.assertEqual(status, 409)
        self.assertFalse(os.path.exists(web_panel._network_path()))
        self.assertEqual(self.req("/api/network", auth="panel:dockerpassword", lan_host=True)[0], 200)

    def test_host_rebinding_denied_and_qr_no_credentials(self):
        web_panel._write_network(True, "passphrase-1234")
        self.start()
        self.assertEqual(self.req("/api/state", lan_host=True)[0], 401)
        # A DNS name must not be accepted as a Host, even with a valid password.
        req = urllib.request.Request(self.base + "/api/state", headers={"Host": "evil.example"})
        with self.assertRaises(urllib.error.HTTPError) as cm:
            urllib.request.urlopen(req, timeout=5)
        self.assertEqual(cm.exception.code, 403)
        with mock.patch.object(web_panel, "_local_addresses", return_value=["192.168.20.14"]):
            status, data = self.req("/api/network")
            self.assertIn("http://192.168.20.14:", data["addresses"][0])
            status, qr = self.req("/api/network/qr")
            self.assertEqual(status, 200)
            self.assertIn(b"<svg", qr)
            self.assertNotIn(b"passphrase-1234", qr)

    def test_managed_server_restarts_only_panel(self):
        self.start()
        os.environ["YUJIA_MANAGED_BY_LAUNCHD"] = "1"
        with mock.patch.object(web_panel, "_restart_panel_after_response", return_value=True) as restart:
            status, res = self.req("/api/network", {"enabled": True, "password": "very-safe-password"})
        self.assertEqual(status, 200)
        self.assertTrue(res["restarting"])
        restart.assert_called_once()


if __name__ == "__main__":
    unittest.main()
