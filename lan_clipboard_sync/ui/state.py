from __future__ import annotations

from dataclasses import dataclass, field
import time


@dataclass(frozen=True)
class PeerStatus:
    name: str
    address: str
    state: str = "offline"

    @property
    def label(self) -> str:
        return f"{self.name}  {self.address}"


@dataclass(frozen=True)
class UiLogEntry:
    message: str
    level: str = "info"
    timestamp: float = field(default_factory=time.time)


@dataclass
class SyncUiState:
    active_view: str = "overview"
    running: bool = False
    paused: bool = False
    listen_port: int = 8765
    latest_latency_ms: int = 0
    sync_count: int = 0
    duplicate_blocks: int = 0
    max_logs: int = 100
    peers: list[PeerStatus] = field(default_factory=list)
    logs: list[UiLogEntry] = field(default_factory=list)
    valid_views: tuple[str, ...] = ("overview", "devices", "history", "settings")

    @property
    def status_text(self) -> str:
        if not self.running:
            return "未启动"
        if self.paused:
            return "已暂停"
        if self.connected_count == 0:
            return "未连接设备"
        return "同步中"

    @property
    def status_key(self) -> str:
        if not self.running:
            return "stopped"
        if self.paused:
            return "degraded"
        if self.connected_count == 0:
            return "disconnected"
        return "running"

    @property
    def connected_count(self) -> int:
        return sum(1 for peer in self.peers if peer.state == "online")

    def set_running(self, running: bool) -> None:
        self.running = running
        if not running:
            self.paused = False

    def set_paused(self, paused: bool) -> None:
        if self.running:
            self.paused = paused

    def upsert_peer(self, peer: PeerStatus) -> None:
        for index, existing in enumerate(self.peers):
            if existing.address == peer.address or existing.name == peer.name:
                self.peers[index] = peer
                return
        self.peers.append(peer)

    def add_log(self, message: str, level: str = "info") -> None:
        self.logs.insert(0, UiLogEntry(message=message, level=level))
        del self.logs[self.max_logs :]

    def clear_logs(self) -> None:
        self.logs.clear()

    def set_active_view(self, view: str) -> bool:
        if view not in self.valid_views:
            return False
        self.active_view = view
        return True
