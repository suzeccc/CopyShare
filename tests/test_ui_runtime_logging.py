import logging
import unittest

from lan_clipboard_sync.ui.controller import SyncUiController
from lan_clipboard_sync.ui.tk_app import BackgroundSyncRuntime, SyncDesktopUi


class UiRuntimeLoggingTests(unittest.TestCase):
    def test_runtime_logger_forwards_connection_records_to_ui_log(self) -> None:
        entries: list[tuple[str, str]] = []
        runtime = BackgroundSyncRuntime(device_id="device-a", log=lambda message, level="info": entries.append((message, level)))

        logger = runtime._make_ui_logger()
        logger.info("peer ws://192.168.1.20:8765/ connected")
        logger.error("clipboard watch tick failed: boom")

        self.assertEqual(
            entries,
            [
                ("peer ws://192.168.1.20:8765/ connected", "info"),
                ("clipboard watch tick failed: boom", "error"),
            ],
        )

    def test_runtime_device_id_can_be_updated_before_restart(self) -> None:
        runtime = BackgroundSyncRuntime(device_id="device-a")

        runtime.set_device_id("office-laptop")

        self.assertEqual(runtime.device_id, "office-laptop")

    def test_runtime_state_hint_uses_remote_device_id_for_peer_name(self) -> None:
        controller = SyncUiController(device_id="device-a")
        ui = SyncDesktopUi.__new__(SyncDesktopUi)
        ui.controller = controller

        ui._apply_runtime_state_hint("设备已识别：ws://192.168.1.20:8765/（设备 ID：office-laptop）")

        self.assertEqual(len(controller.state.peers), 1)
        self.assertEqual(controller.state.peers[0].name, "office-laptop")
        self.assertEqual(controller.state.peers[0].address, "ws://192.168.1.20:8765/")
        self.assertEqual(controller.state.peers[0].state, "online")


if __name__ == "__main__":
    unittest.main()
