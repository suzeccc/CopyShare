import type { FileTransferStatus } from "@/types/fileTransfer";

export type HistoryDirection = "local" | "remote";
export type ClipboardContentType = "text" | "image" | "fileList";
export type HistorySyncStatus = "synced" | "unsynced";

export interface HistoryItem {
  id: string;
  direction: HistoryDirection;
  sourceDevice: string;
  summary: string;
  content?: string;
  contentType: ClipboardContentType;
  syncStatus: HistorySyncStatus;
  fileTransferId?: string;
  fileTransferStatus?: FileTransferStatus;
  success: boolean;
  createdAt: string;
}
