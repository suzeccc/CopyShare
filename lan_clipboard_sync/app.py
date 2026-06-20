from __future__ import annotations

import asyncio
from collections.abc import Iterable
import logging

from .clipboard import Clipboard, create_system_clipboard
from .messages import ClipboardMessage, decode_message, encode_message
from .sync_engine import SyncEngine
from .websocket import ReconnectingWebSocketClient, WebSocketServer, normalize_peer_url


CLIPBOARD_FORMAT_LABELS = {
    "text": "文本",
    "image": "图片",
}


class ClipboardSyncApp:
    def __init__(
        self,
        *,
        device_id: str,
        host: str = "0.0.0.0",
        port: int = 8765,
        peers: Iterable[str] = (),
        poll_interval: float = 0.1,
        reconnect_delay: float = 2.0,
        clipboard: Clipboard | None = None,
        logger: logging.Logger | None = None,
    ) -> None:
        self.device_id = device_id
        self.host = host
        self.port = port
        self.peers = [normalize_peer_url(peer, default_port=port) for peer in peers]
        self.poll_interval = poll_interval
        self.reconnect_delay = reconnect_delay
        self.clipboard = clipboard or create_system_clipboard()
        self.logger = logger or logging.getLogger(__name__)
        self.engine = SyncEngine(device_id=device_id)
        self.hello_message = encode_message({"type": "hello", "deviceId": self.device_id})
        self.server = WebSocketServer(host, port, self._handle_incoming_text, logger=self.logger, hello_message=self.hello_message)
        self.clients = [
            ReconnectingWebSocketClient(
                peer,
                self._handle_incoming_text,
                reconnect_delay=reconnect_delay,
                logger=self.logger,
                hello_message=self.hello_message,
            )
            for peer in self.peers
        ]
        self._watcher_task: asyncio.Task[None] | None = None
        self._stop_event = asyncio.Event()

    async def start(self) -> None:
        await self.server.start()
        for client in self.clients:
            client.start()
        self._watcher_task = asyncio.create_task(self._watch_clipboard())
        self.logger.info("同步核心已监听 %s:%s，设备 ID：%s", self.host, self.server.actual_port, self.device_id)

    async def stop(self) -> None:
        self._stop_event.set()
        if self._watcher_task:
            self._watcher_task.cancel()
            try:
                await self._watcher_task
            except asyncio.CancelledError:
                pass
            self._watcher_task = None
        for client in self.clients:
            await client.stop()
        await self.server.close()

    async def run(self) -> None:
        await self.start()
        try:
            await self._stop_event.wait()
        finally:
            await self.stop()

    async def _watch_clipboard(self) -> None:
        while not self._stop_event.is_set():
            try:
                message = self._read_local_clipboard_message()
                if message:
                    await self._broadcast_message(message)
            except Exception as exc:  # Clipboard and network errors are retryable here.
                self.logger.warning("剪贴板监听失败：%s", exc)
            await asyncio.sleep(self.poll_interval)

    def _read_local_clipboard_message(self) -> ClipboardMessage | None:
        content = self.clipboard.read_content()
        message = self.engine.observe_local_content(content)
        if message:
            self.logger.info("本机复制%s，准备同步", _clipboard_format_label(message.format))
        return message

    async def _handle_incoming_text(self, text: str, reply) -> None:
        try:
            message = decode_message(text)
        except ValueError as exc:
            self.logger.debug("ignored invalid message: %s", exc)
            return

        if isinstance(message, dict):
            if message.get("type") == "ping":
                await reply(encode_message({"type": "pong"}))
            return

        applied = self.engine.apply_remote_message(message, write_content=self.clipboard.write_content)
        if applied:
            self.logger.info("收到远端%s，已写入剪贴板", _clipboard_format_label(message.format))
            await self._broadcast_message(message)

    async def _broadcast_message(self, message: ClipboardMessage) -> None:
        text = encode_message(message)
        await self.server.broadcast(text)
        for client in self.clients:
            await client.send(text)


def _clipboard_format_label(format: str) -> str:
    return CLIPBOARD_FORMAT_LABELS.get(format, format)
