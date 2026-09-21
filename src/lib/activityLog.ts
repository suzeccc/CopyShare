import type {
  ActivityHistoryMetadata,
  ActivityLogEntry,
  ActivityLogFilter,
  ActivityLogStatus,
  PersistedActivityLog,
} from "@/types/activityLog";
import type { DeviceInfo } from "@/types/device";
import type { FileTransferStatus, FileTransferTask } from "@/types/fileTransfer";
import type { AppStatus } from "@/types/status";

export const ACTIVITY_LOG_STORAGE_KEY = "copyshare.sync-activity-log.v1";
export const ACTIVITY_LOG_LIMIT = 500;

type DeviceActivity = "connected" | "disconnected" | "rejected";

export function activityFromHistory(item: ActivityHistoryMetadata): ActivityLogEntry | null {
  if (item.direction === "local" && item.syncStatus !== "synced") return null;

  const direction = item.direction === "local" ? "send" : "receive";
  const sizeBytes = item.contentType === "text" && item.content
    ? new TextEncoder().encode(item.content).byteLength
    : undefined;
  const transferStatus = item.fileTransferStatus;
  const isFileTransfer = Boolean(item.fileTransferId);

  return {
    id: isFileTransfer ? `file:${item.fileTransferId}` : `clipboard:${item.id}`,
    category: isFileTransfer ? "file" : "clipboard",
    status: transferStatus ? fileTransferActivityStatus(transferStatus) : item.success ? "success" : "error",
    direction,
    title: isFileTransfer
      ? fileTransferActivityTitle(direction, transferStatus ?? "transferring")
      : direction === "send" ? "已发送剪贴板" : "已接收剪贴板",
    deviceName: direction === "receive" ? item.sourceDevice : undefined,
    contentType: item.contentType,
    sizeBytes,
    transferId: item.fileTransferId,
    transferStatus,
    createdAt: item.createdAt,
  };
}

export function activityFromFileTransfer(task: FileTransferTask): ActivityLogEntry {
  const direction = task.direction === "send" ? "send" : "receive";
  return {
    id: `file:${task.transferId}`,
    category: "file",
    status: fileTransferActivityStatus(task.status),
    direction,
    title: fileTransferActivityTitle(direction, task.status),
    deviceName: task.peerDeviceName || task.peerDeviceId,
    contentType: "fileList",
    sizeBytes: task.totalSize,
    detail: task.error || undefined,
    transferId: task.transferId,
    transferStatus: task.status,
    createdAt: task.completedAt || new Date().toISOString(),
  };
}

export function activityFromDevice(device: DeviceInfo, activity: DeviceActivity): ActivityLogEntry {
  const view = {
    connected: { title: device.trusted ? "设备已连接" : "设备等待信任确认", status: "success" },
    disconnected: { title: "设备已断开", status: "warning" },
    rejected: { title: "设备连接已拒绝", status: "warning" },
  }[activity] as { title: string; status: ActivityLogStatus };

  return {
    id: uniqueActivityId(`device:${device.id}:${activity}`),
    category: "device",
    status: view.status,
    direction: "none",
    title: view.title,
    deviceName: device.name || device.id,
    createdAt: new Date().toISOString(),
  };
}

export function activityFromSyncStatus(status: AppStatus): ActivityLogEntry {
  const view = status.state === "running"
    ? { title: "同步服务已启动", status: "success" as const }
    : status.state === "error"
      ? { title: "同步服务异常", status: "error" as const }
      : { title: "同步服务已停止", status: "info" as const };

  return {
    id: uniqueActivityId(`system:${status.state}`),
    category: "system",
    status: view.status,
    direction: "none",
    title: view.title,
    detail: status.state === "error" ? status.message || undefined : undefined,
    createdAt: new Date().toISOString(),
  };
}

export function activityFromSyncError(message: string): ActivityLogEntry {
  return {
    id: uniqueActivityId("system:error"),
    category: "system",
    status: "error",
    direction: "none",
    title: "同步发生异常",
    detail: message,
    createdAt: new Date().toISOString(),
  };
}

export function upsertActivity(items: ActivityLogEntry[], entry: ActivityLogEntry): ActivityLogEntry[] {
  return [entry, ...items.filter((item) => item.id !== entry.id)]
    .sort((left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt))
    .slice(0, ACTIVITY_LOG_LIMIT);
}

export function seedActivitiesFromHistory(items: ActivityHistoryMetadata[]): ActivityLogEntry[] {
  return items.reduce<ActivityLogEntry[]>((activities, item) => {
    const entry = activityFromHistory(item);
    return entry ? upsertActivity(activities, entry) : activities;
  }, []);
}

export function filterActivities(items: ActivityLogEntry[], filter: ActivityLogFilter): ActivityLogEntry[] {
  if (filter === "issues") return items.filter((item) => item.status === "warning" || item.status === "error");
  if (filter === "device") return items.filter((item) => item.category === "device");
  if (filter === "file") return items.filter((item) => item.category === "file");
  return items;
}

export function canResumeActivity(entry: ActivityLogEntry): boolean {
  return entry.category === "file"
    && Boolean(entry.transferId)
    && (entry.transferStatus === "paused" || entry.transferStatus === "waitingForPeer");
}

export function activityContentTypeLabel(entry: ActivityLogEntry): string | null {
  if (entry.category === "file") return "文件传输";
  if (entry.contentType === "text") return "文本";
  if (entry.contentType === "image") return "图片";
  if (entry.contentType === "fileList") return "文件";
  return null;
}

export function activityStatusLabel(status: ActivityLogStatus): string {
  return { success: "成功", warning: "需注意", error: "失败", info: "信息" }[status];
}

export function formatActivitySize(size?: number): string | null {
  if (size === undefined) return null;
  if (size < 1024) return `${size} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = size / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(value >= 10 ? 1 : 2)} ${units[unitIndex]}`;
}

export function formatActivityTimestamp(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const today = new Date();
  const sameDay = date.getFullYear() === today.getFullYear()
    && date.getMonth() === today.getMonth()
    && date.getDate() === today.getDate();
  return new Intl.DateTimeFormat("zh-CN", sameDay
    ? { hour: "2-digit", minute: "2-digit", second: "2-digit" }
    : { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" }).format(date);
}

export function readPersistedActivityLog(raw: string | null): PersistedActivityLog | null {
  if (!raw) return null;
  try {
    const value = JSON.parse(raw) as Partial<PersistedActivityLog>;
    if (value.version !== 1 || !Array.isArray(value.items)) return null;
    return {
      version: 1,
      seededFromHistory: Boolean(value.seededFromHistory),
      items: value.items
        .map(sanitizePersistedEntry)
        .filter((item): item is ActivityLogEntry => Boolean(item))
        .slice(0, ACTIVITY_LOG_LIMIT),
    };
  } catch {
    return null;
  }
}

function sanitizePersistedEntry(value: unknown): ActivityLogEntry | null {
  if (!value || typeof value !== "object") return null;
  const entry = value as Partial<ActivityLogEntry>;
  if (
    typeof entry.id !== "string"
    || typeof entry.title !== "string"
    || typeof entry.createdAt !== "string"
    || !["clipboard", "device", "file", "system"].includes(entry.category ?? "")
    || !["success", "warning", "error", "info"].includes(entry.status ?? "")
    || !["send", "receive", "none"].includes(entry.direction ?? "")
  ) return null;

  return {
    id: entry.id,
    category: entry.category as ActivityLogEntry["category"],
    status: entry.status as ActivityLogEntry["status"],
    direction: entry.direction as ActivityLogEntry["direction"],
    title: entry.title,
    deviceName: typeof entry.deviceName === "string" ? entry.deviceName : undefined,
    contentType: ["text", "image", "fileList"].includes(entry.contentType ?? "")
      ? entry.contentType
      : undefined,
    sizeBytes: typeof entry.sizeBytes === "number" ? entry.sizeBytes : undefined,
    detail: typeof entry.detail === "string" ? entry.detail : undefined,
    transferId: typeof entry.transferId === "string" ? entry.transferId : undefined,
    transferStatus: entry.transferStatus,
    createdAt: entry.createdAt,
  };
}

function fileTransferActivityStatus(status: FileTransferStatus): ActivityLogStatus {
  if (status === "completed") return "success";
  if (status === "failed") return "error";
  if (["waitingForPeer", "retrying", "paused", "rejected", "canceled"].includes(status)) return "warning";
  return "info";
}

function fileTransferActivityTitle(direction: "send" | "receive", status: FileTransferStatus): string {
  if (status === "completed") return direction === "send" ? "文件发送完成" : "文件接收完成";
  if (status === "failed") return "文件传输失败";
  if (status === "waitingForPeer") return "等待对方设备上线";
  if (status === "retrying") return "正在恢复文件传输";
  if (status === "paused") return "文件传输已暂停";
  if (status === "rejected") return "文件传输已拒绝";
  if (status === "canceled") return "文件传输已取消";
  return direction === "send" ? "正在发送文件" : "正在接收文件";
}

function uniqueActivityId(prefix: string): string {
  const random = globalThis.crypto?.randomUUID?.() ?? Math.random().toString(36).slice(2);
  return `${prefix}:${Date.now()}:${random}`;
}
