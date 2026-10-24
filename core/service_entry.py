#!/usr/bin/env python3
"""Frozen single executable for both background launchd jobs.

No source path or system Python dependency when packaged with PyInstaller.
"""
import sys
import os

import certifi
import yujia_signin  # noqa: F401
import signin_core  # noqa: F401
import run_daily
import web_panel


def main():
    os.environ.setdefault("SSL_CERT_FILE", certifi.where())
    if "--panel" in sys.argv[1:]:
        return web_panel.serve([])
    if "--if-due" in sys.argv[1:]:
        return run_daily.mode_if_due()
    if "--check" in sys.argv[1:]:
        ok, reason = signin_core.init_storage()
        print("OK" if ok else f"Storage error: {reason}")
        return 0 if ok else 1
    print("YujiaService: expected --panel, --if-due or --check", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
