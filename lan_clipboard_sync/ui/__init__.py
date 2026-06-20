"""Desktop UI package for LAN Clipboard Sync."""

from .controller import SyncUiController
from .state import PeerStatus, SyncUiState, UiLogEntry

__all__ = ["PeerStatus", "SyncUiController", "SyncUiState", "UiLogEntry"]
