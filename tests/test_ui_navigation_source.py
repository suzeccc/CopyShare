import unittest

from pathlib import Path


class UiNavigationSourceTests(unittest.TestCase):
    def test_sidebar_uses_buttons_for_all_navigation_items(self) -> None:
        source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")

        for view, label in (
            ("overview", "总览"),
            ("devices", "设备连接"),
            ("history", "剪贴历史"),
            ("settings", "设置"),
        ):
            self.assertIn(f'self._make_nav_button(sidebar, "{view}", "{label}")', source)
        self.assertIn("self._show_view", source)
        self.assertIn("tk.Button", source)
        self.assertIn('cursor="hand2"', source)

    def test_sidebar_navigation_defers_heavy_view_switch_until_idle(self) -> None:
        source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")

        self.assertIn("command=lambda selected=view: self._request_view_switch(selected)", source)
        self.assertIn("def _request_view_switch", source)
        self.assertIn("self._pending_view", source)
        self.assertIn("self.root.after_idle(self._flush_pending_view)", source)
        self.assertIn("def _flush_pending_view", source)


if __name__ == "__main__":
    unittest.main()
