import asyncio
import unittest

from lan_clipboard_sync.websocket import (
    WebSocketFrame,
    encode_text_frame,
    normalize_peer_url,
    read_frame,
)


class WebSocketUrlTests(unittest.TestCase):
    def test_normalize_peer_url_accepts_bare_ip_and_host_port(self) -> None:
        self.assertEqual(normalize_peer_url("192.168.1.8"), "ws://192.168.1.8:8765/")
        self.assertEqual(normalize_peer_url("host.local:9000"), "ws://host.local:9000/")

    def test_normalize_peer_url_preserves_ws_urls(self) -> None:
        self.assertEqual(
            normalize_peer_url("ws://192.168.1.8:9000/clip"),
            "ws://192.168.1.8:9000/clip",
        )

    def test_normalize_peer_url_rejects_secure_websocket_for_mvp(self) -> None:
        with self.assertRaises(ValueError):
            normalize_peer_url("wss://example.test:8765/")


class WebSocketFrameTests(unittest.IsolatedAsyncioTestCase):
    async def test_read_unmasked_server_text_frame(self) -> None:
        reader = asyncio.StreamReader()
        reader.feed_data(encode_text_frame("hello", mask=False))

        frame = await read_frame(reader)

        self.assertEqual(frame, WebSocketFrame(fin=True, opcode=1, payload=b"hello"))

    async def test_read_masked_client_text_frame(self) -> None:
        reader = asyncio.StreamReader()
        reader.feed_data(encode_text_frame("hello", mask=True, mask_key=b"\x01\x02\x03\x04"))

        frame = await read_frame(reader)

        self.assertEqual(frame.payload, b"hello")
        self.assertEqual(frame.opcode, 1)


if __name__ == "__main__":
    unittest.main()
