import unittest

from lan_clipboard_sync.ui.controller import SyncUiController


class UiControllerTests(unittest.TestCase):
    def test_start_updates_state_and_normalizes_peers(self) -> None:
        controller = SyncUiController(device_id="device-a")

        controller.start(port=9000, peers=["192.168.1.20"])

        self.assertTrue(controller.state.running)
        self.assertEqual(controller.state.listen_port, 9000)
        self.assertEqual(controller.configured_peers, ["ws://192.168.1.20:9000/"])
        self.assertIn("同步服务已启动", controller.state.logs[0].message)

    def test_pause_and_resume_update_state(self) -> None:
        controller = SyncUiController(device_id="device-a")

        controller.start(port=8765, peers=[])
        controller.pause()
        self.assertTrue(controller.state.paused)
        self.assertEqual(controller.state.status_text, "已暂停")

        controller.resume()
        self.assertFalse(controller.state.paused)
        self.assertEqual(controller.state.status_text, "未连接设备")

    def test_invalid_peer_is_logged_without_starting(self) -> None:
        controller = SyncUiController(device_id="device-a")

        controller.start(port=8765, peers=["wss://example.test"])

        self.assertFalse(controller.state.running)
        self.assertIn("设备地址无效", controller.state.logs[0].message)

    def test_add_peer_uses_current_listen_port(self) -> None:
        controller = SyncUiController(device_id="device-a")
        controller.start(port=9000, peers=[])

        controller.add_peer("office-pc")

        self.assertEqual(controller.configured_peers, ["ws://office-pc:9000/"])
        self.assertEqual(controller.state.peers[0].address, "ws://office-pc:9000/")

    def test_start_uses_preconfigured_peers_when_peer_input_is_empty(self) -> None:
        calls: list[tuple[str, int, list[str]]] = []

        class FakeRuntime:
            def start(self, port: int, peers: list[str]) -> None:
                calls.append(("start", port, peers))

            def stop(self) -> None:
                calls.append(("stop", 0, []))

        controller = SyncUiController(device_id="device-a", runtime=FakeRuntime())
        controller.add_peer("192.168.1.20")

        controller.start(port=8765, peers=[])

        self.assertEqual(calls, [("start", 8765, ["ws://192.168.1.20:8765/"])])

    def test_remove_and_clear_peers_update_state(self) -> None:
        controller = SyncUiController(device_id="device-a")
        controller.add_peer("192.168.1.20")
        controller.add_peer("office-pc")

        self.assertTrue(controller.remove_peer("ws://192.168.1.20:8765/"))
        self.assertEqual(controller.configured_peers, ["ws://office-pc:8765/"])
        self.assertEqual([peer.address for peer in controller.state.peers], ["ws://office-pc:8765/"])

        controller.clear_peers()
        self.assertEqual(controller.configured_peers, [])
        self.assertEqual(controller.state.peers, [])

    def test_set_view_and_clear_logs(self) -> None:
        controller = SyncUiController(device_id="device-a")
        controller.state.add_log("server started")

        self.assertTrue(controller.set_view("history"))
        self.assertEqual(controller.state.active_view, "history")
        self.assertFalse(controller.set_view("unknown"))

        controller.clear_logs()
        self.assertEqual(controller.state.logs, [])

    def test_apply_settings_updates_port_when_stopped(self) -> None:
        controller = SyncUiController(device_id="device-a")

        self.assertTrue(controller.apply_settings(port_text="9000"))
        self.assertEqual(controller.state.listen_port, 9000)
        self.assertIn("设置已保存", controller.state.logs[0].message)

    def test_apply_settings_updates_device_id_when_stopped(self) -> None:
        calls: list[str] = []

        class FakeRuntime:
            def set_device_id(self, device_id: str) -> None:
                calls.append(device_id)

            def start(self, port: int, peers: list[str]) -> None:
                pass

            def stop(self) -> None:
                pass

        controller = SyncUiController(device_id="device-a", runtime=FakeRuntime())

        self.assertTrue(controller.apply_settings(port_text="9000", device_id_text=" office-laptop "))
        self.assertEqual(controller.device_id, "office-laptop")
        self.assertEqual(controller.state.listen_port, 9000)
        self.assertEqual(calls, ["office-laptop"])
        self.assertIn("设备 ID office-laptop", controller.state.logs[0].message)

    def test_apply_settings_rejects_empty_device_id(self) -> None:
        controller = SyncUiController(device_id="device-a")

        self.assertFalse(controller.apply_settings(port_text="9000", device_id_text="   "))
        self.assertEqual(controller.device_id, "device-a")
        self.assertEqual(controller.state.listen_port, 8765)
        self.assertIn("设备 ID 不能为空", controller.state.logs[0].message)

    def test_apply_settings_rejects_invalid_port(self) -> None:
        controller = SyncUiController(device_id="device-a")

        self.assertFalse(controller.apply_settings(port_text="bad"))
        self.assertEqual(controller.state.listen_port, 8765)
        self.assertIn("端口必须是数字", controller.state.logs[0].message)

    def test_stop_clears_running_and_logs(self) -> None:
        controller = SyncUiController(device_id="device-a")
        controller.start(port=8765, peers=[])

        controller.stop()

        self.assertFalse(controller.state.running)
        self.assertFalse(controller.state.paused)
        self.assertEqual(controller.state.status_text, "未启动")
        self.assertIn("同步服务已停止", controller.state.logs[0].message)

    def test_runtime_hook_is_started_and_stopped(self) -> None:
        calls: list[tuple[str, int, list[str]]] = []

        class FakeRuntime:
            def start(self, port: int, peers: list[str]) -> None:
                calls.append(("start", port, peers))

            def stop(self) -> None:
                calls.append(("stop", 0, []))

        controller = SyncUiController(device_id="device-a", runtime=FakeRuntime())
        controller.start(port=9000, peers=["192.168.1.20"])
        controller.stop()

        self.assertEqual(
            calls,
            [
                ("start", 9000, ["ws://192.168.1.20:9000/"]),
                ("stop", 0, []),
            ],
        )

    def test_pause_stops_runtime_and_resume_restarts_runtime(self) -> None:
        calls: list[tuple[str, int, list[str]]] = []

        class FakeRuntime:
            def start(self, port: int, peers: list[str]) -> None:
                calls.append(("start", port, peers))

            def stop(self) -> None:
                calls.append(("stop", 0, []))

        controller = SyncUiController(device_id="device-a", runtime=FakeRuntime())
        controller.start(port=9000, peers=["192.168.1.20"])
        controller.pause()
        controller.resume()

        self.assertEqual(
            calls,
            [
                ("start", 9000, ["ws://192.168.1.20:9000/"]),
                ("stop", 0, []),
                ("start", 9000, ["ws://192.168.1.20:9000/"]),
            ],
        )


if __name__ == "__main__":
    unittest.main()
