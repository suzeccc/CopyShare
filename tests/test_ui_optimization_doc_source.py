import unittest
from pathlib import Path


class UiOptimizationDocSourceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")
        self.preview = Path(".superpowers/brainstorm/ui-stable/index.html").read_text(encoding="utf-8")

    def test_desktop_ui_matches_optimization_information_architecture(self) -> None:
        for text in ("总览", "设备连接", "剪贴历史", "设置"):
            self.assertIn(text, self.source)

        for marker in (
            "_build_overview_view",
            "_build_status_strip",
            "_build_primary_sync_action",
            "_build_device_cards",
            "_build_activity_stream",
        ):
            self.assertIn(marker, self.source)

    def test_desktop_ui_uses_user_facing_connection_language(self) -> None:
        for text in ("输入对方 IP", "添加设备", "自动发现", "扫码配对"):
            self.assertIn(text, self.source)
        self.assertNotIn("Peer 地址", self.source)

    def test_user_facing_copy_does_not_use_peer_jargon(self) -> None:
        for old_copy in (
            "输入 peer 后点击添加即可更新列表",
            "请先选择要移除的 peer",
        ):
            self.assertNotIn(old_copy, self.source)
            self.assertNotIn(old_copy, self.preview)

    def test_preview_matches_documented_dashboard_structure(self) -> None:
        for marker in (
            "function overviewView",
            "function statusStripHtml",
            "function statusDisplay",
            "function primaryActionHtml",
            "function deviceCardsHtml",
            "function activityStreamHtml",
        ):
            self.assertIn(marker, self.preview)

        for text in ("总览", "剪贴历史", "自动发现", "扫码配对", "同步流"):
            self.assertIn(text, self.preview)

    def test_status_strip_keeps_internal_key_separate_from_display_copy(self) -> None:
        self.assertIn("status_display", self.source)
        self.assertIn("state.status_text", self.source)
        self.assertIn("state.status_key", self.source)
        self.assertIn("disconnected", self.preview)
        self.assertIn("未连接设备", self.preview)


if __name__ == "__main__":
    unittest.main()
