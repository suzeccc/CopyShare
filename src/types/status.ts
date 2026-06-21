export type SyncState = "stopped" | "running" | "error";

export interface AppStatus {
  running: boolean;
  deviceName: string;
  deviceId: string;
  localIp: string | null;
  port: number;
  connectedCount: number;
  lastSyncAt: string | null;
  state: SyncState;
  message: string | null;
}
