#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""验收测试：全部使用本地假 API / 注入异常，不请求真实账号。

运行：python tests/test_signin.py -v
说明：标 STATIC 的检查项在 tests_static_launchd.py / tests_static_docker.py。
"""
import datetime
import json
import os
import sys
import tempfile
import threading
import unittest
import urllib.request

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, BASE)
sys.path.insert(0, os.path.join(BASE, "tests"))

import yujia_signin as y          # noqa: E402
import signin_core as core        # noqa: E402
from fake_api import FakeAPI      # noqa: E402

TODAY = datetime.date.today().isoformat()


class TestBase(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp(prefix="yjt-")
        y.CONFIG_DIR = self.dir
        y.CONFIG_PATH = os.path.join(self.dir, "config.json")
        y.LOG_PATH = os.path.join(self.dir, "signin.log")
        self.api = FakeAPI()
        self.api.start()
        y.BASE_URL = self.api.base_url
        os.environ.pop("YUJIA_SSL_INSECURE", None)

    def tearDown(self):
        self.api.stop()

    def make_accounts(self, *specs):
        """specs: (name, user_id, token)"""
        accounts = []
        for name, uid, tok in specs:
            accounts.append({
                "name": name,
                "public": {"UserID": uid, "UserName": "U-" + uid, "EmpNo": "E-" + uid,
                           "AreaID": "GL", "deviceUUID": "dev-" + uid},
                "secrets": {"ID": "UID-" + uid, "AccessToken": tok, "RefreshToken": "R-" + uid,
                            "ExpiresIn": "7200", "AutoLoginToken": "A-" + uid,
                            "CheckCode": "C-" + uid},
                "note": "", "enabled": True})
        y.save_accounts(accounts)
        return accounts

    def statuses(self, summary):
        return {r["account_name"]: r["status"] for r in summary["results"]}


class TestStatusClassification(TestBase):
    def test_1_success(self):
        self.make_accounts(("accA", "userA", "TOK_userA"))
        s = core.run_accounts(["accA"], source="manual")
        self.assertEqual(self.statuses(s), {"accA": core.ST_SUCCESS})
        rec = s["results"][0]
        self.assertEqual(rec["verified"], 1)
        self.assertEqual(rec["score_after"], "100")
        self.assertEqual(rec["streak_days"], "5")
        self.assertFalse(s["busy"])

    def test_2_already_with_server_message(self):
        self.api.add_mode = "already"
        self.make_accounts(("accA", "userA", "TOK_userA"))
        s = core.run_accounts(["accA"])
        rec = s["results"][0]
        self.assertEqual(rec["status"], core.ST_ALREADY)
        self.assertIn("无法确定执行来源", rec["message"])

    def test_2b_already_via_ledger_when_message_unclear(self):
        """服务端非 ok 且消息未说明 → 用 SignPoints 账本判定今日已签。"""
        self.api.add_mode = "api_error"
        self.api.signed_today = True
        self.make_accounts(("accA", "userA", "TOK_userA"))
        s = core.run_accounts(["accA"])
        rec = s["results"][0]
        self.assertEqual(rec["status"], core.ST_ALREADY)
        self.assertIn("无法确定执行来源", rec["message"])

    def test_2c_api_error_when_ledger_empty(self):
        self.api.add_mode = "api_error"
        self.make_accounts(("accA", "userA", "TOK_userA"))
        s = core.run_accounts(["accA"])
        rec = s["results"][0]
        self.assertEqual(rec["status"], core.ST_API)
        self.assertIn("真实业务错误", rec["message"])

    def test_3_network_error_classification(self):
        y.BASE_URL = "http://127.0.0.1:1/api/"  # 端口拒绝
        self.make_accounts(("accA", "userA", "TOK_userA"))
        s = core.run_accounts(["accA"])
        self.assertEqual(s["results"][0]["status"], core.ST_NETWORK)
        self.assertTrue(s["retryable"])

    def test_3b_ssl_error_classification(self):
        orig = y.sign_in

        def boom(token):
            raise y.SSLCheckError("HTTPS 证书校验失败：self-signed")
        y.sign_in = boom
        try:
            self.make_accounts(("accA", "userA", "TOK_userA"))
            s = core.run_accounts(["accA"])
        finally:
            y.sign_in = orig
        self.assertEqual(s["results"][0]["status"], core.ST_SSL)
        self.assertFalse(s["retryable"])

    def test_3c_verification_failed_on_garbage_body(self):
        self.api.add_mode = "garbage"
        self.make_accounts(("accA", "userA", "TOK_userA"))
        s = core.run_accounts(["accA"])
        self.assertEqual(s["results"][0]["status"], core.ST_UNVERIFIED)

    def test_3d_login_expired_when_recover_fails(self):
        self.make_accounts(("accA", "userA", "TOK_LOST"))
        s = core.run_accounts(["accA"])
        rec = s["results"][0]
        self.assertEqual(rec["status"], core.ST_LOGIN_EXPIRED)
        self.assertEqual(self.api.counts.get("refresh"), 1)   # 不无限重试
        self.assertEqual(self.api.counts.get("auto"), 1)
        self.assertEqual(self.api.counts.get("add_detail"), 1)

    def test_3e_401_recovered_by_refresh(self):
        self.make_accounts(("accA", "userA", "TOK_BAD"))
        s = core.run_accounts(["accA"])
        self.assertEqual(s["results"][0]["status"], core.ST_SUCCESS)
        self.assertEqual(s["results"][0]["retry_count"], 1)
        accounts = y.load_accounts()
        self.assertEqual(accounts[0]["secrets"]["AccessToken"], "TOK_NEW")

    def test_3f_401_recovered_by_auto_login(self):
        self.make_accounts(("accA", "userA", "TOK_DEAD"))
        s = core.run_accounts(["accA"])
        self.assertEqual(s["results"][0]["status"], core.ST_SUCCESS)
        events = [e["type"] for e in s["results"][0]["events"]]
        self.assertIn("token_refresh_failed", events)
        self.assertIn("auto_login_used", events)

    def test_3g_429_stops_retry_and_classified(self):
        self.make_accounts(("accA", "userA", "TOK_429"))
        s = core.run_accounts(["accA"])
        self.assertEqual(s["results"][0]["status"], core.ST_API)
        self.assertEqual(s["results"][0]["error_type"], "http_429")
        self.assertEqual(self.api.counts.get("add_detail"), 1)  # 不盲目重试
        self.assertFalse(s["retryable"])


class TestRecords(TestBase):
    def test_4_failure_recorded(self):
        self.make_accounts(("accA", "userA", "TOK_LOST"))
        core.run_accounts(["accA"])
        rows = core.query_history()
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["status"], core.ST_LOGIN_EXPIRED)

    def test_5_12_history_persists_across_processes(self):
        self.make_accounts(("accA", "userA", "TOK_userA"))
        env = dict(os.environ, YUJIA_CONFIG_DIR=self.dir, YUJIA_BASE_URL=y.BASE_URL)
        import subprocess
        r = subprocess.run([sys.executable, os.path.join(BASE, "core", "yujia_signin.py"), "run"],
                           capture_output=True, text=True, env=env, timeout=60)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        rows = core.query_history()
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["status"], core.ST_SUCCESS)

    def test_10_unwritable_dir_is_visible(self):
        blocker = os.path.join(self.dir, "not-a-dir")
        open(blocker, "w").close()
        y.CONFIG_DIR = blocker
        ok, err = core.init_storage()
        self.assertFalse(ok)
        snap = core.state_snapshot()
        self.assertFalse(snap["storage_ok"])
        self.assertTrue(any("不可写" in w for w in snap["warnings"]))

    def test_10b_run_daily_exits_3_on_unwritable_dir(self):
        import subprocess
        blocker = os.path.join(self.dir, "not-a-dir")
        open(blocker, "w").close()
        env = dict(os.environ, YUJIA_CONFIG_DIR=blocker)
        r = subprocess.run([sys.executable, os.path.join(BASE, "core", "run_daily.py"), "--if-due"],
                           capture_output=True, text=True, env=env, timeout=60)
        self.assertEqual(r.returncode, 3)


class TestLocking(TestBase):
    def test_6_manual_vs_scheduled_not_concurrent(self):
        self.make_accounts(("accA", "userA", "TOK_userA"))
        lock = core.RunLock()
        got, _ = lock.acquire()
        self.assertTrue(got)
        try:
            s = core.run_accounts(["accA"], source="scheduled")
            self.assertTrue(s["busy"])
            self.assertEqual(s["results"], [])
            self.assertEqual(self.api.counts.get("add_detail", 0), 0)
        finally:
            lock.release()

    def test_6b_lock_released_after_run(self):
        self.make_accounts(("accA", "userA", "TOK_userA"))
        core.run_accounts(["accA"])
        self.assertFalse(core.lock_probe())

    def test_6c_exit_code_zero_when_busy(self):
        self.make_accounts(("accA", "userA", "TOK_userA"))
        lock = core.RunLock()
        lock.acquire()
        try:
            self.assertEqual(core.summary_exit_code(
                core.run_accounts(["accA"])), 0)
        finally:
            lock.release()


class TestMultiAccount(TestBase):
    def test_13_two_accounts_distinct_tokens_no_cross_write(self):
        self.make_accounts(("accA", "userA", "TOK_userA"), ("accB", "userB", "TOK_userB"))
        s = core.run_accounts()
        self.assertEqual(self.statuses(s),
                         {"accA": core.ST_SUCCESS, "accB": core.ST_SUCCESS})
        accs = {a["name"]: a for a in y.load_accounts()}
        self.assertEqual(accs["accA"]["secrets"]["AccessToken"], "TOK_userA")
        self.assertEqual(accs["accB"]["secrets"]["AccessToken"], "TOK_userB")

    def test_14_one_fails_other_proceeds(self):
        self.make_accounts(("accA", "userA", "TOK_userA"), ("accB", "userB", "TOK_LOST"))
        s = core.run_accounts()
        st = self.statuses(s)
        self.assertEqual(st["accA"], core.ST_SUCCESS)
        self.assertEqual(st["accB"], core.ST_LOGIN_EXPIRED)
        self.assertFalse(s["all_ok"])
        self.assertEqual(core.summary_exit_code(s), 1)

    def test_15_per_account_history(self):
        self.make_accounts(("accA", "userA", "TOK_userA"), ("accB", "userB", "TOK_userB"))
        core.run_accounts()
        a_rows = core.query_history(account="accA")
        b_rows = core.query_history(account="accB")
        self.assertEqual({r["account_name"] for r in a_rows}, {"accA"})
        self.assertEqual({r["account_name"] for r in b_rows}, {"accB"})

    def test_17_disabled_account_skipped(self):
        self.make_accounts(("accA", "userA", "TOK_userA"), ("accB", "userB", "TOK_userB"))
        core.set_account_enabled("accB", False)
        core.run_accounts()
        self.assertEqual(self.api.counts.get("add_detail"), 1)
        self.assertEqual({r["account_name"] for r in core.query_history()}, {"accA"})

    def test_18_delete_one_keeps_other_and_records(self):
        self.make_accounts(("accA", "userA", "TOK_userA"), ("accB", "userB", "TOK_userB"))
        core.run_accounts()
        self.assertEqual(core.delete_account("accA"), 0)
        names = {a["name"] for a in y.load_accounts()}
        self.assertEqual(names, {"accB"})
        all_rows = core.query_history(limit=50)
        self.assertEqual({r["account_name"] for r in all_rows}, {"accA", "accB"})  # 记录保留

    def test_19_config_backup_on_upgrade(self):
        # 旧版 v1 配置（无 v 字段）→ init_storage 应生成一次性备份
        accs = [{"name": "old", "public": {"UserID": "u1"}, "secrets": {"AccessToken": "T"}}]
        with open(y.CONFIG_PATH, "w", encoding="utf-8") as f:
            json.dump({"accounts": accs}, f)
        core.init_storage()
        backups = [f for f in os.listdir(self.dir) if "upgrade-backup" in f]
        self.assertEqual(len(backups), 1)


class TestStateAndPrivacy(TestBase):
    def test_7_missing_fields_shown_as_unknown(self):
        self.make_accounts(("accA", "userA", "TOK_userA"))
        snap = core.state_snapshot()
        a = snap["accounts"][0]
        self.assertIsNone(a["coins"])          # 前端显示 未知
        self.assertIsNone(a["streak_days"])
        self.assertIsNone(a["last_run_at"])

    def test_8_22_no_secrets_in_records_logs_state_web(self):
        self.make_accounts(("accA", "userA", "TOK_userA"), ("accB", "userB", "TOK_DEAD"))
        core.run_accounts()
        # 记录
        blob = json.dumps(core.query_history(limit=50), ensure_ascii=False)
        for secret in ("TOK_userA", "TOK_userB", "TOK_NEW", "TOK_NEW2", "R-userA", "A-userA"):
            self.assertNotIn(secret, blob)
        # 日志
        with open(y.LOG_PATH, encoding="utf-8") as f:
            logs = f.read()
        for secret in ("TOK_", "RT_NEW", "ALT-", "RT-user"):
            self.assertNotIn(secret, logs)
        # 状态快照
        snap = json.dumps(core.state_snapshot(include_token=False))
        self.assertNotIn("TOK_", snap)
        self.assertNotIn("RT_", snap)

    def test_8b_log_sanitizes_bearer(self):
        y.log("Authorization: Bearer abcdef0123456789abcdef0123456789 done")
        with open(y.LOG_PATH, encoding="utf-8") as f:
            self.assertIn("Bearer ***", f.read())

    def test_9_web_binds_localhost_only(self):
        import web_panel
        httpd = web_panel.build_server(0)
        try:
            host, port = httpd.server_address
            self.assertEqual(host, "127.0.0.1")
            t = threading.Thread(target=httpd.serve_forever, daemon=True)
            t.start()
            with urllib.request.urlopen(
                    "http://127.0.0.1:%d/api/health" % port, timeout=5) as r:
                self.assertEqual(r.status, 200)
        finally:
            httpd.shutdown()
            httpd.server_close()

    def test_22_web_post_requires_panel_token(self):
        import web_panel
        httpd = web_panel.build_server(0)
        port = httpd.server_address[1]
        t = threading.Thread(target=httpd.serve_forever, daemon=True)
        t.start()
        try:
            req = urllib.request.Request("http://127.0.0.1:%d/api/run" % port,
                                         data=b"{}", method="POST",
                                         headers={"Content-Type": "application/json",
                                                  "Host": "127.0.0.1:%d" % port})
            try:
                urllib.request.urlopen(req, timeout=5)
                self.fail("should be rejected")
            except urllib.error.HTTPError as e:
                self.assertEqual(e.code, 403)
        finally:
            httpd.shutdown()
            httpd.server_close()

    def test_11_4_scheduler_state_not_faked_as_healthy(self):
        self.make_accounts(("accA", "userA", "TOK_userA"))
        os.environ["SIGNIN_TIME"] = "00:00"  # 让“已过点”判定不依赖真实时钟
        try:
            snap = core.state_snapshot()
        finally:
            os.environ.pop("SIGNIN_TIME", None)
        self.assertIn(snap["scheduler"]["mode"], ("unknown", "docker-or-other"))
        self.assertTrue(any("未签" in w for w in snap["warnings"]))
        self.assertIsNone(snap["scheduler"]["launchd_loaded"])  # 非 macOS 不假装知道


class TestCatchUp(TestBase):
    def test_pending_accounts_dedupes_by_records(self):
        self.make_accounts(("accA", "userA", "TOK_userA"), ("accB", "userB", "TOK_userB"))
        self.assertEqual({a["name"] for a in core.pending_accounts()}, {"accA", "accB"})
        core.run_accounts(["accA"])
        self.assertEqual({a["name"] for a in core.pending_accounts()}, {"accB"})


if __name__ == "__main__":
    unittest.main(verbosity=2)
