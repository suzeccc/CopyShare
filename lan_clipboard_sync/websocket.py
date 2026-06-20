from __future__ import annotations

import asyncio
from collections.abc import Awaitable, Callable
from dataclasses import dataclass
import base64
import hashlib
import json
import logging
import os
from types import TracebackType
from urllib.parse import ParseResult, urlparse, urlunparse


GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"


@dataclass(frozen=True)
class WebSocketFrame:
    fin: bool
    opcode: int
    payload: bytes


MessageHandler = Callable[[str, Callable[[str], Awaitable[None]]], Awaitable[None]]


def normalize_peer_url(peer: str, default_port: int = 8765) -> str:
    candidate = peer.strip()
    if not candidate:
        raise ValueError("peer must not be empty")
    if "://" not in candidate:
        candidate = f"ws://{candidate}"

    parsed = urlparse(candidate)
    if parsed.scheme != "ws":
        raise ValueError("MVP supports ws:// peers only")
    if not parsed.hostname:
        raise ValueError("peer URL must include a host")

    netloc = _format_netloc(parsed, default_port)
    path = parsed.path or "/"
    return urlunparse(("ws", netloc, path, "", parsed.query, ""))


def _format_netloc(parsed: ParseResult, default_port: int) -> str:
    host = parsed.hostname or ""
    if ":" in host and not host.startswith("["):
        host = f"[{host}]"
    port = parsed.port or default_port
    return f"{host}:{port}"


def encode_text_frame(
    text: str,
    *,
    mask: bool,
    mask_key: bytes | None = None,
) -> bytes:
    return encode_frame(1, text.encode("utf-8"), mask=mask, mask_key=mask_key)


def encode_frame(
    opcode: int,
    payload: bytes,
    *,
    mask: bool,
    mask_key: bytes | None = None,
) -> bytes:
    first = 0x80 | (opcode & 0x0F)
    length = len(payload)
    header = bytearray([first])

    if length < 126:
        header.append((0x80 if mask else 0) | length)
    elif length < 65536:
        header.append((0x80 if mask else 0) | 126)
        header.extend(length.to_bytes(2, "big"))
    else:
        header.append((0x80 if mask else 0) | 127)
        header.extend(length.to_bytes(8, "big"))

    if not mask:
        return bytes(header) + payload

    key = mask_key or os.urandom(4)
    if len(key) != 4:
        raise ValueError("mask_key must be exactly 4 bytes")
    masked = bytes(byte ^ key[index % 4] for index, byte in enumerate(payload))
    return bytes(header) + key + masked


async def read_frame(reader: asyncio.StreamReader) -> WebSocketFrame:
    first_two = await reader.readexactly(2)
    first, second = first_two
    fin = bool(first & 0x80)
    opcode = first & 0x0F
    masked = bool(second & 0x80)
    length = second & 0x7F

    if length == 126:
        length = int.from_bytes(await reader.readexactly(2), "big")
    elif length == 127:
        length = int.from_bytes(await reader.readexactly(8), "big")

    mask_key = await reader.readexactly(4) if masked else b""
    payload = await reader.readexactly(length) if length else b""
    if masked:
        payload = bytes(byte ^ mask_key[index % 4] for index, byte in enumerate(payload))
    return WebSocketFrame(fin=fin, opcode=opcode, payload=payload)


async def send_text(writer: asyncio.StreamWriter, text: str, *, mask: bool) -> None:
    writer.write(encode_text_frame(text, mask=mask))
    await writer.drain()


class WebSocketServer:
    def __init__(
        self,
        host: str,
        port: int,
        on_message: MessageHandler,
        *,
        logger: logging.Logger | None = None,
        hello_message: str | None = None,
    ) -> None:
        self.host = host
        self.port = port
        self.on_message = on_message
        self.logger = logger or logging.getLogger(__name__)
        self.hello_message = hello_message
        self._server: asyncio.AbstractServer | None = None
        self._clients: set[asyncio.StreamWriter] = set()

    @property
    def actual_port(self) -> int:
        if not self._server or not self._server.sockets:
            return self.port
        return int(self._server.sockets[0].getsockname()[1])

    async def start(self) -> None:
        self._server = await asyncio.start_server(self._handle_client, self.host, self.port)

    async def close(self) -> None:
        if self._server:
            self._server.close()
            await self._server.wait_closed()
            self._server = None
        clients = list(self._clients)
        self._clients.clear()
        for writer in clients:
            writer.close()
            await _wait_closed(writer)

    async def __aenter__(self) -> WebSocketServer:
        await self.start()
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        await self.close()

    async def broadcast(self, text: str) -> None:
        disconnected: list[asyncio.StreamWriter] = []
        for writer in list(self._clients):
            try:
                await send_text(writer, text, mask=False)
            except (ConnectionError, asyncio.IncompleteReadError, OSError):
                disconnected.append(writer)
        for writer in disconnected:
            self._clients.discard(writer)
            writer.close()
            await _wait_closed(writer)

    async def _handle_client(
        self,
        reader: asyncio.StreamReader,
        writer: asyncio.StreamWriter,
    ) -> None:
        peer_name = _format_socket_peer(writer.get_extra_info("peername"))
        connected = False
        try:
            await _server_handshake(reader, writer)
            self._clients.add(writer)
            connected = True
            self.logger.info("远端设备已连接：%s", peer_name)
            if self.hello_message:
                await send_text(writer, self.hello_message, mask=False)

            async def reply(text: str) -> None:
                await send_text(writer, text, mask=False)

            while not reader.at_eof():
                frame = await read_frame(reader)
                if frame.opcode == 8:
                    break
                if frame.opcode == 9:
                    writer.write(encode_frame(10, frame.payload, mask=False))
                    await writer.drain()
                    continue
                if frame.opcode != 1:
                    continue
                payload = frame.payload.decode("utf-8")
                device_id = _extract_hello_device_id(payload)
                if device_id:
                    self.logger.info("设备已识别：%s（设备 ID：%s）", peer_name, device_id)
                    continue
                await self.on_message(payload, reply)
        except (asyncio.IncompleteReadError, ConnectionError, OSError, ValueError) as exc:
            self.logger.debug("远端设备连接异常：%s", exc)
        finally:
            self._clients.discard(writer)
            writer.close()
            await _wait_closed(writer)
            if connected:
                self.logger.info("远端设备已断开：%s", peer_name)


class ReconnectingWebSocketClient:
    def __init__(
        self,
        url: str,
        on_message: MessageHandler,
        *,
        reconnect_delay: float = 2.0,
        logger: logging.Logger | None = None,
        hello_message: str | None = None,
    ) -> None:
        self.url = normalize_peer_url(url)
        self.on_message = on_message
        self.reconnect_delay = reconnect_delay
        self.logger = logger or logging.getLogger(__name__)
        self.hello_message = hello_message
        self._writer: asyncio.StreamWriter | None = None
        self._task: asyncio.Task[None] | None = None
        self._stopped = asyncio.Event()

    @property
    def connected(self) -> bool:
        return self._writer is not None and not self._writer.is_closing()

    def start(self) -> None:
        if self._task is None:
            self._task = asyncio.create_task(self._run())

    async def stop(self) -> None:
        self._stopped.set()
        if self._writer:
            self._writer.close()
            await _wait_closed(self._writer)
        if self._task:
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass
            self._task = None

    async def send(self, text: str) -> bool:
        if not self.connected or self._writer is None:
            return False
        try:
            await send_text(self._writer, text, mask=True)
            return True
        except (ConnectionError, OSError):
            self._writer = None
            return False

    async def _run(self) -> None:
        while not self._stopped.is_set():
            try:
                reader, writer = await _connect(self.url)
                self._writer = writer
                self.logger.info("已连接设备：%s", self.url)
                if self.hello_message:
                    await send_text(writer, self.hello_message, mask=True)

                async def reply(text: str) -> None:
                    await send_text(writer, text, mask=True)

                while not self._stopped.is_set() and not reader.at_eof():
                    frame = await read_frame(reader)
                    if frame.opcode == 8:
                        break
                    if frame.opcode == 9:
                        writer.write(encode_frame(10, frame.payload, mask=True))
                        await writer.drain()
                        continue
                    if frame.opcode != 1:
                        continue
                    payload = frame.payload.decode("utf-8")
                    device_id = _extract_hello_device_id(payload)
                    if device_id:
                        self.logger.info("设备已识别：%s（设备 ID：%s）", self.url, device_id)
                        continue
                    await self.on_message(payload, reply)
            except (asyncio.IncompleteReadError, ConnectionError, OSError, ValueError) as exc:
                self.logger.info("设备暂不可用：%s（%s）", self.url, exc)
            finally:
                if self._writer:
                    self.logger.info("设备连接已断开：%s", self.url)
                    self._writer.close()
                    await _wait_closed(self._writer)
                    self._writer = None

            try:
                await asyncio.wait_for(self._stopped.wait(), timeout=self.reconnect_delay)
            except TimeoutError:
                continue


async def _connect(url: str) -> tuple[asyncio.StreamReader, asyncio.StreamWriter]:
    parsed = urlparse(normalize_peer_url(url))
    host = parsed.hostname or ""
    port = parsed.port or 8765
    path = parsed.path or "/"
    if parsed.query:
        path = f"{path}?{parsed.query}"
    reader, writer = await asyncio.open_connection(host, port)
    await _client_handshake(reader, writer, host, port, path)
    return reader, writer


async def _server_handshake(
    reader: asyncio.StreamReader,
    writer: asyncio.StreamWriter,
) -> None:
    request = await _read_http_header_block(reader)
    lines = request.decode("iso-8859-1").split("\r\n")
    if not lines or not lines[0].startswith("GET "):
        raise ValueError("invalid WebSocket request")
    headers = _parse_headers(lines[1:])
    key = headers.get("sec-websocket-key")
    if not key:
        raise ValueError("missing Sec-WebSocket-Key")
    accept = _accept_key(key)
    response = (
        "HTTP/1.1 101 Switching Protocols\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        f"Sec-WebSocket-Accept: {accept}\r\n"
        "\r\n"
    )
    writer.write(response.encode("ascii"))
    await writer.drain()


async def _client_handshake(
    reader: asyncio.StreamReader,
    writer: asyncio.StreamWriter,
    host: str,
    port: int,
    path: str,
) -> None:
    key = base64.b64encode(os.urandom(16)).decode("ascii")
    request = (
        f"GET {path} HTTP/1.1\r\n"
        f"Host: {host}:{port}\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        f"Sec-WebSocket-Key: {key}\r\n"
        "Sec-WebSocket-Version: 13\r\n"
        "\r\n"
    )
    writer.write(request.encode("ascii"))
    await writer.drain()
    response = await _read_http_header_block(reader)
    lines = response.decode("iso-8859-1").split("\r\n")
    if not lines or " 101 " not in lines[0]:
        raise ValueError("WebSocket upgrade failed")
    headers = _parse_headers(lines[1:])
    if headers.get("sec-websocket-accept") != _accept_key(key):
        raise ValueError("WebSocket accept key mismatch")


async def _read_http_header_block(reader: asyncio.StreamReader) -> bytes:
    data = await reader.readuntil(b"\r\n\r\n")
    if len(data) > 16384:
        raise ValueError("HTTP header block is too large")
    return data


def _parse_headers(lines: list[str]) -> dict[str, str]:
    headers: dict[str, str] = {}
    for line in lines:
        if not line or ":" not in line:
            continue
        name, value = line.split(":", 1)
        headers[name.strip().lower()] = value.strip()
    return headers


def _accept_key(key: str) -> str:
    digest = hashlib.sha1((key + GUID).encode("ascii")).digest()
    return base64.b64encode(digest).decode("ascii")


def _format_socket_peer(peer: object) -> str:
    if isinstance(peer, tuple) and len(peer) >= 2:
        return f"{peer[0]}:{peer[1]}"
    return str(peer or "unknown")


def _extract_hello_device_id(text: str) -> str | None:
    try:
        payload = json.loads(text)
    except json.JSONDecodeError:
        return None
    if not isinstance(payload, dict) or payload.get("type") != "hello":
        return None
    device_id = payload.get("deviceId")
    if not isinstance(device_id, str) or not device_id.strip():
        return None
    return device_id.strip()


async def _wait_closed(writer: asyncio.StreamWriter) -> None:
    try:
        await writer.wait_closed()
    except (ConnectionError, OSError):
        pass
