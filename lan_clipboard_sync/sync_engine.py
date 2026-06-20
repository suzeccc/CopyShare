from __future__ import annotations

import time
import uuid
from typing import Callable

from .clipboard import ClipboardContent
from .messages import ClipboardMessage, content_hash, make_clipboard_message


class SyncEngine:
    def __init__(
        self,
        device_id: str,
        *,
        id_factory: Callable[[], str] | None = None,
        now: Callable[[], float] | None = None,
    ) -> None:
        self.device_id = device_id
        self._id_factory = id_factory or (lambda: str(uuid.uuid4()))
        self._now = now or time.time
        self._seen_message_ids: set[str] = set()
        self._last_local_hash: str | None = None
        self._last_remote_hash: str | None = None
        self._pending_remote_echo_hashes: set[str] = set()

    def observe_local_text(self, text: str) -> ClipboardMessage | None:
        return self.observe_local_content(ClipboardContent(format="text", content=text))

    def observe_local_content(self, content: ClipboardContent) -> ClipboardMessage | None:
        local_hash = content_hash(content.content, content.format)

        if local_hash in self._pending_remote_echo_hashes:
            self._pending_remote_echo_hashes.discard(local_hash)
            self._last_local_hash = local_hash
            return None

        if local_hash == self._last_local_hash:
            return None

        message = make_clipboard_message(
            device_id=self.device_id,
            content=content.content,
            format=content.format,
            message_id=self._id_factory(),
            now=self._now,
        )
        self._last_local_hash = local_hash
        self._seen_message_ids.add(message.id)
        return message

    def apply_remote_message(
        self,
        message: ClipboardMessage,
        write_text: Callable[[str], None] | None = None,
        *,
        write_content: Callable[[ClipboardContent], None] | None = None,
    ) -> bool:
        if message.device_id == self.device_id:
            return False
        if message.id in self._seen_message_ids:
            return False

        self._seen_message_ids.add(message.id)
        text_hash = content_hash(message.content, message.format)
        if text_hash == self._last_remote_hash or text_hash == self._last_local_hash:
            return False

        if write_content is not None:
            write_content(ClipboardContent(format=message.format, content=message.content))
        elif message.format == "text" and write_text is not None:
            write_text(message.content)
        else:
            raise ValueError("a content writer is required for non-text clipboard messages")
        self._last_remote_hash = text_hash
        self._last_local_hash = text_hash
        self._pending_remote_echo_hashes.add(text_hash)
        return True

    def should_suppress_watcher_echo(self, text: str) -> bool:
        return content_hash(text, "text") in self._pending_remote_echo_hashes

    def should_suppress_watcher_echo_content(self, content: ClipboardContent) -> bool:
        return content_hash(content.content, content.format) in self._pending_remote_echo_hashes
