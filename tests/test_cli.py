import unittest

from lan_clipboard_sync.__main__ import build_parser


class CliTests(unittest.TestCase):
    def test_parser_defaults_match_design(self) -> None:
        args = build_parser().parse_args([])

        self.assertEqual(args.host, "0.0.0.0")
        self.assertEqual(args.port, 8765)
        self.assertEqual(args.peer, [])
        self.assertEqual(args.poll_interval, 0.1)
        self.assertEqual(args.reconnect_delay, 2.0)

    def test_parser_accepts_multiple_manual_peers(self) -> None:
        args = build_parser().parse_args(
            ["--peer", "192.168.1.8", "--peer", "ws://host.local:9000/clip"]
        )

        self.assertEqual(args.peer, ["192.168.1.8", "ws://host.local:9000/clip"])

    def test_help_uses_user_facing_device_language(self) -> None:
        help_text = build_parser().format_help()

        self.assertIn("设备 IP、host:port 或 ws:// URL；可重复传入", help_text)
        self.assertIn("对端设备断开后的重连等待秒数", help_text)
        self.assertNotIn("unavailable peer", help_text)


if __name__ == "__main__":
    unittest.main()
