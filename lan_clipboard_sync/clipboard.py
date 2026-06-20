from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import shutil
import subprocess
import sys
from typing import Protocol


@dataclass(frozen=True)
class ClipboardContent:
    format: str
    content: str


class Clipboard(Protocol):
    def read_content(self) -> ClipboardContent:
        ...

    def write_content(self, content: ClipboardContent) -> None:
        ...

    def read_text(self) -> str:
        ...

    def write_text(self, text: str) -> None:
        ...


class ClipboardError(RuntimeError):
    """Raised when system clipboard access fails."""


@dataclass
class InMemoryClipboard:
    text: str = ""
    image: str | None = None
    active_format: str = "text"

    def read_content(self) -> ClipboardContent:
        if self.active_format == "image" and self.image is not None:
            return ClipboardContent(format="image", content=self.image)
        return ClipboardContent(format="text", content=self.text)

    def write_content(self, content: ClipboardContent) -> None:
        if content.format == "image":
            self.write_image(content.content)
            return
        self.write_text(content.content)

    def read_text(self) -> str:
        if self.active_format != "text":
            return ""
        return self.text

    def write_text(self, text: str) -> None:
        self.text = text
        self.image = None
        self.active_format = "text"

    def read_image(self) -> str | None:
        if self.active_format != "image":
            return None
        return self.image

    def write_image(self, image_base64: str) -> None:
        self.image = image_base64
        self.active_format = "image"


class PowerShellClipboard:
    def __init__(self, executable: str | None = None) -> None:
        self.executable = executable or _find_powershell()

    def read_content(self) -> ClipboardContent:
        image = self.read_image()
        if image is not None:
            return ClipboardContent(format="image", content=image)
        return ClipboardContent(format="text", content=self.read_text())

    def write_content(self, content: ClipboardContent) -> None:
        if content.format == "image":
            self.write_image(content.content)
            return
        self.write_text(content.content)

    def read_text(self) -> str:
        completed = self._run(
            "Get-Clipboard -Raw",
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
        if completed.returncode != 0:
            raise ClipboardError(completed.stderr.strip() or "Get-Clipboard failed")
        return completed.stdout

    def write_text(self, text: str) -> None:
        completed = self._run(
            "$input | Set-Clipboard",
            input=text,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
        if completed.returncode != 0:
            raise ClipboardError(completed.stderr.strip() or "Set-Clipboard failed")

    def read_image(self) -> str | None:
        completed = self._run(
            """
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$image = [System.Windows.Forms.Clipboard]::GetImage()
if ($null -eq $image) { exit 2 }
$stream = [System.IO.MemoryStream]::new()
try {
    $image.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
    [Convert]::ToBase64String($stream.ToArray())
} finally {
    $stream.Dispose()
    if ($null -ne $image) { $image.Dispose() }
}
""",
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
            sta=True,
        )
        if completed.returncode == 2:
            return None
        if completed.returncode != 0:
            raise ClipboardError(completed.stderr.strip() or "Get clipboard image failed")
        return completed.stdout.strip()

    def write_image(self, image_base64: str) -> None:
        completed = self._run(
            """
$ImageBase64 = [Console]::In.ReadToEnd()
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bytes = [Convert]::FromBase64String($ImageBase64)
$stream = [System.IO.MemoryStream]::new($bytes)
$image = $null
try {
    $image = [System.Drawing.Image]::FromStream($stream)
    [System.Windows.Forms.Clipboard]::SetImage($image)
} finally {
    if ($null -ne $image) { $image.Dispose() }
    $stream.Dispose()
}
""",
            input=image_base64,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
            sta=True,
        )
        if completed.returncode != 0:
            raise ClipboardError(completed.stderr.strip() or "Set clipboard image failed")

    def _run(self, command: str, *, sta: bool = False, **kwargs: object) -> subprocess.CompletedProcess[str]:
        args = [self.executable, "-NoProfile", "-Command", command]
        if sta and Path(self.executable).stem.lower() == "powershell":
            args.insert(1, "-STA")
        return subprocess.run(args, **kwargs)


def _find_powershell() -> str:
    for candidate in ("powershell", "pwsh"):
        found = shutil.which(candidate)
        if found:
            return found
    raise ClipboardError("PowerShell clipboard commands are not available")


def create_system_clipboard() -> Clipboard:
    if sys.platform.startswith("win"):
        return PowerShellClipboard()
    raise ClipboardError("this MVP currently supports the system clipboard on Windows only")
