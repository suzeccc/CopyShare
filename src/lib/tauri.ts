import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { emitTo, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  type CloseRequestedEvent,
  availableMonitors,
  currentMonitor,
  Effect,
  getCurrentWindow,
  LogicalPosition,
  LogicalSize,
  PhysicalPosition,
} from "@tauri-apps/api/window";

import {
  FLOATING_BALL_BOUNDS,
  FLOATING_STARTUP_OFFSET,
  FLOATING_WINDOW_BOUNDS,
  getFloatingBallDockPosition,
  getFloatingWindowTopRightPosition,
  MAIN_WINDOW_BOUNDS,
  TRANSPARENT_WINDOW_BACKGROUND,
  waitForWindowSize,
  type AppWindowMode,
} from "./windowMode.ts";
import { translateSource } from "../i18n/index.ts";
import type { ClipboardPreviewItem } from "@/lib/historyPreview";
import { getMediaPreviewWindowPosition } from "./mediaPreviewWindow.ts";
import type { AppConfig } from "@/types/config";
import type { DeviceInfo } from "@/types/device";
import type { NetworkDiagnosticReport } from "@/types/networkDiagnostics";
import type { FileTransferTask } from "@/types/fileTransfer";
import type { HistoryItem } from "@/types/history";
import type {
  CreateSnippetInput,
  LibraryItemUpdate,
  LibrarySnapshot,
} from "@/types/library";
import type { MobileSessionView } from "@/types/mobile";
import type { OcrResponse } from "@/types/ocr";
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
  | "history-updated"
  | "sync-error"
  | "config-updated"
  | "library-updated"
  | "navigate-to-page"
  | "global-shortcut-triggered"
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

export function getNetworkDiagnostics(): Promise<NetworkDiagnosticReport> {
  return invoke<NetworkDiagnosticReport>("get_network_diagnostics");
}

export function repairWindowsFirewall(): Promise<NetworkDiagnosticReport> {
  return invoke<NetworkDiagnosticReport>("repair_windows_firewall");
}

export function getHistory(): Promise<HistoryItem[]> {
  return invoke<HistoryItem[]>("get_history");
}

export function setHistoryItemPinned(historyId: string, pinned: boolean): Promise<HistoryItem[]> {
  return invoke<HistoryItem[]>("set_history_item_pinned", { historyId, pinned });
}

export function getLibrary(): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("get_library");
}

export function collectHistoryItem(
  historyId: string,
  pin: boolean,
): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("collect_history_item", { historyId, pin });
}

export function createTextSnippet(input: CreateSnippetInput): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("create_text_snippet", { ...input });
}

export function updateLibraryItem(
  id: string,
  update: LibraryItemUpdate,
): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("update_library_item", { id, update });
}

export function convertLibraryItemToSnippet(id: string): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("convert_library_item_to_snippet", { id });
}

export function setLibraryItemPinned(
  id: string,
  pinned: boolean,
): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("set_library_item_pinned", { id, pinned });
}

export function reorderPinnedLibraryItems(orderedIds: string[]): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("reorder_pinned_library_items", { orderedIds });
}

export function removeLibraryItem(id: string): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("remove_library_item", { id });
}

export function copyLibraryItem(id: string): Promise<void> {
  return invoke<void>("copy_library_item", { id });
}

export function getLibraryStorageSize(): Promise<number> {
  return invoke<number>("get_library_storage_size");
}

export function getLibraryImageThumbnail(id: string, maxSize = 200): Promise<string> {
  return invoke<string>("get_library_image_thumbnail", { id, maxSize });
}

export function getLibraryVideoPreviewPath(id: string, assetIndex: number): Promise<string> {
  return invoke<string>("get_library_video_preview_path", { id, assetIndex });
}

export function getClipboardHistory(): Promise<Array<{ id: string; text: string; createdAt?: string; sourceDevice?: string }>> {
  return invoke<Array<{ id: string; text: string; createdAt?: string; sourceDevice?: string }>>("get_clipboard_history");
}

export function readClipboardText(): Promise<string> {
  return invoke<string>("read_clipboard_text");
}

export function recognizeClipboardImage(): Promise<OcrResponse> {
  return invoke<OcrResponse>("recognize_clipboard_image");
}

export function resumeFileTransfer(transferId: string): Promise<FileTransferTask> {
  return invoke<FileTransferTask>("resume_file_transfer", { transferId });
}

export function getTransferSaveDir(): Promise<string> {
  return invoke<string>("get_transfer_save_dir");
}

export function selectTransferSaveDir(persist = true): Promise<AppConfig | null> {
  return invoke<AppConfig | null>("select_transfer_save_dir", { persist });
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

export function saveHistoryMedia(historyId: string): Promise<boolean> {
  return invoke<boolean>("save_history_media", { historyId });
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

export type MediaPreviewPayload = {
  kind: "image" | "video";
  historyId: string;
  title: string;
  src: string;
  items?: ClipboardPreviewItem[];
};

export const MEDIA_PREVIEW_ITEMS_STORAGE_KEY = "copyshare:media-preview-items";

export type FloatingClipboardHistoryPayload = {
  items: ClipboardPreviewItem[];
};

function mediaPreviewUrl(payload: MediaPreviewPayload): string {
  const params = new URLSearchParams({
    kind: payload.kind,
    historyId: payload.historyId,
    title: payload.title,
  });
  params.set("src", payload.src);
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
  window.localStorage.setItem(MEDIA_PREVIEW_ITEMS_STORAGE_KEY, JSON.stringify(payload.items ?? []));
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
    title: translateSource("媒体预览"),
    width: MEDIA_PREVIEW_WINDOW_BOUNDS.width,
    height: MEDIA_PREVIEW_WINDOW_BOUNDS.height,
    minWidth: 420,
    minHeight: 300,
    x: position?.x,
    y: position?.y,
    decorations: false,
    transparent: true,
    backgroundColor: TRANSPARENT_WINDOW_BACKGROUND,
    windowEffects: { effects: [Effect.Acrylic, Effect.HudWindow] },
    resizable: true,
    visible: true,
    focus: true,
    alwaysOnTop: true,
    shadow: true,
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
    title: translateSource("剪贴板内容"),
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

export async function toggleFloatingClipboardHistoryWindow(
  payload: FloatingClipboardHistoryPayload,
): Promise<void> {
  writeFloatingClipboardHistoryPayload(payload);
  const existing = await WebviewWindow.getByLabel(FLOATING_CLIPBOARD_WINDOW_LABEL);
  if (!existing) {
    await openFloatingClipboardHistoryWindow(payload);
    return;
  }

  if (await existing.isVisible()) {
    await existing.hide();
    return;
  }

  await emitTo(FLOATING_CLIPBOARD_WINDOW_LABEL, "floating-clipboard-refresh", payload);
  await existing.show();
  await existing.setFocus();
}

export function openExternalUrl(url: string): Promise<void> {
  return invoke<void>("open_external_url", { url });
}

export function openWindowsNetworkSettings(): Promise<void> {
  return invoke<void>("open_windows_network_settings");
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

export async function isMainWindowVisible(): Promise<boolean> {
  const window = getCurrentWindow();
  const [visible, minimized] = await Promise.all([window.isVisible(), window.isMinimized()]);
  return visible && !minimized;
}

export function onMainWindowFocusChanged(handler: () => void): Promise<UnlistenFn> {
  return getCurrentWindow().onFocusChanged(handler);
}

export function exitApp(): Promise<void> {
  return invoke<void>("exit_app");
}

export function onMainWindowCloseRequested(
  handler: (event: CloseRequestedEvent) => void | Promise<void>,
): Promise<UnlistenFn> {
  return getCurrentWindow().onCloseRequested(handler);
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

export function waitForPrimaryMouseRelease(): Promise<void> {
  return invoke<void>("wait_for_primary_mouse_release");
}

export function closeWindow(): Promise<void> {
  return getCurrentWindow().close();
}

export function hideWindow(): Promise<void> {
  return getCurrentWindow().hide();
}

type WindowGeometry = {
  size: { width: number; height: number };
  position: { x: number; y: number };
  maximized?: boolean;
};
const savedWindowGeometry: Partial<Record<AppWindowMode, WindowGeometry>> = {};
let activeWindowMode: AppWindowMode | null = null;
const FLOATING_BALL_POSITION_KEY = "copyshare:floating-ball-position";

async function rememberWindowGeometry(nextMode: AppWindowMode): Promise<void> {
  if (!activeWindowMode || activeWindowMode === nextMode) return;
  const window = getCurrentWindow();
  const [size, position, scale, maximized] = await Promise.all([
    window.innerSize(), window.outerPosition(), window.scaleFactor(), window.isMaximized(),
  ]);
  savedWindowGeometry[activeWindowMode] = {
    size: maximized && activeWindowMode === "main"
      ? savedWindowGeometry.main?.size ?? MAIN_WINDOW_BOUNDS
      : { width: size.width / scale, height: size.height / scale },
    position: { x: position.x, y: position.y },
    maximized,
  };
}

async function ballDockPosition(anchor: { x: number; y: number }): Promise<{ x: number; y: number } | undefined> {
  const monitors = await availableMonitors().catch(() => []);
  const monitor = monitors.find((item) => {
    const position = item.workArea?.position ?? item.position;
    const size = item.workArea?.size ?? item.size;
    return anchor.x >= position.x && anchor.x < position.x + size.width
      && anchor.y >= position.y && anchor.y < position.y + size.height;
  }) ?? await currentMonitor().catch(() => null) ?? monitors[0];
  if (!monitor) return undefined;
  return getFloatingBallDockPosition({
    position: monitor.workArea?.position ?? monitor.position,
    size: monitor.workArea?.size ?? monitor.size,
    scaleFactor: monitor.scaleFactor,
  }, anchor);
}

export async function enterBallWindow(position: "current" | "top-right" = "current"): Promise<void> {
  await rememberWindowGeometry("ball");
  const window = getCurrentWindow();
  const [sourcePosition, sourceSize, scale] = await Promise.all([
    window.outerPosition(), window.outerSize(), window.scaleFactor(),
  ]);
  let saved: { x: number; y: number } | undefined;
  try {
    const value = JSON.parse(localStorage.getItem(FLOATING_BALL_POSITION_KEY) ?? "null");
    if (Number.isFinite(value?.x) && Number.isFinite(value?.y)) saved = value;
  } catch { /* Fall back to the current monitor. */ }
  const monitor = await currentMonitor().catch(() => null);
  const monitorPosition = monitor?.workArea?.position ?? monitor?.position;
  const monitorSize = monitor?.workArea?.size ?? monitor?.size;
  const anchor = saved ? {
    x: saved.x + FLOATING_BALL_BOUNDS.width * scale / 2,
    y: saved.y,
  } : position === "top-right" && monitorPosition && monitorSize ? {
    x: monitorPosition.x + monitorSize.width - 1,
    y: monitorPosition.y + FLOATING_STARTUP_OFFSET.top * monitor!.scaleFactor,
  } : {
    x: sourcePosition.x + sourceSize.width / 2,
    y: sourcePosition.y + Math.min(sourceSize.height / 2, 48 * scale),
  };
  await window.setBackgroundColor(TRANSPARENT_WINDOW_BACKGROUND);
  await window.setAlwaysOnTop(true);
  if (await window.isMaximized()) await window.toggleMaximize();
  await window.setMaxSize(null);
  await window.setMinSize(new LogicalSize(FLOATING_BALL_BOUNDS.width, FLOATING_BALL_BOUNDS.height));
  await window.setResizable(false);
  await window.setSize(new LogicalSize(FLOATING_BALL_BOUNDS.width, FLOATING_BALL_BOUNDS.height));
  const dock = await ballDockPosition(anchor);
  if (dock) {
    await window.setPosition(new PhysicalPosition(dock.x, dock.y));
    try { localStorage.setItem(FLOATING_BALL_POSITION_KEY, JSON.stringify(dock)); }
    catch { /* Position remains usable for this session. */ }
  }
  await window.setShadow(false);
  await waitForWindowSize(FLOATING_BALL_BOUNDS);
  await invoke("set_floating_ball_shape", { enabled: true });
  activeWindowMode = "ball";
}

export async function dockBallWindow(): Promise<void> {
  if (activeWindowMode !== "ball") return;
  const window = getCurrentWindow();
  const [position, size] = await Promise.all([window.outerPosition(), window.outerSize()]);
  const dock = await ballDockPosition({ x: position.x + size.width / 2, y: position.y });
  if (!dock) return;
  await window.setPosition(new PhysicalPosition(dock.x, dock.y));
  try { localStorage.setItem(FLOATING_BALL_POSITION_KEY, JSON.stringify(dock)); }
  catch { /* Position remains usable for this session. */ }
}

export async function enterFloatingWindow(position: "cursor" | "top-right" = "cursor"): Promise<void> {
  await invoke("set_floating_ball_shape", { enabled: false });
  await rememberWindowGeometry("floating");
  const saved = savedWindowGeometry.floating;
  const bounds = saved?.size ?? FLOATING_WINDOW_BOUNDS;
  const window = getCurrentWindow();
  const size = new LogicalSize(
    bounds.width,
    bounds.height,
  );

  await window.setBackgroundColor(TRANSPARENT_WINDOW_BACKGROUND);
  await window.setAlwaysOnTop(true);
  await window.setMaxSize(null);
  await window.setMinSize(new LogicalSize(300, 320));
  await window.setResizable(true);
  await window.setSize(size);
  try {
    if (saved) {
      await window.setPosition(new PhysicalPosition(saved.position.x, saved.position.y));
    } else if (position === "top-right") {
      const monitor = await currentMonitor();
      if (monitor) {
        const next = getFloatingWindowTopRightPosition({
          position: monitor.workArea?.position ?? monitor.position,
          size: monitor.workArea?.size ?? monitor.size,
          scaleFactor: monitor.scaleFactor,
        });
        await window.setPosition(new PhysicalPosition(next.x, next.y));
      }
    } else {
      await moveFloatingWindowToCursor();
    }
  } catch (error) {
    console.warn("Unable to position floating window", error);
  }
  await window.setShadow(false);
  await waitForWindowSize(bounds);
  await window.setFocus();
  activeWindowMode = "floating";
}

export async function restoreMainWindow(): Promise<void> {
  await invoke("set_floating_ball_shape", { enabled: false });
  await rememberWindowGeometry("main");
  const saved = savedWindowGeometry.main;
  const bounds = saved?.size ?? MAIN_WINDOW_BOUNDS;
  const window = getCurrentWindow();

  await window.setBackgroundColor(TRANSPARENT_WINDOW_BACKGROUND);
  await window.setMaxSize(null);
  await window.setMinSize(
    new LogicalSize(MAIN_WINDOW_BOUNDS.minWidth, MAIN_WINDOW_BOUNDS.minHeight),
  );
  await window.setResizable(true);
  await window.setAlwaysOnTop(false);
  await window.setSize(
    new LogicalSize(bounds.width, bounds.height),
  );
  try {
    if (saved && !saved.maximized) await window.setPosition(new PhysicalPosition(saved.position.x, saved.position.y));
    else await moveMainWindowToCenter();
  } catch (error) {
    console.warn("Unable to move main window to center", error);
  }
  await waitForWindowSize(bounds);
  if (saved?.maximized) await window.toggleMaximize();
  await window.setFocus();
  activeWindowMode = "main";
}

export function onAppEvent<T>(
  eventName: AppEventName,
  callback: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<T>(eventName, (event) => callback(event.payload));
}

