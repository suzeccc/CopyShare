import type { FileTransferStatus } from "@/types/fileTransfer";
import type {
  ClipboardContentType,
  HistoryDirection,
  HistoryItem,
  HistorySyncStatus,
} from "@/types/history";

export type ClipboardPreviewItem = {
  id: string;
  text: string;
  contentType: ClipboardContentType;
  direction?: HistoryDirection;
  syncStatus: HistorySyncStatus;
  sourceDevice?: string;
  createdAt?: string;
  fileTransferId?: string;
  fileTransferStatus?: FileTransferStatus;
};

export const CLIPBOARD_PREVIEW_LIMIT = 20;
export const FLOATING_CLIPBOARD_PREVIEW_LIMIT = 10;
export const CLIPBOARD_CATEGORIES = ["全部", "文本", "图片", "视频", "链接", "文件"] as const;

export type ClipboardCategory = (typeof CLIPBOARD_CATEGORIES)[number];

export type ClipboardDisplayType = {
  label: Exclude<ClipboardCategory, "全部">;
  icon: string;
  tone: "text" | "image" | "link" | "file" | "video";
};

export type ClipboardFileSummary = {
  name: string;
  size: string | null;
};

function previewText(item: HistoryItem): string {
  if (item.contentType === "text") {
    return item.content || item.summary;
  }
  if (item.contentType === "image") {
    return stripSizeSuffix(item.summary);
  }
  return item.summary;
}

export function stripSizeSuffix(text: string): string {
  return text.replace(/\s+\d+(?:\.\d+)?\s*(?:B|KB|MB|GB|TB)$/i, "");
}

export function splitClipboardFileSummary(text: string): ClipboardFileSummary {
  const normalized = text.trim();
  const match = normalized.match(/^(.*?)\s+(\d+(?:\.\d+)?\s*(?:B|KB|MB|GB|TB))$/i);
  if (!match) {
    return { name: normalized, size: null };
  }
  return {
    name: match[1].trim(),
    size: match[2].replace(/\s+/g, " ").trim(),
  };
}

export function getClipboardLinkUrl(text: string): string | null {
  return text.match(/https?:\/\/[^\s]+/i)?.[0] ?? null;
}

export function isClipboardVideoFile(item: Pick<ClipboardPreviewItem, "text" | "contentType">): boolean {
  if (item.contentType !== "fileList") {
    return false;
  }
  return /\.(mp4|mov|mkv|avi|webm|m4v|wmv)$/i.test(splitClipboardFileSummary(item.text).name);
}

function syncStatus(item: Pick<HistoryItem, "syncStatus">): HistorySyncStatus {
  return item.syncStatus ?? "synced";
}

export function getRecentClipboardItems(
  items: HistoryItem[],
  limit = CLIPBOARD_PREVIEW_LIMIT,
): ClipboardPreviewItem[] {
  return items
    .map((item) => ({
      id: item.id,
      text: previewText(item).trim(),
      contentType: item.contentType,
      direction: item.direction,
      sourceDevice: item.sourceDevice,
      syncStatus: syncStatus(item),
      createdAt: item.createdAt,
      fileTransferId: item.fileTransferId,
      fileTransferStatus: item.fileTransferStatus,
    }))
    .filter((item) => item.text.length > 0)
    .slice(0, limit);
}

export function getClipboardDisplayType(item: ClipboardPreviewItem): ClipboardDisplayType {
  if (item.contentType === "image") {
    return { label: "图片", icon: "图", tone: "image" };
  }
  if (isClipboardVideoFile(item)) {
    return { label: "视频", icon: "视", tone: "video" };
  }
  if (item.contentType === "fileList") {
    return { label: "文件", icon: "文", tone: "file" };
  }
  if (looksLikeLink(item.text)) {
    return { label: "链接", icon: "链", tone: "link" };
  }
  return { label: "文本", icon: "字", tone: "text" };
}

export function filterClipboardItems(
  items: ClipboardPreviewItem[],
  category: ClipboardCategory,
  query: string,
): ClipboardPreviewItem[] {
  const normalizedQuery = query.trim().toLowerCase();
  return items.filter((item) => {
    const type = getClipboardDisplayType(item);
    if (category !== "全部" && type.label !== category) {
      return false;
    }
    if (!normalizedQuery) {
      return true;
    }
    return [item.text, item.sourceDevice ?? "", type.label]
      .join(" ")
      .toLowerCase()
      .includes(normalizedQuery);
  });
}

function looksLikeLink(text: string): boolean {
  return getClipboardLinkUrl(text) !== null;
}

export function getFloatingClipboardItems(
  systemItems: ClipboardPreviewItem[],
  appItems: HistoryItem[],
  limit = FLOATING_CLIPBOARD_PREVIEW_LIMIT,
): ClipboardPreviewItem[] {
  const seen = new Set<string>();
  const recentSystemItems = systemItems
    .map((item) => ({
      id: item.id,
      text: item.text.trim(),
      contentType: item.contentType,
      direction: item.direction,
      syncStatus: item.syncStatus ?? "unsynced",
      createdAt: item.createdAt,
      fileTransferId: item.fileTransferId,
      fileTransferStatus: item.fileTransferStatus,
    }))
    .filter((item) => item.text.length > 0)
    .slice(0, limit);
  const mergedItems: ClipboardPreviewItem[] = [...recentSystemItems];

  for (const item of recentSystemItems) {
    seen.add(item.text);
  }

  for (const item of getRecentClipboardItems(appItems, limit)) {
    if (mergedItems.length >= limit) {
      break;
    }

    if (seen.has(item.text)) {
      continue;
    }

    mergedItems.push(item);
    seen.add(item.text);
  }

  return mergedItems;
}
