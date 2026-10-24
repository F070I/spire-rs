"""Static release and product guarantees, runnable on Linux before CI mac build."""
import os
import pathlib
import re
import unittest
import xml.etree.ElementTree as ET

ROOT=pathlib.Path(__file__).resolve().parents[1]

class StandaloneRelease(unittest.TestCase):
    def test_bundled_entry_has_both_execution_modes(self):
        entry=(ROOT/'core/service_entry.py').read_text()
        self.assertIn('"--if-due"',entry)
        self.assertIn('"--panel"',entry)
        self.assertIn('certifi.where()',entry)
        self.assertIn('web_panel.serve',entry)
    def test_native_gui_no_project_selection_or_mac_compilation(self):
        code=(ROOT/'src/main.m').read_text()
        self.assertNotIn('chooseProject',code)
        self.assertNotIn('selectProjectAction',code)
        self.assertNotIn('YujiaProjectPath',code)
        self.assertIn('importLegacyAction',code)
        self.assertIn('resourcePath',code)
        self.assertIn('runtime/YujiaService/YujiaService',code)
    def test_launchd_single_schedule_and_per_user_data(self):
        script=(ROOT/'backend/install_launchd.sh').read_text()
        self.assertNotIn('StartCalendarInterval',script)
        self.assertIn('<integer>600</integer>',script)
        self.assertIn('09:05',script) # only fallback, not independent 09:05 trigger
        self.assertIn('YUJIA_CONFIG_DIR',script)
        self.assertIn('Application Support/yujia-signin',script)
        self.assertNotIn('command -v python3',script)
        self.assertNotIn('/usr/bin/python3',script)
        self.assertIn('plutil -lint',script)
        self.assertIn('bootout',script)
        self.assertIn('bootstrap',script)
        self.assertIn('--if-due',script)
        self.assertIn('--panel',script)
    def test_real_app_executable_not_shell_bootstrap(self):
        plist=ET.parse(ROOT/'src/Info.plist').getroot()
        tree=''.join(plist.itertext())
        self.assertIn('YujiaApp',tree)
        self.assertIn('12.0',tree)
        workflow=(ROOT/'.github/workflows/build-macos.yml').read_text()
        self.assertIn('macos-15-intel',workflow)
        self.assertIn('macos-15',workflow)
        self.assertIn('PyInstaller',workflow)
        self.assertIn('clang',workflow)
        self.assertIn('ditto -c -k',workflow)
    def test_migration_and_uninstall_do_not_delete_history(self):
        migration=(ROOT/'backend/migrate_data.sh').read_text()
        uninstall=(ROOT/'backend/uninstall_launchd.sh').read_text()
        self.assertIn('backup-before-import',migration)
        self.assertIn('records.db-wal',migration)
        self.assertIn('config.json',migration)
        self.assertNotIn('rm -rf "$TARGET"',migration)
        self.assertNotIn('config.json',uninstall)
        self.assertNotIn('records.db',uninstall)
if __name__=='__main__': unittest.main()
