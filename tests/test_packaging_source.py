import unittest
from pathlib import Path


class PackagingSourceTests(unittest.TestCase):
    def test_windows_packaging_entrypoint_uses_desktop_ui(self) -> None:
        entrypoint = Path("lan_clipboard_sync_ui.py").read_text(encoding="utf-8")

        self.assertIn("from lan_clipboard_sync.ui.__main__ import main", entrypoint)
        self.assertIn("raise SystemExit(main())", entrypoint)

    def test_windows_packaging_script_builds_windowed_executable(self) -> None:
        script = Path("scripts/package_windows.ps1").read_text(encoding="utf-8")

        self.assertIn("python -m PyInstaller", script)
        self.assertIn("--onefile", script)
        self.assertIn("--windowed", script)
        self.assertIn("lan_clipboard_sync_ui.py", script)
        self.assertIn("LanClipboardSync", script)
        self.assertIn("StopRunning", script)


if __name__ == "__main__":
    unittest.main()
