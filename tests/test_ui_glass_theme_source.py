import unittest
from pathlib import Path


class UiGlassThemeSourceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")

    def test_desktop_ui_defines_glass_theme_tokens(self) -> None:
        self.assertIn("GLASS_COLORS", self.source)
        self.assertIn("apply_window_backdrop", self.source)
        self.assertIn("GlassRoot.TFrame", self.source)
        self.assertIn("GlassPanel.TFrame", self.source)

    def test_sidebar_navigation_is_visibly_clickable(self) -> None:
        self.assertIn('cursor="hand2"', self.source)
        self.assertIn("GlassNavButton", self.source)
        for view, label in (
            ("overview", "总览"),
            ("devices", "设备连接"),
            ("history", "剪贴历史"),
            ("settings", "设置"),
        ):
            self.assertIn(f'self._make_nav_button(sidebar, "{view}", "{label}")', self.source)


if __name__ == "__main__":
    unittest.main()
