import unittest

from lan_clipboard_sync.ui.__main__ import build_parser


class UiCliTests(unittest.TestCase):
    def test_parser_defaults_match_ui_design(self) -> None:
        args = build_parser().parse_args([])

        self.assertEqual(args.device_id, "ui-device")
        self.assertEqual(args.port, 8765)
        self.assertEqual(args.peer, [])

    def test_parser_accepts_peers_and_device_id(self) -> None:
        args = build_parser().parse_args(
            ["--device-id", "device-a", "--port", "9000", "--peer", "192.168.1.20"]
        )

        self.assertEqual(args.device_id, "device-a")
        self.assertEqual(args.port, 9000)
        self.assertEqual(args.peer, ["192.168.1.20"])

    def test_help_uses_user_facing_device_language(self) -> None:
        help_text = build_parser().format_help()

        self.assertIn("设备 IP、host:port 或 ws:// URL；可重复传入", help_text)
        self.assertNotIn("peer IP", help_text)


if __name__ == "__main__":
    unittest.main()
