import unittest

from lan_clipboard_sync.clipboard import ClipboardContent, InMemoryClipboard
from lan_clipboard_sync.messages import make_clipboard_message
from lan_clipboard_sync.sync_engine import SyncEngine


class SyncEngineTests(unittest.TestCase):
    def test_local_text_change_creates_one_clipboard_message(self) -> None:
        engine = SyncEngine(
            device_id="device-A",
            id_factory=lambda: "local-1",
            now=lambda: 1710000000.0,
        )

        first = engine.observe_local_text("hello")
        duplicate = engine.observe_local_text("hello")

        self.assertIsNotNone(first)
        self.assertEqual(first.id, "local-1")
        self.assertEqual(first.device_id, "device-A")
        self.assertEqual(first.content, "hello")
        self.assertIsNone(duplicate)

    def test_local_image_change_creates_one_clipboard_message(self) -> None:
        engine = SyncEngine(
            device_id="device-A",
            id_factory=lambda: "image-local-1",
            now=lambda: 1710000000.0,
        )
        content = ClipboardContent(format="image", content="iVBORw0KGgo=")

        first = engine.observe_local_content(content)
        duplicate = engine.observe_local_content(content)

        self.assertIsNotNone(first)
        self.assertEqual(first.id, "image-local-1")
        self.assertEqual(first.format, "image")
        self.assertEqual(first.content, "iVBORw0KGgo=")
        self.assertIsNone(duplicate)

    def test_remote_message_writes_once_and_suppresses_watcher_echo(self) -> None:
        clipboard = InMemoryClipboard("old")
        engine = SyncEngine(device_id="device-A")
        remote = make_clipboard_message(
            device_id="device-B",
            content="remote text",
            message_id="remote-1",
            now=lambda: 1710000000.0,
        )

        self.assertTrue(engine.apply_remote_message(remote, clipboard.write_text))
        self.assertEqual(clipboard.read_text(), "remote text")
        self.assertTrue(engine.should_suppress_watcher_echo("remote text"))
        self.assertIsNone(engine.observe_local_text("remote text"))
        self.assertFalse(engine.apply_remote_message(remote, clipboard.write_text))

    def test_remote_image_message_writes_once_and_suppresses_watcher_echo(self) -> None:
        clipboard = InMemoryClipboard("old")
        engine = SyncEngine(device_id="device-A")
        remote = make_clipboard_message(
            device_id="device-B",
            content="iVBORw0KGgo=",
            format="image",
            message_id="remote-image-1",
            now=lambda: 1710000000.0,
        )

        self.assertTrue(engine.apply_remote_message(remote, write_content=clipboard.write_content))
        self.assertEqual(clipboard.read_content(), ClipboardContent(format="image", content="iVBORw0KGgo="))
        self.assertTrue(engine.should_suppress_watcher_echo_content(clipboard.read_content()))
        self.assertIsNone(engine.observe_local_content(clipboard.read_content()))
        self.assertFalse(engine.apply_remote_message(remote, write_content=clipboard.write_content))

    def test_own_device_messages_are_ignored(self) -> None:
        clipboard = InMemoryClipboard("original")
        engine = SyncEngine(device_id="device-A")
        own = make_clipboard_message(
            device_id="device-A",
            content="echo",
            message_id="own-1",
            now=lambda: 1710000000.0,
        )

        self.assertFalse(engine.apply_remote_message(own, clipboard.write_text))
        self.assertEqual(clipboard.read_text(), "original")

    def test_remote_same_content_with_new_id_is_not_written_again(self) -> None:
        writes: list[str] = []
        engine = SyncEngine(device_id="device-A")
        first = make_clipboard_message(
            device_id="device-B",
            content="same",
            message_id="remote-1",
            now=lambda: 1710000000.0,
        )
        second = make_clipboard_message(
            device_id="device-B",
            content="same",
            message_id="remote-2",
            now=lambda: 1710000001.0,
        )

        self.assertTrue(engine.apply_remote_message(first, writes.append))
        self.assertFalse(engine.apply_remote_message(second, writes.append))
        self.assertEqual(writes, ["same"])


if __name__ == "__main__":
    unittest.main()
