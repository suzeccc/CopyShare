import type { ClipboardContentType } from "@/types/history";

export type LibraryRole = "saved" | "snippet";
export type LibraryAssetKind = "image" | "file";
export type LibraryView = "all" | "pinned" | "snippets";
export type LibraryContentFilter = "all" | ClipboardContentType;

export interface LibraryAssetRef {
  assetId: string;
  kind: LibraryAssetKind;
  fileName: string;
  relativePath: string;
  sha256: string;
  size: number;
}

export interface LibraryItem {
  id: string;
  role: LibraryRole;
  contentType: ClipboardContentType;
  title: string;
  content: string;
  summary: string;
  assets: LibraryAssetRef[];
  sourceHistoryId: string | null;
  sourceContentHash: string | null;
  sourceDevice: string;
  contentHash: string;
  tags: string[];
  note: string;
  isPinned: boolean;
  pinOrder: number | null;
  createdAt: string;
  updatedAt: string;
}

export interface LibrarySnapshot {
  items: LibraryItem[];
  warning: string | null;
}

export interface LibraryItemUpdate {
  title: string;
  content?: string | null;
  tags: string[];
  note: string;
}

export interface CreateSnippetInput {
  title: string;
  content: string;
  tags: string[];
  note: string;
}
