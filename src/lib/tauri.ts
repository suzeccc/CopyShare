import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { emitTo, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  currentMonitor,
  getCurrentWindow,
  LogicalPosition,
  LogicalSize,
} from "@tauri-apps/api/window";

import {
  FLOATING_WINDOW_BOUNDS,
  MAIN_WINDOW_BOUNDS,
  TRANSPARENT_WINDOW_BACKGROUND,
} from "@/lib/windowMode";
import type { ClipboardPreviewItem } from "@/lib/historyPreview";
import { getMediaPreviewWindowPosition } from "@/lib/mediaPreviewWindow";
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
import type { TranslateResponse } from "@/types/translation";

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

export function openHistoryFileLocation(historyId: string): Promise<void> {
  return invoke<void>("open_history_file_location", { historyId });
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

export function getCacheSize(): Promise<number> {
  return invoke<number>("get_cache_size");
}

export function clearCache(): Promise<number> {
  return invoke<number>("clear_cache");
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

export function getHistoryFileThumbnail(
  historyId: string,
  maxSize = 200,
): Promise<string> {
  return invoke<string>("get_history_file_thumbnail", { historyId, maxSize });
}

export function getHistoryFilePreviewPath(historyId: string): Promise<string> {
  return invoke<string>("get_history_file_preview_path", { historyId });
}

export function convertLocalFileSrc(filePath: string): string {
  return convertFileSrc(filePath);
}

export const MEDIA_PREVIEW_WINDOW_LABEL = "media-preview";

export const MEDIA_PREVIEW_WINDOW_BOUNDS = {
  width: 720,
  height: 520,
  offset: 14,
} as const;

export const FLOATING_CLIPBOARD_WINDOW_LABEL = "floating-clipboard-history";

export const FLOATING_CLIPBOARD_WINDOW_BOUNDS = {
  width: 460,
  height: 620,
  offset: 14,
} as const;

export const FLOATING_CLIPBOARD_HISTORY_STORAGE_KEY = "copyshare:floating-clipboard-history";

export type MediaPreviewKind = "image" | "video";

export type MediaPreviewPayload = {
  kind: MediaPreviewKind;
  historyId: string;
  title: string;
  src?: string;
};

export type FloatingClipboardHistoryPayload = {
  items: ClipboardPreviewItem[];
};

function mediaPreviewUrl(payload: MediaPreviewPayload): string {
  const params = new URLSearchParams({
    kind: payload.kind,
    historyId: payload.historyId,
    title: payload.title,
  });
  if (payload.src) {
    params.set("src", payload.src);
  }
  return `/#/media-preview?${params.toString()}`;
}

async function nearbyFloatingWindowInitialPosition(
  bounds: { width: number; height: number; offset: number },
): Promise<LogicalPosition | undefined> {
  try {
    const current = getCurrentWindow();
    const [position, size, scaleFactor, monitor] = await Promise.all([
      current.outerPosition(),
      current.outerSize(),
      current.scaleFactor(),
      currentMonitor(),
    ]);
    const monitorPosition = monitor?.workArea?.position ?? monitor?.position;
    const monitorSize = monitor?.workArea?.size ?? monitor?.size;
    if (!monitorPosition || !monitorSize) {
      return undefined;
    }
    const next = getMediaPreviewWindowPosition({
      floating: {
        x: position.x / scaleFactor,
        y: position.y / scaleFactor,
        width: size.width / scaleFactor,
        height: size.height / scaleFactor,
      },
      monitor: {
        x: monitorPosition.x / scaleFactor,
        y: monitorPosition.y / scaleFactor,
        width: monitorSize.width / scaleFactor,
        height: monitorSize.height / scaleFactor,
      },
      preview: bounds,
    });

    return new LogicalPosition(next.x, next.y);
  } catch {
    return undefined;
  }
}

async function mediaPreviewInitialPosition(): Promise<LogicalPosition | undefined> {
  return nearbyFloatingWindowInitialPosition(MEDIA_PREVIEW_WINDOW_BOUNDS);
}

export async function openMediaPreviewWindow(payload: MediaPreviewPayload): Promise<void> {
  const existing = await WebviewWindow.getByLabel(MEDIA_PREVIEW_WINDOW_LABEL);
  if (existing) {
    await emitTo(MEDIA_PREVIEW_WINDOW_LABEL, "media-preview-open", payload);
    await existing.show();
    await existing.setFocus();
    return;
  }

  const position = await mediaPreviewInitialPosition();
  new WebviewWindow(MEDIA_PREVIEW_WINDOW_LABEL, {
    url: mediaPreviewUrl(payload),
    title: "媒体预览",
    width: MEDIA_PREVIEW_WINDOW_BOUNDS.width,
    height: MEDIA_PREVIEW_WINDOW_BOUNDS.height,
    minWidth: 420,
    minHeight: 300,
    x: position?.x,
    y: position?.y,
    decorations: false,
    transparent: true,
    backgroundColor: TRANSPARENT_WINDOW_BACKGROUND,
    resizable: true,
    visible: true,
    focus: true,
    alwaysOnTop: true,
    shadow: false,
  });
}

function writeFloatingClipboardHistoryPayload(payload: FloatingClipboardHistoryPayload): void {
  window.localStorage.setItem(FLOATING_CLIPBOARD_HISTORY_STORAGE_KEY, JSON.stringify(payload));
}

export async function updateFloatingClipboardHistoryWindow(payload: FloatingClipboardHistoryPayload): Promise<void> {
  writeFloatingClipboardHistoryPayload(payload);
  const existing = await WebviewWindow.getByLabel(FLOATING_CLIPBOARD_WINDOW_LABEL);
  if (!existing) {
    return;
  }

  await emitTo(FLOATING_CLIPBOARD_WINDOW_LABEL, "floating-clipboard-refresh", payload);
}

export async function openFloatingClipboardHistoryWindow(payload: FloatingClipboardHistoryPayload): Promise<void> {
  writeFloatingClipboardHistoryPayload(payload);
  const existing = await WebviewWindow.getByLabel(FLOATING_CLIPBOARD_WINDOW_LABEL);
  if (existing) {
    await emitTo(FLOATING_CLIPBOARD_WINDOW_LABEL, "floating-clipboard-refresh", payload);
    await existing.show();
    await existing.setFocus();
    return;
  }

  const position = await nearbyFloatingWindowInitialPosition(FLOATING_CLIPBOARD_WINDOW_BOUNDS);
  new WebviewWindow(FLOATING_CLIPBOARD_WINDOW_LABEL, {
    url: "/#/floating-clipboard",
    title: "剪贴板内容",
    width: FLOATING_CLIPBOARD_WINDOW_BOUNDS.width,
    height: FLOATING_CLIPBOARD_WINDOW_BOUNDS.height,
    minWidth: 360,
    minHeight: 360,
    x: position?.x,
    y: position?.y,
    decorations: false,
    transparent: true,
    backgroundColor: TRANSPARENT_WINDOW_BACKGROUND,
    resizable: true,
    visible: true,
    focus: true,
    alwaysOnTop: true,
    shadow: false,
  });
}

export function openExternalUrl(url: string): Promise<void> {
  return invoke<void>("open_external_url", { url });
}

export function translateText(text: string, targetLang: string): Promise<TranslateResponse> {
  return invoke<TranslateResponse>("translate_text", { text, targetLang });
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

