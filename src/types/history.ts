export type HistoryDirection = "local" | "remote";

export interface HistoryItem {
  id: string;
  direction: HistoryDirection;
  sourceDevice: string;
  summary: string;
  content?: string;
  contentType: "text";
  success: boolean;
  createdAt: string;
}
