import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { AppConfig } from "@/types/config";
import type { DeviceInfo } from "@/types/device";
import type { HistoryItem } from "@/types/history";
import type { AppStatus } from "@/types/status";

export type AppEventName =
  | "sync-status-changed"
  | "device-discovered"
  | "device-connected"
  | "device-disconnected"
  | "clipboard-synced"
  | "sync-error"
  | "config-updated";

export function getStatus(): Promise<AppStatus> {
  return invoke<AppStatus>("get_status");
}

export function startSync(): Promise<AppStatus> {
  return invoke<AppStatus>("start_sync");
}

export function stopSync(): Promise<AppStatus> {
  return invoke<AppStatus>("stop_sync");
}

export function getDevices(): Promise<DeviceInfo[]> {
  return invoke<DeviceInfo[]>("get_devices");
}

export function connectDevice(ip: string, port: number): Promise<DeviceInfo> {
  return invoke<DeviceInfo>("connect_device", { ip, port });
}

export function disconnectDevice(deviceId: string): Promise<void> {
  return invoke<void>("disconnect_device", { deviceId });
}

export function trustDevice(deviceId: string): Promise<void> {
  return invoke<void>("trust_device", { deviceId });
}

export function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export function updateConfig(config: AppConfig): Promise<AppConfig> {
  return invoke<AppConfig>("update_config", { config });
}

export function getHistory(): Promise<HistoryItem[]> {
  return invoke<HistoryItem[]>("get_history");
}

export function clearHistory(): Promise<void> {
  return invoke<void>("clear_history");
}

export function showMainWindow(): Promise<void> {
  return invoke<void>("show_main_window");
}

export function hideMainWindow(): Promise<void> {
  return invoke<void>("hide_main_window");
}

export function onAppEvent<T>(
  eventName: AppEventName,
  callback: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<T>(eventName, (event) => callback(event.payload));
}
