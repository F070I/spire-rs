#!/usr/bin/env python3
"""Keep the source UI and inlined frozen Python web page in sync.

No external runtime assets: PyInstaller embeds the HTML directly in web_panel.py.
Use `python ui/sync_panel.py` after changing ui/dashboard.html.
Use `python ui/sync_panel.py --check` in CI to catch forgotten rebuilds.
"""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
UI = (ROOT / 'ui/dashboard.html').read_text(encoding='utf-8')
PYFILE = ROOT / 'core/web_panel.py'
content = PYFILE.read_text(encoding='utf-8')
a = content.index('PAGE = r"""')
b = content.index('\n\n\ndef build_server(', a)
rendered = content[:a] + 'PAGE = r"""' + UI + '"""' + content[b:]
if '--check' in sys.argv:
    if rendered != content:
        sys.exit('Dashboard template differs from embedded web_panel.py; run python ui/sync_panel.py')
    print('Dashboard in sync')
else:
    PYFILE.write_text(rendered, encoding='utf-8')
    print('Embedded', len(UI), 'characters')
