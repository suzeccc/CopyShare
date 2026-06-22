import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

import {
  FLOATING_WINDOW_BOUNDS,
  MAIN_WINDOW_BACKGROUND,
  MAIN_WINDOW_BOUNDS,
  TRANSPARENT_WINDOW_BACKGROUND,
} from "@/lib/windowMode";
import type { AppConfig } from "@/types/config";
import type { DeviceInfo } from "@/types/device";
import type { ClipboardPreviewItem } from "@/lib/historyPreview";
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

export function getClipboardHistory(): Promise<ClipboardPreviewItem[]> {
  return invoke<ClipboardPreviewItem[]>("get_clipboard_history");
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

export function minimizeWindow(): Promise<void> {
  return getCurrentWindow().minimize();
}

export function toggleMaximizeWindow(): Promise<void> {
  return getCurrentWindow().toggleMaximize();
}

export function startWindowDrag(): Promise<void> {
  return getCurrentWindow().startDragging();
}

export function closeWindow(): Promise<void> {
  return getCurrentWindow().close();
}

export async function enterFloatingWindow(): Promise<void> {
  const window = getCurrentWindow();
  const size = new LogicalSize(
    FLOATING_WINDOW_BOUNDS.width,
    FLOATING_WINDOW_BOUNDS.height,
  );

  await window.setBackgroundColor(TRANSPARENT_WINDOW_BACKGROUND);
  await window.setAlwaysOnTop(true);
  await window.setResizable(false);
  await window.setMinSize(size);
  await window.setMaxSize(size);
  await window.setSize(size);
  await window.setShadow(false);
  await window.setFocus();
}

export async function restoreMainWindow(): Promise<void> {
  const window = getCurrentWindow();

  await window.setMaxSize(null);
  await window.setMinSize(
    new LogicalSize(MAIN_WINDOW_BOUNDS.minWidth, MAIN_WINDOW_BOUNDS.minHeight),
  );
  await window.setResizable(true);
  await window.setAlwaysOnTop(false);
  await window.setSize(
    new LogicalSize(MAIN_WINDOW_BOUNDS.width, MAIN_WINDOW_BOUNDS.height),
  );
  await window.setBackgroundColor(MAIN_WINDOW_BACKGROUND);
  await window.setFocus();
}

export function onAppEvent<T>(
  eventName: AppEventName,
  callback: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<T>(eventName, (event) => callback(event.payload));
}
