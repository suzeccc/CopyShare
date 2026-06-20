import unittest
from pathlib import Path


class UiInteractivePreviewTests(unittest.TestCase):
    def setUp(self) -> None:
        self.html = Path(".superpowers/brainstorm/ui-stable/index.html").read_text(encoding="utf-8")

    def test_sidebar_navigation_uses_clickable_buttons(self) -> None:
        for view, label in (
            ("overview", "总览"),
            ("devices", "设备连接"),
            ("history", "剪贴历史"),
            ("settings", "设置"),
        ):
            self.assertIn(f'<button class="nav-item" data-view="{view}"', self.html)
            self.assertIn(label, self.html)

        self.assertIn("function switchView", self.html)
        self.assertNotIn('<div class="nav-item', self.html)

    def test_preview_exposes_core_controls(self) -> None:
        for control_id in (
            "openCompact",
            "addPeer",
            "removePeer",
            "clearPeers",
            "clearLogs",
            "copyLogs",
            "saveSettings",
            "discoverDevices",
            "scanQr",
            "compactPause",
            "compactOpen",
        ):
            self.assertIn(f'id="{control_id}"', self.html)

        self.assertIn('const actionId = state.running ? "stopSync" : "startSync"', self.html)
        self.assertIn('id="${actionId}"', self.html)
        self.assertIn("startSync: () => setRunning(true)", self.html)
        self.assertIn("stopSync: () => setRunning(false)", self.html)


if __name__ == "__main__":
    unittest.main()
