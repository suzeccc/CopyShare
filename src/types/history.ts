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
  success: boolean;
  createdAt: string;
}
