#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""调度器：launchd 与 Docker 共用，执行逻辑统一走 signin_core（同一把锁、同一套记录）。

用法：
  run_daily.py --if-due   补跑模式（launchd 每 10 分钟触发）：
                          已过当天 SIGNIN_TIME 且仍有启用账号未签 → 执行；否则静默退出。
                          launchd 睡眠错过的触发在唤醒后会继续触发，本模式即明确的补跑策略，
                          并按当天记录去重，避免重复签到。
  run_daily.py            循环模式（Docker 容器内默认）：每天 SIGNIN_TIME 执行一次；
                          启动时若当天已过点且未签 → 立即补跑；失败 30 分钟重试，
                          当天最多 6 次；登录失效/限流等不可重试错误当天不再重试。
  环境变量 YUJIA_WEB=1 时附带启动 Web 面板线程（Docker 用；launchd 方案由独立
  LaunchAgent 常驻面板，两者互不影响）。

退出码：0 成功/已完成/已有任务在跑；1 有账号失败；2 未配置账号；3 环境错误（如目录不可写）。
"""

import datetime
import os
import sys
import threading
import time

import yujia_signin as y
import signin_core as core



def _now():
    return datetime.datetime.now()


def log(msg):
    y.log(msg)


def storage_ready():
    ok, err = core.init_storage()
    if not ok:
        log("环境错误：记录目录不可写（%s），请检查 YUJIA_CONFIG_DIR 指向的目录权限" % err)
        return False
    return True


def run_web_if_enabled():
    if os.environ.get("YUJIA_WEB") == "1":
        def _serve():
            try:
                import web_panel
                web_panel.serve([], foreground=False)
            except Exception as e:
                log("Web 面板线程异常退出：%s" % e)
        threading.Thread(target=_serve, daemon=True).start()


def run_all_pending(source, names=None):
    summary = core.run_accounts(names, source=source)
    return core.summary_exit_code(summary), summary


def mode_if_due():
    """launchd 补跑模式：按持久化计划执行，断网后限速重试且不会重试风控错误。"""
    if not storage_ready():
        return 3
    if core.is_paused():
        return 0  # 定时签到已暂停（面板可恢复）；手动执行不受影响
    h, m = core.signin_time()
    now = _now()
    if (now.hour, now.minute) < (h, m):
        return 0  # 还没到点，静默

    pending = core.pending_accounts()
    if not pending:
        core.clear_scheduler_retry_state()
        return 0  # 今天全部已签，清理昨天/旧的重试状态
    eligible = core.scheduler_pending_accounts()
    if not eligible:
        return 0  # 今天的错误不可自动重试，等人工处理或次日

    retry = core.scheduler_retry_status(now)
    if not retry["ready"]:
        return 0  # 未到下次重试时间或已暂停自动重试

    log("补跑检查：%d 个账号今日可自动处理（计划 %02d:%02d）" %
        (len(eligible), h, m))
    code, summary = run_all_pending("scheduled", [a["name"] for a in eligible])
    retry = core.record_scheduler_result(summary, now=_now())
    if summary.get("retryable"):
        if retry["count"] <= retry["limit"]:
            log("网络类失败：第 %d 次，%d 分钟后自动重试" %
                (retry["count"], core.SCHEDULER_RETRY_INTERVAL_MIN))
        else:
            log("连续网络类失败 %d 次：改为每 %d 小时低频探测，不需要手动重启" %
                (retry["count"], core.SCHEDULER_RETRY_SLOW_INTERVAL_MIN // 60))
    elif not summary.get("all_ok") and not summary.get("busy"):
        log("本次存在不可自动重试的错误，暂停该账号的定时重试，等待人工处理")
    return code


def mode_loop():
    """容器循环模式：自动读面板计划；网络恢复后无需重启容器即可继续重试。"""
    if not storage_ready():
        return 3
    run_web_if_enabled()
    h, m = core.signin_time()
    log("每日签到调度器启动，签到时刻 %02d:%02d（时区 %s）" %
        (h, m, time.tzname))
    while True:
        now = _now()
        if core.is_paused():
            time.sleep(60)  # 定时签到已暂停：Web 面板线程不受影响
            continue
        h, m = core.signin_time()  # 面板改计划后立即采用新值
        target_today = now.replace(hour=h, minute=m, second=0, microsecond=0)
        pending = core.pending_accounts()
        if not pending:
            core.clear_scheduler_retry_state()
            time.sleep(max(1, 60 - now.second))
            continue
        if now < target_today:
            time.sleep(max(1, 60 - now.second))
            continue

        eligible = core.scheduler_pending_accounts()
        if not eligible:
            # 例如登录态失效、服务端限流或业务错误；避免每分钟重复请求。
            time.sleep(60)
            continue
        retry = core.scheduler_retry_status(now)
        if not retry["ready"]:
            time.sleep(max(1, 60 - now.second))
            continue

        if retry["count"]:
            log("自动重试第 %d 次（%d 个待处理账号）" %
                (retry["count"] + 1, len(eligible)))
        code, summary = run_all_pending("scheduled", [a["name"] for a in eligible])
        retry = core.record_scheduler_result(summary, now=_now())
        if summary.get("busy"):
            log("已有任务运行，本次调度让位")
        elif code == 0 and summary.get("all_ok"):
            log("今日自动任务完成")
        elif summary.get("retryable"):
            if retry["count"] <= retry["limit"]:
                log("网络类失败：%d 分钟后自动重试" % core.SCHEDULER_RETRY_INTERVAL_MIN)
            else:
                log("连续网络类失败 %d 次：改为每 %d 小时低频探测" %
                    (retry["count"], core.SCHEDULER_RETRY_SLOW_INTERVAL_MIN // 60))
        elif code == 2:
            log("未配置账号，等待人工 login")
        else:
            log("失败不可自动恢复（登录失效/证书/接口业务错误），已暂停该账号自动重试")
        time.sleep(max(1, 60 - _now().second))


def main():
    if "--if-due" in sys.argv[1:]:
        try:
            return mode_if_due()
        except Exception as e:
            log("补跑模式异常：%s" % e)
            return 3
    try:
        return mode_loop()
    except Exception as e:
        log("调度器异常退出：%s" % e)
        return 3


if __name__ == "__main__":
    sys.exit(main())
