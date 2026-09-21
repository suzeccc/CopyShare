import type { FileTransferStatus } from "@/types/fileTransfer";
import type { ClipboardContentType, HistoryDirection } from "@/types/history";

export type ActivityLogCategory = "clipboard" | "device" | "file" | "system";
export type ActivityLogStatus = "success" | "warning" | "error" | "info";
export type ActivityLogDirection = "send" | "receive" | "none";
export type ActivityLogFilter = "all" | "issues" | "device" | "file";

export interface ActivityLogEntry {
  id: string;
  category: ActivityLogCategory;
  status: ActivityLogStatus;
  direction: ActivityLogDirection;
  title: string;
  deviceName?: string;
  contentType?: ClipboardContentType;
  sizeBytes?: number;
  detail?: string;
  transferId?: string;
  transferStatus?: FileTransferStatus;
  createdAt: string;
}

export interface PersistedActivityLog {
  version: 1;
  seededFromHistory: boolean;
  items: ActivityLogEntry[];
}

export interface ActivityHistoryMetadata {
  id: string;
  direction: HistoryDirection;
  sourceDevice: string;
  content?: string;
  contentType: ClipboardContentType;
  syncStatus: "synced" | "unsynced";
  fileTransferId?: string;
  fileTransferStatus?: FileTransferStatus;
  success: boolean;
  createdAt: string;
}
