from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol

from ..websocket import normalize_peer_url
from .state import PeerStatus, SyncUiState


class SyncRuntime(Protocol):
    def start(self, port: int, peers: list[str]) -> None:
        ...

    def stop(self) -> None:
        ...

    def set_device_id(self, device_id: str) -> None:
        ...


@dataclass
class SyncUiController:
    device_id: str
    runtime: SyncRuntime | None = None

    def __post_init__(self) -> None:
        self.state = SyncUiState()
        self.configured_peers: list[str] = []
        self._last_port = self.state.listen_port

    def start(self, *, port: int, peers: list[str]) -> bool:
        normalized: list[str] = []
        try:
            normalized = [normalize_peer_url(peer, default_port=port) for peer in peers if peer.strip()]
        except ValueError as exc:
            self.state.add_log(f"设备地址无效：{exc}", level="error")
            self.state.set_running(False)
            return False

        if not normalized:
            normalized = list(self.configured_peers)

        self.state.listen_port = port
        self._last_port = port
        self.configured_peers = normalized
        self.state.set_running(True)
        self.state.set_paused(False)
        self.state.add_log(f"同步服务已启动：端口 {port}")

        for peer in normalized:
            self.state.upsert_peer(PeerStatus(name=_peer_name(peer), address=peer, state="reconnecting"))

        if self.runtime is not None:
            try:
                self.runtime.start(port, normalized)
            except Exception as exc:
                self.state.set_running(False)
                self.state.add_log(f"同步核心启动失败：{exc}", level="error")
                return False
        return True

    def pause(self) -> None:
        if not self.state.running or self.state.paused:
            return
        if self.runtime is not None:
            self.runtime.stop()
        self.state.set_paused(True)
        self.state.add_log("同步已暂停")

    def resume(self) -> None:
        if not self.state.running or not self.state.paused:
            return
        if self.runtime is not None:
            try:
                self.runtime.start(self._last_port, self.configured_peers)
            except Exception as exc:
                self.state.add_log(f"同步核心恢复失败：{exc}", level="error")
                return
        self.state.set_paused(False)
        self.state.add_log("同步已恢复")

    def stop(self) -> None:
        if self.runtime is not None and self.state.running:
            self.runtime.stop()
        self.state.set_running(False)
        self.state.add_log("同步服务已停止")

    def add_peer(self, address: str) -> bool:
        try:
            normalized = normalize_peer_url(address, default_port=self.state.listen_port)
        except ValueError as exc:
            self.state.add_log(f"设备地址无效：{exc}", level="error")
            return False

        if normalized not in self.configured_peers:
            self.configured_peers.append(normalized)
            self.state.upsert_peer(PeerStatus(name=_peer_name(normalized), address=normalized, state="reconnecting"))
            self.state.add_log(f"已添加设备：{_peer_name(normalized)}")
            self._restart_if_running()
        return True

    def remove_peer(self, address: str) -> bool:
        if address not in self.configured_peers:
            return False
        self.configured_peers.remove(address)
        self.state.peers = [peer for peer in self.state.peers if peer.address != address]
        self.state.add_log(f"已移除设备：{_peer_name(address)}")
        self._restart_if_running()
        return True

    def clear_peers(self) -> None:
        self.configured_peers.clear()
        self.state.peers.clear()
        self.state.add_log("已清空设备列表")
        self._restart_if_running()

    def clear_logs(self) -> None:
        self.state.clear_logs()

    def set_view(self, view: str) -> bool:
        return self.state.set_active_view(view)

    def apply_settings(self, *, port_text: str, device_id_text: str | None = None) -> bool:
        device_id = self.device_id if device_id_text is None else device_id_text.strip()
        if not device_id:
            self.state.add_log("设备 ID 不能为空", level="error")
            return False
        try:
            port = int(port_text.strip())
        except ValueError:
            self.state.add_log("端口必须是数字", level="error")
            return False
        if not 1 <= port <= 65535:
            self.state.add_log("端口必须在 1 到 65535 之间", level="error")
            return False
        if self.state.running:
            self.state.add_log("运行中不能修改设备 ID 或端口，请先断开全部", level="error")
            return False
        self.device_id = device_id
        if self.runtime is not None:
            self.runtime.set_device_id(device_id)
        self.state.listen_port = port
        self._last_port = port
        self.state.add_log(f"设置已保存：设备 ID {device_id}，端口 {port}")
        return True

    def _restart_if_running(self) -> None:
        if self.runtime is None or not self.state.running or self.state.paused:
            return
        self.runtime.stop()
        self.runtime.start(self._last_port, self.configured_peers)


def _peer_name(url: str) -> str:
    without_scheme = url.removeprefix("ws://")
    return without_scheme.split("/", 1)[0]
