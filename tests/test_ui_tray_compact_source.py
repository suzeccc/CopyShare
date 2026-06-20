import unittest
from pathlib import Path


class UiTrayCompactSourceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")
        self.preview = Path(".superpowers/brainstorm/ui-stable/index.html").read_text(encoding="utf-8")

    def test_desktop_compact_window_matches_reference_controls(self) -> None:
        for text in ("托盘小浮窗", "剪贴板同步", "已连接设备", "最近延迟", "监听端口", "暂停", "打开面板"):
            self.assertIn(text, self.source)

        for marker in (
            "_build_compact_window_shell",
            "_compact_metric_row",
            "_refresh_compact",
            "_open_main_from_compact",
        ):
            self.assertIn(marker, self.source)

    def test_minimizing_main_window_enters_compact_mode(self) -> None:
        self.assertIn('self.root.bind("<Unmap>"', self.source)
        self.assertIn("_handle_main_window_unmap", self.source)
        self.assertIn("_enter_compact_mode", self.source)
        self.assertIn("self.root.withdraw()", self.source)
        self.assertIn("self.root.deiconify()", self.source)
        self.assertRegex(self.source, r"compact_window\.(withdraw|destroy)\(")

    def test_preview_explains_tray_compact_behavior(self) -> None:
        for text in (
            "托盘小浮窗",
            "已连接设备",
            "最近延迟",
            "监听端口",
            "暂停",
            "打开面板",
        ):
            self.assertIn(text, self.preview)

        self.assertIn("function compactFloatingHtml", self.preview)
        self.assertIn('id="compactPanel"', self.preview)
        self.assertIn('id="openCompact"', self.preview)


if __name__ == "__main__":
    unittest.main()
