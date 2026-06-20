import json
import unittest

from lan_clipboard_sync.messages import (
    ClipboardMessage,
    content_hash,
    decode_message,
    encode_message,
    make_clipboard_message,
    parse_message,
)


class MessageTests(unittest.TestCase):
    def test_content_hash_is_stable_and_includes_format(self) -> None:
        first = content_hash("hello", "text")
        second = content_hash("hello", "text")
        image_hash = content_hash("hello", "image")

        self.assertEqual(first, second)
        self.assertNotEqual(first, image_hash)
        self.assertEqual(len(first), 64)

    def test_make_clipboard_message_matches_wire_protocol(self) -> None:
        message = make_clipboard_message(
            device_id="device-A",
            content="hello world",
            message_id="msg-1",
            now=lambda: 1710000000.25,
        )

        self.assertIsInstance(message, ClipboardMessage)
        self.assertEqual(message.type, "clipboard")
        self.assertEqual(message.id, "msg-1")
        self.assertEqual(message.device_id, "device-A")
        self.assertEqual(message.timestamp, 1710000000.25)
        self.assertEqual(message.format, "text")
        self.assertEqual(message.content, "hello world")
        self.assertEqual(
            message.to_wire(),
            {
                "type": "clipboard",
                "id": "msg-1",
                "deviceId": "device-A",
                "timestamp": 1710000000.25,
                "format": "text",
                "content": "hello world",
            },
        )

    def test_make_image_clipboard_message_round_trips(self) -> None:
        message = make_clipboard_message(
            device_id="device-A",
            content="iVBORw0KGgo=",
            format="image",
            message_id="image-1",
            now=lambda: 1710000000.5,
        )

        encoded = encode_message(message)
        decoded = decode_message(encoded)

        self.assertEqual(message.format, "image")
        self.assertEqual(json.loads(encoded)["format"], "image")
        self.assertEqual(decoded, message)

    def test_encode_decode_round_trip(self) -> None:
        message = make_clipboard_message(
            device_id="device-A",
            content="hello",
            message_id="msg-1",
            now=lambda: 1710000000.0,
        )

        encoded = encode_message(message)
        decoded = decode_message(encoded)

        self.assertEqual(json.loads(encoded)["deviceId"], "device-A")
        self.assertEqual(decoded, message)

    def test_parse_accepts_ping_and_pong(self) -> None:
        self.assertEqual(parse_message({"type": "ping"}), {"type": "ping"})
        self.assertEqual(parse_message({"type": "pong"}), {"type": "pong"})

    def test_parse_accepts_hello_with_device_id(self) -> None:
        self.assertEqual(parse_message({"type": "hello", "deviceId": "office-laptop"}), {"type": "hello", "deviceId": "office-laptop"})

        with self.assertRaises(ValueError):
            parse_message({"type": "hello", "deviceId": ""})

    def test_parse_rejects_invalid_clipboard_messages(self) -> None:
        with self.assertRaises(ValueError):
            parse_message({"type": "clipboard", "format": "file", "content": "x"})

        with self.assertRaises(ValueError):
            parse_message(
                {
                    "type": "clipboard",
                    "id": "bad-image",
                    "deviceId": "device-A",
                    "timestamp": 1710000000,
                    "format": "image",
                    "content": "not base64!",
                }
            )

        with self.assertRaises(ValueError):
            parse_message(
                {
                    "type": "clipboard",
                    "id": "not-png",
                    "deviceId": "device-A",
                    "timestamp": 1710000000,
                    "format": "image",
                    "content": "aGVsbG8=",
                }
            )

        with self.assertRaises(ValueError):
            decode_message("{not json")


if __name__ == "__main__":
    unittest.main()
