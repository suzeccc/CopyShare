import unittest
import logging

from lan_clipboard_sync.app import ClipboardSyncApp
from lan_clipboard_sync.clipboard import ClipboardContent, InMemoryClipboard
from lan_clipboard_sync.messages import encode_message, make_clipboard_message


class ListLogHandler(logging.Handler):
    def __init__(self) -> None:
        super().__init__()
        self.records: list[logging.LogRecord] = []

    def emit(self, record: logging.LogRecord) -> None:
        self.records.append(record)

    @property
    def messages(self) -> list[str]:
        return [record.getMessage() for record in self.records]


class ClipboardSyncAppTests(unittest.IsolatedAsyncioTestCase):
    def test_websocket_connections_send_local_device_hello(self) -> None:
        app = ClipboardSyncApp(device_id="office-laptop", port=8765, peers=["192.168.1.20"])
        expected = encode_message({"type": "hello", "deviceId": "office-laptop"})

        self.assertEqual(app.server.hello_message, expected)
        self.assertEqual(app.clients[0].hello_message, expected)

    def make_logger(self) -> tuple[logging.Logger, ListLogHandler]:
        logger = logging.getLogger(f"test.clipboard-sync.{self._testMethodName}")
        logger.handlers.clear()
        logger.setLevel(logging.INFO)
        logger.propagate = False
        handler = ListLogHandler()
        logger.addHandler(handler)
        return logger, handler

    async def test_remote_image_message_is_written_to_clipboard(self) -> None:
        clipboard = InMemoryClipboard("old")
        app = ClipboardSyncApp(device_id="device-A", port=0, clipboard=clipboard)
        rebroadcasts = []

        async def capture_broadcast(message):
            rebroadcasts.append(message)

        app._broadcast_message = capture_broadcast
        message = make_clipboard_message(
            device_id="device-B",
            content="iVBORw0KGgo=",
            format="image",
            message_id="remote-image-1",
            now=lambda: 1710000000.0,
        )

        async def reply(_text: str) -> None:
            raise AssertionError("clipboard messages should not reply directly")

        await app._handle_incoming_text(encode_message(message), reply)

        self.assertEqual(clipboard.read_content(), ClipboardContent(format="image", content="iVBORw0KGgo="))
        self.assertEqual(rebroadcasts, [message])

    async def test_local_image_content_can_be_broadcast(self) -> None:
        clipboard = InMemoryClipboard()
        clipboard.write_content(ClipboardContent(format="image", content="iVBORw0KGgo="))
        app = ClipboardSyncApp(device_id="device-A", port=0, clipboard=clipboard)

        message = app._read_local_clipboard_message()

        self.assertIsNotNone(message)
        self.assertEqual(message.format, "image")
        self.assertEqual(message.content, "iVBORw0KGgo=")

    async def test_local_clipboard_change_logs_clipboard_format(self) -> None:
        logger, handler = self.make_logger()
        clipboard = InMemoryClipboard()
        clipboard.write_content(ClipboardContent(format="image", content="iVBORw0KGgo="))
        app = ClipboardSyncApp(device_id="device-A", port=0, clipboard=clipboard, logger=logger)

        app._read_local_clipboard_message()

        self.assertTrue(any("本机复制图片，准备同步" in message for message in handler.messages))

    async def test_remote_clipboard_change_logs_clipboard_format(self) -> None:
        logger, handler = self.make_logger()
        clipboard = InMemoryClipboard("old")
        app = ClipboardSyncApp(device_id="device-A", port=0, clipboard=clipboard, logger=logger)

        async def ignore_broadcast(_message):
            return None

        app._broadcast_message = ignore_broadcast
        message = make_clipboard_message(
            device_id="device-B",
            content="iVBORw0KGgo=",
            format="image",
            message_id="remote-image-1",
            now=lambda: 1710000000.0,
        )

        async def reply(_text: str) -> None:
            raise AssertionError("clipboard messages should not reply directly")

        await app._handle_incoming_text(encode_message(message), reply)

        self.assertTrue(any("收到远端图片，已写入剪贴板" in message for message in handler.messages))


if __name__ == "__main__":
    unittest.main()
