export type DeviceStatus = "online" | "connecting" | "offline" | "blocked";

export interface DeviceInfo {
  id: string;
  name: string;
  ip: string;
  port: number;
  connected: boolean;
  trusted: boolean;
  remoteTrusted: boolean;
  hasConnectedBefore: boolean;
  lastSeenAt: string | null;
  status: DeviceStatus;
}

export type LanDiscoveryScanStatus = "idle" | "running" | "done" | "empty" | "failed";

export interface LanDiscoveryProgress {
  scanId: number;
  status: LanDiscoveryScanStatus;
  running: boolean;
  done: number;
  total: number;
  rangeCount: number;
  startedAt: number;
  finishedAt: number | null;
}
