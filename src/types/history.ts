export type HistoryDirection = "local" | "remote";
export type ClipboardContentType = "text" | "image" | "fileList";

export interface HistoryItem {
  id: string;
  direction: HistoryDirection;
  sourceDevice: string;
  summary: string;
  content?: string;
  contentType: ClipboardContentType;
  success: boolean;
  createdAt: string;
}
