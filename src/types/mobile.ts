export type MobileSessionMode = "sendToMobile" | "receiveFromMobile" | "bidirectional";
export type MobileSessionPhase =
  | "waiting"
  | "opened"
  | "copied"
  | "submitted"
  | "written"
  | "expired"
  | "closed";

export interface MobileClipboardTextItem {
  id: string;
  text: string;
  sourceDevice?: string;
}

export interface MobileSessionView {
  id: string;
  url: string;
  mode: MobileSessionMode;
  phase: MobileSessionPhase;
  expiresAt: string | null;
  remainingSeconds: number | null;
  summary: string;
  submittedSummary: string | null;
  contentItems: MobileClipboardTextItem[];
  submittedItems: MobileClipboardTextItem[];
}

