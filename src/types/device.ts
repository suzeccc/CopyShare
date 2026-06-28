export type DeviceStatus = "online" | "connecting" | "offline" | "blocked";

export interface DeviceInfo {
  id: string;
  name: string;
  ip: string;
  port: number;
  connected: boolean;
  trusted: boolean;
  remoteTrusted: boolean;
  lastSeenAt: string | null;
  status: DeviceStatus;
}
