from __future__ import annotations

import base64
import binascii
from dataclasses import dataclass
import hashlib
import json
import time
import uuid
from typing import Any, Callable


SUPPORTED_CLIPBOARD_FORMATS = {"text", "image"}
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


@dataclass(frozen=True)
class ClipboardMessage:
    id: str
    device_id: str
    timestamp: float
    format: str
    content: str
    type: str = "clipboard"

    def to_wire(self) -> dict[str, Any]:
        return {
            "type": self.type,
            "id": self.id,
            "deviceId": self.device_id,
            "timestamp": self.timestamp,
            "format": self.format,
            "content": self.content,
        }


ProtocolMessage = ClipboardMessage | dict[str, str]


def content_hash(content: str, format: str = "text") -> str:
    data = f"{format}\0{content}".encode("utf-8")
    return hashlib.sha256(data).hexdigest()


def make_clipboard_message(
    device_id: str,
    content: str,
    *,
    format: str = "text",
    message_id: str | None = None,
    now: Callable[[], float] | None = None,
) -> ClipboardMessage:
    validate_clipboard_content(format, content)
    clock = now or time.time
    return ClipboardMessage(
        id=message_id or str(uuid.uuid4()),
        device_id=device_id,
        timestamp=clock(),
        format=format,
        content=content,
    )


def parse_message(raw: Any) -> ProtocolMessage:
    if not isinstance(raw, dict):
        raise ValueError("message must be a JSON object")

    message_type = raw.get("type")
    if message_type in {"ping", "pong"}:
        return {"type": message_type}
    if message_type == "hello":
        device_id = raw.get("deviceId")
        if not isinstance(device_id, str) or not device_id.strip():
            raise ValueError("hello deviceId must be a non-empty string")
        return {"type": "hello", "deviceId": device_id.strip()}

    if message_type != "clipboard":
        raise ValueError(f"unsupported message type: {message_type!r}")

    missing = [key for key in ("id", "deviceId", "timestamp", "format", "content") if key not in raw]
    if missing:
        raise ValueError(f"clipboard message missing fields: {', '.join(missing)}")

    message_id = raw["id"]
    device_id = raw["deviceId"]
    format = raw["format"]
    content = raw["content"]
    if not isinstance(message_id, str) or not message_id:
        raise ValueError("clipboard message id must be a non-empty string")
    if not isinstance(device_id, str) or not device_id:
        raise ValueError("clipboard deviceId must be a non-empty string")
    validate_clipboard_content(format, content)

    try:
        timestamp = float(raw["timestamp"])
    except (TypeError, ValueError) as exc:
        raise ValueError("clipboard timestamp must be numeric") from exc

    return ClipboardMessage(
        id=message_id,
        device_id=device_id,
        timestamp=timestamp,
        format=format,
        content=content,
    )


def validate_clipboard_content(format: Any, content: Any) -> None:
    if not isinstance(format, str) or format not in SUPPORTED_CLIPBOARD_FORMATS:
        raise ValueError(f"unsupported clipboard format: {format!r}")
    if not isinstance(content, str):
        raise ValueError("clipboard content must be text")
    if format == "image":
        try:
            image_bytes = base64.b64decode(content.encode("ascii"), validate=True)
        except (UnicodeEncodeError, binascii.Error) as exc:
            raise ValueError("image clipboard content must be base64 PNG data") from exc
        if not image_bytes.startswith(PNG_SIGNATURE):
            raise ValueError("image clipboard content must be base64 PNG data")


def encode_message(message: ProtocolMessage) -> str:
    if isinstance(message, ClipboardMessage):
        payload: dict[str, Any] = message.to_wire()
    else:
        payload = message
    return json.dumps(payload, ensure_ascii=False, separators=(",", ":"))


def decode_message(data: str) -> ProtocolMessage:
    try:
        raw = json.loads(data)
    except json.JSONDecodeError as exc:
        raise ValueError("message is not valid JSON") from exc
    return parse_message(raw)
