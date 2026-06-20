import subprocess
import unittest
from unittest.mock import patch

from lan_clipboard_sync.clipboard import ClipboardContent, InMemoryClipboard, PowerShellClipboard


class ClipboardTests(unittest.TestCase):
    def test_in_memory_clipboard_tracks_active_text_or_image_content(self) -> None:
        clipboard = InMemoryClipboard("hello")

        self.assertEqual(clipboard.read_content(), ClipboardContent(format="text", content="hello"))

        clipboard.write_content(ClipboardContent(format="image", content="iVBORw0KGgo="))
        self.assertEqual(clipboard.read_content(), ClipboardContent(format="image", content="iVBORw0KGgo="))
        self.assertEqual(clipboard.read_image(), "iVBORw0KGgo=")

        clipboard.write_text("back to text")
        self.assertEqual(clipboard.read_content(), ClipboardContent(format="text", content="back to text"))
        self.assertIsNone(clipboard.read_image())

    @patch("lan_clipboard_sync.clipboard.subprocess.run")
    def test_powershell_clipboard_reads_image_before_text(self, run) -> None:
        run.return_value = subprocess.CompletedProcess(
            args=[],
            returncode=0,
            stdout="iVBORw0KGgo=\r\n",
            stderr="",
        )
        clipboard = PowerShellClipboard("powershell")

        content = clipboard.read_content()

        self.assertEqual(content, ClipboardContent(format="image", content="iVBORw0KGgo="))
        command = " ".join(run.call_args.args[0])
        self.assertIn("-STA", command)
        self.assertIn("Clipboard]::GetImage", command)
        self.assertIn("ImageFormat]::Png", command)

    @patch("lan_clipboard_sync.clipboard.subprocess.run")
    def test_powershell_clipboard_falls_back_to_text_when_no_image_exists(self, run) -> None:
        run.side_effect = [
            subprocess.CompletedProcess(args=[], returncode=2, stdout="", stderr=""),
            subprocess.CompletedProcess(args=[], returncode=0, stdout="plain text", stderr=""),
        ]
        clipboard = PowerShellClipboard("powershell")

        self.assertEqual(clipboard.read_content(), ClipboardContent(format="text", content="plain text"))

    @patch("lan_clipboard_sync.clipboard.subprocess.run")
    def test_powershell_clipboard_writes_image_from_base64_png(self, run) -> None:
        run.return_value = subprocess.CompletedProcess(args=[], returncode=0, stdout="", stderr="")
        clipboard = PowerShellClipboard("powershell")

        clipboard.write_content(ClipboardContent(format="image", content="iVBORw0KGgo="))

        call = run.call_args
        command = " ".join(call.args[0])
        self.assertIn("-STA", command)
        self.assertIn("FromBase64String", command)
        self.assertIn("Clipboard]::SetImage", command)
        self.assertEqual(call.kwargs["input"], "iVBORw0KGgo=")


if __name__ == "__main__":
    unittest.main()
