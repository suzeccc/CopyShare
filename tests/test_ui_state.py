import unittest

from lan_clipboard_sync.ui.state import PeerStatus, SyncUiState


class UiStateTests(unittest.TestCase):
    def test_default_state_matches_stopped_ui(self) -> None:
        state = SyncUiState()

        self.assertFalse(state.running)
        self.assertEqual(state.status_text, "未启动")
        self.assertEqual(state.connected_count, 0)
        self.assertEqual(state.listen_port, 8765)
        self.assertEqual(state.active_view, "overview")

    def test_peer_status_updates_connected_count(self) -> None:
        state = SyncUiState()

        state.upsert_peer(PeerStatus(name="device-a", address="192.168.1.12", state="online"))
        state.upsert_peer(PeerStatus(name="office-pc", address="192.168.1.31", state="reconnecting"))

        self.assertEqual(state.connected_count, 1)
        self.assertEqual(state.peers[0].label, "device-a  192.168.1.12")

    def test_logs_keep_latest_entries_first(self) -> None:
        state = SyncUiState(max_logs=2)

        state.add_log("server started")
        state.add_log("peer connected")
        state.add_log("duplicate suppressed")

        self.assertEqual([entry.message for entry in state.logs], ["duplicate suppressed", "peer connected"])

    def test_running_and_paused_status_text(self) -> None:
        state = SyncUiState()

        state.set_running(True)
        self.assertEqual(state.status_text, "未连接设备")
        self.assertEqual(state.status_key, "disconnected")

        state.upsert_peer(PeerStatus(name="device-a", address="192.168.1.12", state="online"))
        self.assertEqual(state.status_text, "同步中")
        self.assertEqual(state.status_key, "running")

        state.set_paused(True)
        self.assertEqual(state.status_text, "已暂停")
        self.assertEqual(state.status_key, "degraded")

        state.set_running(False)
        self.assertEqual(state.status_text, "未启动")
        self.assertEqual(state.status_key, "stopped")

    def test_active_view_accepts_known_views_only(self) -> None:
        state = SyncUiState()

        self.assertTrue(state.set_active_view("history"))
        self.assertEqual(state.active_view, "history")
        self.assertFalse(state.set_active_view("missing"))
        self.assertEqual(state.active_view, "history")

    def test_clear_logs_removes_all_entries(self) -> None:
        state = SyncUiState()
        state.add_log("server started")
        state.add_log("peer connected")

        state.clear_logs()

        self.assertEqual(state.logs, [])


if __name__ == "__main__":
    unittest.main()
