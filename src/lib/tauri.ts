import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  getCurrentWindow,
  LogicalSize,
} from "@tauri-apps/api/window";

import {
  FLOATING_WINDOW_BOUNDS,
  MAIN_WINDOW_BOUNDS,
  TRANSPARENT_WINDOW_BACKGROUND,
} from "@/lib/windowMode";
import type { AppConfig } from "@/types/config";
import type { DeviceInfo } from "@/types/device";
import type {
  FileTransferProgressEvent,
  FileTransferTask,
  SelectedTransferFile,
} from "@/types/fileTransfer";
import type { HistoryItem } from "@/types/history";
import type { MobileSessionView } from "@/types/mobile";
import type { AppStatus } from "@/types/status";

export type AppEventName =
  | "sync-status-changed"
  | "device-discovered"
  | "device-connected"
  | "device-disconnected"
  | "device-rejected"
  | "lan-discovery-progress"
  | "clipboard-synced"
  | "sync-error"
  | "config-updated"
  | "navigate-to-page"
  | "file-transfer-offer"
  | "file-transfer-updated"
  | "file-transfer-progress"
  | "file-transfer-completed"
  | "file-transfer-failed";

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

export function rejectDevice(deviceId: string): Promise<void> {
  return invoke<void>("reject_device", { deviceId });
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

export function getClipboardHistory(): Promise<Array<{ id: string; text: string }>> {
  return invoke<Array<{ id: string; text: string }>>("get_clipboard_history");
}

export function selectFileForTransfer(): Promise<SelectedTransferFile | null> {
  return invoke<SelectedTransferFile | null>("select_file_for_transfer");
}

export function selectFilesForTransfer(): Promise<SelectedTransferFile[]> {
  return invoke<SelectedTransferFile[]>("select_files_for_transfer");
}

export function sendFileToDevice(
  deviceId: string,
  filePath: string,
): Promise<FileTransferTask> {
  return invoke<FileTransferTask>("send_file_to_device", { deviceId, filePath });
}

export function sendFilesToDevice(
  deviceId: string,
  filePaths: string[],
): Promise<FileTransferTask> {
  return invoke<FileTransferTask>("send_files_to_device", { deviceId, filePaths });
}

export function acceptFileTransfer(transferId: string): Promise<FileTransferTask> {
  return invoke<FileTransferTask>("accept_file_transfer", { transferId });
}

export function rejectFileTransfer(transferId: string): Promise<FileTransferTask> {
  return invoke<FileTransferTask>("reject_file_transfer", { transferId });
}

export function cancelFileTransfer(transferId: string): Promise<FileTransferTask> {
  return invoke<FileTransferTask>("cancel_file_transfer", { transferId });
}

export function getFileTransfers(): Promise<FileTransferTask[]> {
  return invoke<FileTransferTask[]>("get_file_transfers");
}

export function getTransferSaveDir(): Promise<string> {
  return invoke<string>("get_transfer_save_dir");
}

export function selectTransferSaveDir(): Promise<AppConfig | null> {
  return invoke<AppConfig | null>("select_transfer_save_dir");
}

export function resetTransferSaveDir(): Promise<AppConfig> {
  return invoke<AppConfig>("reset_transfer_save_dir");
}

export function openTransferFolder(): Promise<void> {
  return invoke<void>("open_transfer_folder");
}

export function createMobileSession(): Promise<MobileSessionView> {
  return invoke<MobileSessionView>("create_mobile_session");
}

export function getMobileSessionStatus(sessionId: string): Promise<MobileSessionView> {
  return invoke<MobileSessionView>("get_mobile_session_status", { sessionId });
}

export function closeMobileSession(sessionId: string): Promise<MobileSessionView> {
  return invoke<MobileSessionView>("close_mobile_session", { sessionId });
}

export function confirmMobileClipboardWrite(sessionId: string): Promise<MobileSessionView> {
  return invoke<MobileSessionView>("confirm_mobile_clipboard_write", { sessionId });
}

export function clearHistory(): Promise<void> {
  return invoke<void>("clear_history");
}

export type CopyHistoryResult = "copied" | "downloadStarted" | "downloading";

export function copyHistoryItem(historyId: string): Promise<CopyHistoryResult> {
  return invoke<CopyHistoryResult>("copy_history_item", { historyId });
}

export function getHistoryImageThumbnail(
  historyId: string,
  maxSize = 200,
): Promise<string> {
  return invoke<string>("get_history_image_thumbnail", { historyId, maxSize });
}

export function openExternalUrl(url: string): Promise<void> {
  return invoke<void>("open_external_url", { url });
}

export function showMainWindow(): Promise<void> {
  return invoke<void>("show_main_window");
}

export function hideMainWindow(): Promise<void> {
  return invoke<void>("hide_main_window");
}

export function sendTestNotification(): Promise<void> {
  return invoke<void>("send_test_notification");
}

export function moveFloatingWindowToCursor(): Promise<void> {
  return invoke<void>("move_floating_window_to_cursor");
}

export function moveMainWindowToCenter(): Promise<void> {
  return invoke<void>("move_main_window_to_center");
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
  try {
    await moveFloatingWindowToCursor();
  } catch (error) {
    console.warn("Unable to move floating window to cursor", error);
  }
  await window.setShadow(false);
  await window.setFocus();
}

export async function restoreMainWindow(): Promise<void> {
  const window = getCurrentWindow();

  await window.setBackgroundColor(TRANSPARENT_WINDOW_BACKGROUND);
  await window.setMaxSize(null);
  await window.setMinSize(
    new LogicalSize(MAIN_WINDOW_BOUNDS.minWidth, MAIN_WINDOW_BOUNDS.minHeight),
  );
  await window.setResizable(true);
  await window.setAlwaysOnTop(false);
  await window.setSize(
    new LogicalSize(MAIN_WINDOW_BOUNDS.width, MAIN_WINDOW_BOUNDS.height),
  );
  try {
    await moveMainWindowToCenter();
  } catch (error) {
    console.warn("Unable to move main window to center", error);
  }
  await window.setFocus();
}

export function onAppEvent<T>(
  eventName: AppEventName,
  callback: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<T>(eventName, (event) => callback(event.payload));
}

