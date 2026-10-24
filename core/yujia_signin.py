#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""寓+（Foxconn yujia）每日自动签到得金币，支持多账号。

从 APK 反编译还原的接口（BASE_URL 默认中国区，可用环境变量 YUJIA_BASE_URL 覆盖）：
  POST {BASE}v2/User/UserLogin2      账号密码登录（密码经 appSecret+GUID+时间戳 SHA256 链混淆）
  POST {BASE}v2/Auth/RefreshToken2   用旧 token 换新 token（Bearer 头带旧 AccessToken）
  POST {BASE}v2/User/AutoUserLogin2  自动登录兜底（AutoLoginToken + CheckCode）
  POST {BASE}v1/AddDetail            签到，body {"storeInfo":"STORE01"}，Code=="ok" 即成功
  GET  {BASE}v1/SignTotalPoints      金币总数/连续签到天数
  GET  {BASE}v1/SignPoints           签到积分流水
  GET  {BASE}v1/User/LoginAreaID     登录前按工号查可选区域

用法：
  python yujia_signin.py login [名称]   登录一个账号并保存（名称缺省用 UserID；重名则更新该账号）
  python yujia_signin.py run [名称...] [--source=manual|scheduled|web|catchup]
                                        签到：无参数=全部启用的账号；与定时/面板共用任务入口与锁
  python yujia_signin.py status [名称...]  查看金币与近期签到流水（实时查询）
  python yujia_signin.py history [名称] [--limit=20] [--status=success]  执行历史（本地记录）
  python yujia_signin.py refresh [名称...] 手动刷新 token
  python yujia_signin.py enable|disable 账号名   启用/停用账号（停用不删除数据）
  python yujia_signin.py list           列出已保存的账号
  python yujia_signin.py serve          启动本机 Web 管理面板（默认 127.0.0.1:8765）

多账号：所有账号保存在 config.json 的 accounts 列表中，逐账号独立刷新 token、独立签到；
run 在任一账号失败时退出码为 1（调度器据此决定当天重试），全部成功为 0。
执行历史保存在 config 目录的 records.db（SQLite），供 CLI history 与 Web 面板查询。
"""

import base64
import ctypes
import ctypes.wintypes
import getpass
import hashlib
import json
import os
import re
import ssl
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

BASE_URL = os.environ.get("YUJIA_BASE_URL", "https://yujia.foxconn.com/api/")
APP_SECRET = "4pms6y3p4e90m9M9eD03E5Hh1pl20cvn"
APP_VERSION = "2.1.16"
STORE_INFO = "STORE01"
LANG = "zh-CN"
DEVICE_SYSTEM = "android"
DEVICE_SYSTEM_VERSION = "12"
DEVICE_TYPE = "AutoSignin-PC"

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))


def _resolve_config_dir():
    """配置/记录目录：环境变量 > 脚本目录（可写时）> 用户数据目录。
    不使用 /data 之类的系统路径作为 macOS 默认值（非 root 不可写）。"""
    env = os.environ.get("YUJIA_CONFIG_DIR")
    if env:
        return env
    if os.access(SCRIPT_DIR, os.W_OK):
        return SCRIPT_DIR
    fallback = os.path.join(os.path.expanduser("~"), "Library", "Application Support",
                            "yujia-signin")
    try:
        os.makedirs(fallback, exist_ok=True)
        print("警告：脚本目录 %s 不可写，配置与记录改用 %s" % (SCRIPT_DIR, fallback))
        return fallback
    except OSError:
        return SCRIPT_DIR


CONFIG_DIR = _resolve_config_dir()
CONFIG_PATH = os.path.join(CONFIG_DIR, "config.json")
LOG_PATH = os.path.join(CONFIG_DIR, "signin.log")

# 日志/记录中禁止出现的敏感信息
_BEARER_RE = re.compile(r"(Bearer\s+)[A-Za-z0-9._\-]+")
_LONGHEX_RE = re.compile(r"\b[0-9a-fA-F]{32,}\b")


def sanitize_text(s):
    """脱敏：Bearer token、长十六进制串（token/AutoLoginToken 等）。"""
    s = _BEARER_RE.sub(r"\1***", str(s))
    return _LONGHEX_RE.sub("***", s)


def log(msg):
    line = time.strftime("[%Y-%m-%d %H:%M:%S] ") + sanitize_text(msg)
    print(line)
    try:
        with open(LOG_PATH, "a", encoding="utf-8") as f:
            f.write(line + "\n")
    except OSError:
        pass


class NetworkError(RuntimeError):
    """网络连接失败或超时（内部重试已耗尽）。"""


class SSLCheckError(RuntimeError):
    """TLS 证书校验失败（全部候选信任源均失败）。"""


# ---------------------------------------------------------------- 配置存储（多账号）

class _DATA_BLOB(ctypes.Structure):
    _fields_ = [("cbData", ctypes.wintypes.DWORD),
                ("pbData", ctypes.POINTER(ctypes.c_char))]


def dpapi_protect(data: bytes) -> bytes:
    blob_in = _DATA_BLOB(len(data), ctypes.cast(
        ctypes.create_string_buffer(data, len(data)), ctypes.POINTER(ctypes.c_char)))
    blob_out = _DATA_BLOB()
    if not ctypes.windll.crypt32.CryptProtectData(
            ctypes.byref(blob_in), None, None, None, None, 0, ctypes.byref(blob_out)):
        raise OSError("CryptProtectData failed")
    try:
        out = ctypes.string_at(blob_out.pbData, blob_out.cbData)
    finally:
        ctypes.windll.kernel32.LocalFree(blob_out.pbData)
    return out


def dpapi_unprotect(data: bytes) -> bytes:
    blob_in = _DATA_BLOB(len(data), ctypes.cast(
        ctypes.create_string_buffer(data, len(data)), ctypes.POINTER(ctypes.c_char)))
    blob_out = _DATA_BLOB()
    if not ctypes.windll.crypt32.CryptUnprotectData(
            ctypes.byref(blob_in), None, None, None, None, 0, ctypes.byref(blob_out)):
        raise OSError("CryptUnprotectData failed")
    try:
        out = ctypes.string_at(blob_out.pbData, blob_out.cbData)
    finally:
        ctypes.windll.kernel32.LocalFree(blob_out.pbData)
    return out


def _encrypt_secrets(secrets):
    """返回可直接写进 JSON 的字段；Windows 用 DPAPI 加密，其他平台明文。"""
    if sys.platform == "win32":
        try:
            return {"secrets_enc": base64.b64encode(
                dpapi_protect(json.dumps(secrets).encode("utf-8"))).decode("ascii")}
        except OSError:
            pass
    return {"secrets": secrets}


def _decrypt_secrets(entry):
    if "secrets_enc" in entry:
        return json.loads(dpapi_unprotect(base64.b64decode(entry["secrets_enc"])).decode("utf-8"))
    return entry.get("secrets", {})


def load_accounts():
    """返回账号列表 [{"name", "public", "secrets"}]；兼容旧版单账号格式。"""
    if not os.path.exists(CONFIG_PATH):
        return []
    with open(CONFIG_PATH, "r", encoding="utf-8") as f:
        raw = json.load(f)
    entries = raw.get("accounts")
    if entries is None:
        if "public" in raw or "secrets_enc" in raw or "secrets" in raw:
            entries = [raw]  # 旧版单账号
        else:
            return []
    accounts = []
    for e in entries:
        try:
            secrets = _decrypt_secrets(e)
        except (OSError, ValueError) as ex:
            log("警告：账号 %s 的 token 解密失败（%s），请重新 login 覆盖该账号"
                % (e.get("name", "?"), ex))
            continue
        public = e.get("public", {})
        accounts.append({
            "name": e.get("name") or public.get("UserID", "?"),
            "public": public,
            "secrets": secrets,
            "note": e.get("note", ""),
            "enabled": bool(e.get("enabled", True)),
        })
    return accounts


def save_accounts(accounts):
    out = {"accounts": [], "v": 2}
    for acc in accounts:
        entry = {
            "name": acc["name"],
            "public": acc.get("public", {}),
            "note": acc.get("note", ""),
            "enabled": bool(acc.get("enabled", True)),
        }
        entry.update(_encrypt_secrets(acc["secrets"]))
        out["accounts"].append(entry)
    os.makedirs(CONFIG_DIR, exist_ok=True)
    with open(CONFIG_PATH, "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, indent=2)
    try:
        os.chmod(CONFIG_PATH, 0o600)  # 非 Windows 下限制为仅属主可读写
    except OSError:
        pass


def select_accounts(accounts, names):
    """按命令行名称过滤；未知名称报错返回 None。"""
    if not names:
        return list(accounts)
    by_name = {a["name"]: a for a in accounts}
    picked, ok = [], True
    for n in names:
        if n in by_name:
            picked.append(by_name[n])
        else:
            log("未知账号：%s（用 list 查看已有账号）" % n)
            ok = False
    return picked if ok else None


# ---------------------------------------------------------------- HTTP

_SSL_STATE = {"ctx": None, "warned": False}


def _is_cert_error(e):
    reason = getattr(e, "reason", None)
    return isinstance(reason, ssl.SSLCertVerificationError) or \
        "CERTIFICATE_VERIFY_FAILED" in str(getattr(e, "reason", e))


def _ssl_contexts():
    """候选信任源，按优先级：显式 CA 文件 → certifi → 系统（Windows 为系统证书库）。"""
    ctxs = []
    cafile = os.environ.get("YUJIA_CAFILE")
    if cafile:
        ctxs.append(("YUJIA_CAFILE=" + cafile, ssl.create_default_context(cafile=cafile)))
    try:
        import certifi
        ctxs.append(("certifi", ssl.create_default_context(cafile=certifi.where())))
    except ImportError:
        pass
    ctxs.append(("系统证书库", ssl.create_default_context()))
    return ctxs


def _open(req, timeout):
    """urlopen：证书校验失败时逐个候选信任源重试，全部失败给出明确解决办法。"""
    if os.environ.get("YUJIA_SSL_INSECURE") == "1":
        if not _SSL_STATE["warned"]:
            log("警告：YUJIA_SSL_INSECURE=1，已跳过 HTTPS 证书校验，仅限确认可信的网络使用")
            _SSL_STATE["warned"] = True
        return urllib.request.urlopen(req, timeout=timeout,
                                      context=ssl._create_unverified_context())
    candidates = None
    if _SSL_STATE["ctx"] is not None:
        try:
            return urllib.request.urlopen(req, timeout=timeout, context=_SSL_STATE["ctx"])
        except urllib.error.URLError as e:
            if not _is_cert_error(e):
                raise
            candidates = _ssl_contexts()  # 缓存的信任源失效（如切换了网络），重新选择
    else:
        candidates = _ssl_contexts()
    last = None
    for name, ctx in candidates:
        try:
            resp = urllib.request.urlopen(req, timeout=timeout, context=ctx)
        except urllib.error.HTTPError as e:
            # 能收到 HTTP 响应说明 TLS 握手已通过（如 401/404），记录信任源后再交给上层处理
            _SSL_STATE["ctx"] = ctx
            log("SSL 证书校验通过（%s）" % name)
            raise
        except urllib.error.URLError as e:
            if not _is_cert_error(e):
                raise
            if last is None:  # 报告最高优先级候选源的失败原因
                last = e
            continue
        _SSL_STATE["ctx"] = ctx
        log("SSL 证书校验通过（%s）" % name)
        return resp
    raise SSLCheckError(
        "HTTPS 证书校验失败：%s\n解决办法（任选其一）：\n"
        "  1) pip3 install certifi —— macOS 官网版 Python 缺系统证书时最常见，装好即自动生效\n"
        "  2) 设环境变量 YUJIA_CAFILE=证书.pem（公司代理/自签 CA 场景）\n"
        "  3) 确认网络可信后设 YUJIA_SSL_INSECURE=1 跳过校验（不推荐）" % last)


def http_json(method, path, body=None, token=None, timeout=30):
    url = BASE_URL + path.lstrip("/")
    headers = {
        "Content-Type": "application/json",
        "lang": LANG,
        "User-Agent": "yujia-signin/1.0",
    }
    if token:
        headers["Authorization"] = "Bearer " + token
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    last_err = None
    for attempt in range(3):
        try:
            with _open(req, timeout=timeout) as resp:
                status = resp.status
                raw = resp.read().decode("utf-8", "replace")
            try:
                return status, json.loads(raw)
            except ValueError:
                # HTTP 成功但响应体不是 JSON（如代理错误页）：交由上层归类为"无法确认状态"
                return status, {"RawBody": raw[:200]}
        except urllib.error.HTTPError as e:
            raw = ""
            try:
                raw = e.read().decode("utf-8", "replace")
            except Exception:
                pass
            try:
                return e.code, json.loads(raw)
            except ValueError:
                return e.code, {"Code": "error", "Message": raw[:200] or ("HTTP %d" % e.code)}
        except (urllib.error.URLError, OSError, ValueError) as e:
            last_err = e
            time.sleep(2 * (attempt + 1))
    raise NetworkError("网络请求失败：%s" % last_err)


# ---------------------------------------------------------------- 密码算法（与 login.js 一致）

def sha256_hex(s):
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def make_user_pwd(password, guid, ts):
    """login.js: m=sha256(pwd); m=sha256(sha256(sha256(m+secret)+guid)+ts)+m+guid+ts"""
    m0 = sha256_hex(password)
    c = sha256_hex(sha256_hex(sha256_hex(m0 + APP_SECRET) + guid) + str(ts))
    return c + m0 + guid + str(ts)


# ---------------------------------------------------------------- 业务动作

def get_areas(user_id):
    code, body = http_json("GET", "v1/User/LoginAreaID?empID=" + urllib.parse.quote(user_id))
    if code == 200 and body.get("Code") == "ok" and body.get("Data"):
        return body["Data"]
    return []


def do_login(user_id, password, area_id):
    """请求登录接口，成功返回 (public, secrets)，失败返回 (None, resp)。"""
    device_uuid = uuid.uuid4().hex
    guid = uuid.uuid4().hex
    ts = int(time.time() * 1000)
    body = {
        "loginType": "UserID",
        "userID": user_id,
        "phone": "",
        "email": "",
        "userPwd": make_user_pwd(password, guid, ts),
        "areaID": area_id,
        "authCode": "",
        "checkCodeID": "",
        "deviceUUID": device_uuid,
        "deviceSystem": DEVICE_SYSTEM,
        "deviceSystemVersion": DEVICE_SYSTEM_VERSION,
        "deviceType": DEVICE_TYPE,
        "appVersion": APP_VERSION,
    }
    code, resp = http_json("POST", "v2/User/UserLogin2", body)
    if code != 200 or resp.get("Code") != "ok" or not resp.get("Data"):
        return None, resp
    d = resp["Data"]
    secrets = {
        "ID": d.get("ID", ""),
        "AccessToken": d.get("AccessToken", ""),
        "RefreshToken": d.get("RefreshToken", ""),
        "ExpiresIn": d.get("ExpiresIn", ""),
        "AutoLoginToken": d.get("AutoLoginToken", ""),
        "CheckCode": d.get("CheckCode", ""),
    }
    public = {
        "UserID": user_id,
        "UserName": d.get("Name", ""),
        "EmpNo": d.get("EmpNo", ""),
        "AreaID": area_id,
        "deviceUUID": device_uuid,
    }
    return (public, secrets), resp


def refresh_token(account):
    secrets = account["secrets"]
    body = {
        "ID": secrets.get("ID", ""),
        "AccessToken": secrets.get("AccessToken", ""),
        "RefreshToken": secrets.get("RefreshToken", ""),
        "ExpiresIn": secrets.get("ExpiresIn", ""),
    }
    code, resp = http_json("POST", "v2/Auth/RefreshToken2", body,
                           token=secrets.get("AccessToken", ""))
    if code != 200 or resp.get("Code") != "ok" or not resp.get("Data"):
        return False, resp
    d = resp["Data"]
    secrets["AccessToken"] = d.get("AccessToken", secrets["AccessToken"])
    secrets["RefreshToken"] = d.get("RefreshToken", secrets["RefreshToken"])
    if d.get("ExpiresIn"):
        secrets["ExpiresIn"] = d["ExpiresIn"]
    return True, resp


def auto_login(account):
    public, secrets = account["public"], account["secrets"]
    body = {
        "userUniqueID": secrets.get("ID", ""),
        "token": secrets.get("AutoLoginToken", ""),
        "checkCode": secrets.get("CheckCode", ""),
        "accessToken": secrets.get("AccessToken", ""),
        "refreshToken": secrets.get("RefreshToken", ""),
        "deviceUUID": public.get("deviceUUID", uuid.uuid4().hex),
        "deviceSystem": DEVICE_SYSTEM,
        "deviceSystemVersion": DEVICE_SYSTEM_VERSION,
        "deviceType": DEVICE_TYPE,
        "appVersion": APP_VERSION,
        "areaID": public.get("AreaID", ""),
    }
    code, resp = http_json("POST", "v2/User/AutoUserLogin2", body,
                           token=secrets.get("AccessToken", ""))
    if code != 200 or resp.get("Code") != "ok" or not resp.get("Data"):
        return False, resp
    d = resp["Data"]
    secrets.update({
        "ID": d.get("ID", secrets.get("ID", "")),
        "AccessToken": d.get("AccessToken", ""),
        "RefreshToken": d.get("RefreshToken", ""),
        "AutoLoginToken": d.get("AutoLoginToken", ""),
        "CheckCode": d.get("CheckCode", ""),
    })
    return True, resp


def sign_in(token):
    return http_json("POST", "v1/AddDetail", {"storeInfo": STORE_INFO}, token=token)


def total_points(token):
    code, resp = http_json("GET", "v1/SignTotalPoints", token=token)
    if code == 200 and resp.get("Code") == "ok" and resp.get("Data"):
        return resp["Data"][0]
    return None


def recent_points(token, size=10):
    code, resp = http_json("GET", "v1/SignPoints?oldtime=m3&page=1&size=%d" % size, token=token)
    if code == 200 and resp.get("Code") == "ok":
        return resp.get("Data") or []
    return None


# ---------------------------------------------------------------- 命令

def cmd_login(args):
    accounts = load_accounts()
    user_id = input("账号/工号(UserID): ").strip()
    if not user_id:
        log("未输入账号，退出")
        return 2
    password = getpass.getpass("密码: ")
    if not password:
        log("未输入密码，退出")
        return 2
    log("查询可选区域…")
    areas = get_areas(user_id)
    if areas:
        for i, a in enumerate(areas):
            log("  %d. %s  (AreaID=%s)" % (i + 1, a.get("Name", "?"), a.get("AreaID", "?")))
        pick = input("选择区域序号（回车=1）: ").strip() or "1"
        try:
            area_id = areas[int(pick) - 1].get("AreaID")
        except (ValueError, IndexError):
            log("序号无效，退出")
            return 2
    else:
        log("区域列表获取失败，改为手动输入 AreaID（例如 GL / LH 等园区代码）")
        area_id = input("AreaID: ").strip()
        if not area_id:
            return 2
    result, resp = do_login(user_id, password, area_id)
    if not result:
        log("登录失败：%s" % json.dumps(resp, ensure_ascii=False)[:500])
        return 1
    public, secrets = result
    name = args[0] if args else user_id
    account = {"name": name, "public": public, "secrets": secrets,
               "note": "", "enabled": True}
    replaced = any(a["name"] == name for a in accounts)
    accounts = [account if a["name"] == name else a for a in accounts] if replaced \
        else accounts + [account]
    save_accounts(accounts)
    log("账号 [%s] 登录成功：%s（工号 %s，AreaID=%s），token 已保存%s"
        % (name, public.get("UserName", "?"), public.get("EmpNo", "?"), area_id,
           "（覆盖更新）" if replaced else "，当前共 %d 个账号" % len(accounts)))
    tp = total_points(secrets["AccessToken"])
    if tp:
        log("当前金币：%s，连续签到：%s 天" % (tp.get("TOTAL", "?"), tp.get("MCOUNT", "?")))
    return 0


def cmd_run(args):
    """统一走 signin_core.run_accounts（与 launchd / Web 面板同一任务入口与同一把锁）。"""
    import signin_core as core
    names = [a for a in args if not a.startswith("--")]
    source = "manual"
    for a in args:
        if a.startswith("--source="):
            source = a.split("=", 1)[1] or source
    summary = core.run_accounts(names, source=source)
    return core.summary_exit_code(summary)


def cmd_status(args):
    accounts = load_accounts()
    if not accounts:
        log("尚未登录任何账号：请先执行  python yujia_signin.py login")
        return 2
    targets = select_accounts(accounts, args)
    if targets is None:
        return 2
    for acc in targets:
        log("账号 [%s]（%s / 工号 %s）" % (acc["name"],
            acc["public"].get("UserName", "?"), acc["public"].get("EmpNo", "?")))
        try:
            tp = total_points(acc["secrets"]["AccessToken"])
        except RuntimeError as e:
            log("  查询失败：%s" % e)
            continue
        if tp:
            log("  金币总数：%s，连续签到：%s 天" % (tp.get("TOTAL", "?"), tp.get("MCOUNT", "?")))
        else:
            log("  汇总查询失败（token 可能已失效，可试 refresh 或重新 login）")
            continue
        items = recent_points(acc["secrets"]["AccessToken"])
        if items:
            log("  近 3 个月签到流水（最新 10 条）：")
            for item in items:
                log("    " + json.dumps(item, ensure_ascii=False))
    return 0


def cmd_refresh(args):
    accounts = load_accounts()
    if not accounts:
        log("尚未登录任何账号：请先执行  python yujia_signin.py login")
        return 2
    targets = select_accounts(accounts, args)
    if targets is None:
        return 2
    all_ok = True
    for acc in targets:
        ok, resp = refresh_token(acc)
        if ok:
            log("账号 [%s] token 刷新成功" % acc["name"])
        else:
            all_ok = False
            ok2, resp2 = auto_login(acc)
            if ok2:
                log("账号 [%s] refresh 失败，AutoUserLogin2 兜底成功" % acc["name"])
            else:
                all_ok = False
                log("账号 [%s] 刷新失败：%s" % (acc["name"],
                    json.dumps(resp, ensure_ascii=False)[:250]))
    save_accounts(accounts)
    return 0 if all_ok else 1


def cmd_list(args):
    accounts = load_accounts()
    if not accounts:
        log("尚未保存任何账号，执行  python yujia_signin.py login [名称]  添加")
        return 2
    log("共 %d 个账号：" % len(accounts))
    for a in accounts:
        log("  [%s]  %s / 工号 %s / AreaID=%s"
            % (a["name"], a["public"].get("UserName", "?"),
               a["public"].get("EmpNo", "?"), a["public"].get("AreaID", "?")))
    return 0


def cmd_history(args):
    """查看执行历史（来自 records.db，与 Web 面板同源）。"""
    import signin_core as core
    names = [a for a in args if not a.startswith("--")]
    limit = 20
    status = None
    for a in args:
        if a.startswith("--limit="):
            try:
                limit = max(1, min(200, int(a.split("=", 1)[1])))
            except ValueError:
                pass
        if a.startswith("--status="):
            status = a.split("=", 1)[1]
    rows = core.query_history(account=names[0] if names else None, status=status, limit=limit)
    if not rows:
        log("暂无执行记录")
        return 0
    for r in rows:
        log("%s  [%s] %-22s %s%s  %s" % (
            r["finished_at"] or r["started_at"], r["source"], r["account_name"],
            r["status"], "" if not r["error_type"] else "(%s)" % r["error_type"],
            (r["message"] or "")[:80]))
    return 0


def cmd_set_enabled(args, enabled):
    import signin_core as core
    if not args:
        log("用法：python yujia_signin.py enable|disable 账号名")
        return 2
    return core.set_account_enabled(args[0], enabled)


def cmd_serve(args):
    """启动本地 Web 管理面板（默认 http://127.0.0.1:8765，仅监听本机）。"""
    import web_panel
    return web_panel.serve(args)


def main():
    argv = sys.argv[1:]
    cmd = argv[0] if argv else "run"
    args = argv[1:]
    handlers = {"login": cmd_login, "run": cmd_run, "status": cmd_status,
                "refresh": cmd_refresh, "list": cmd_list, "history": cmd_history,
                "enable": lambda a: cmd_set_enabled(a, True),
                "disable": lambda a: cmd_set_enabled(a, False),
                "serve": cmd_serve}
    if cmd not in handlers:
        print(__doc__)
        return 2
    try:
        return handlers[cmd](args)
    except RuntimeError as e:
        log(str(e))
        return 1


if __name__ == "__main__":
    sys.exit(main())
