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
  contentHash: string;
  contentType: ClipboardContentType;
  direction?: HistoryDirection;
  syncStatus: HistorySyncStatus;
  sourceDevice?: string;
  createdAt?: string;
  fileTransferId?: string;
  fileTransferFileId?: string;
  clipboardBatchId?: string;
  fileTransferStatus?: FileTransferStatus;
  isPinned?: boolean;
  pinnedAt?: string;
};

export const CLIPBOARD_PREVIEW_LIMIT = 20;
export const FLOATING_CLIPBOARD_PREVIEW_LIMIT = 20;
export const FLOATING_CLIPBOARD_HISTORY_LIMIT = 100;
export const CLIPBOARD_CATEGORIES = ["全部", "文本", "图片", "视频", "链接", "文件"] as const;

const FILE_SIZE_SUFFIX_PATTERN = /\s+\d+(?:\.\d+)?\s*(?:B|KB|MB|GB|TB)$/i;
const FILE_SUMMARY_PATTERN = /^(.*?)\s+(\d+(?:\.\d+)?\s*(?:B|KB|MB|GB|TB))$/i;
const LEGACY_CORRUPTED_MULTI_FILE_LABEL = "\u6d93\ue045\u6783\u6d60?";
const HTTP_URL_PATTERN = /https?:\/\/[^\s]+/i;
const VIDEO_FILE_PATTERN = /\.(mp4|mov|mkv|avi|webm|m4v|wmv)$/i;

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
  return item.contentType === "fileList"
    ? normalizeClipboardFileSummaryText(item.summary)
    : item.summary;
}

function normalizeClipboardFileSummaryText(text: string): string {
  return text.replace(LEGACY_CORRUPTED_MULTI_FILE_LABEL, "个文件 · ");
}

function floatingClipboardDedupKey(item: ClipboardPreviewItem): string {
  if (item.contentType === "text") {
    return `text:${item.text}`;
  }

  const contentHash = (item.contentHash ?? "").trim();
  return contentHash
    ? `${item.contentType}:hash:${contentHash}`
    : `${item.contentType}:id:${item.id}`;
}

export function stripSizeSuffix(text: string): string {
  return normalizeClipboardFileSummaryText(text)
    .replace(FILE_SIZE_SUFFIX_PATTERN, "")
    .replace(/\s*·\s*$/, "");
}

export function splitClipboardFileSummary(text: string): ClipboardFileSummary {
  const normalized = normalizeClipboardFileSummaryText(text).trim();
  const match = normalized.match(FILE_SUMMARY_PATTERN);
  if (!match) {
    return { name: normalized, size: null };
  }
  return {
    name: match[1].trim().replace(/\s*·\s*$/, ""),
    size: match[2].replace(/\s+/g, " ").trim(),
  };
}

export function getClipboardLinkUrl(text: string): string | null {
  return text.match(HTTP_URL_PATTERN)?.[0] ?? null;
}

export function shouldShowClipboardItemMore(
  element: Pick<HTMLElement, "scrollHeight" | "clientHeight" | "scrollWidth" | "clientWidth">,
): boolean {
  return element.scrollHeight > element.clientHeight + 1
    || element.scrollWidth > element.clientWidth + 1;
}

export function isClipboardVideoFile(item: Pick<ClipboardPreviewItem, "text" | "contentType">): boolean {
  if (item.contentType !== "fileList") {
    return false;
  }
  return VIDEO_FILE_PATTERN.test(splitClipboardFileSummary(item.text).name);
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
      contentHash: item.contentHash,
      contentType: item.contentType,
      direction: item.direction,
      sourceDevice: item.sourceDevice,
      syncStatus: syncStatus(item),
      createdAt: item.createdAt,
      fileTransferId: item.fileTransferId,
      fileTransferFileId: item.fileTransferFileId,
      clipboardBatchId: item.clipboardBatchId,
      fileTransferStatus: item.fileTransferStatus,
      isPinned: item.isPinned,
      pinnedAt: item.pinnedAt,
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
  const appPreviewItems = getRecentClipboardItems(appItems, FLOATING_CLIPBOARD_HISTORY_LIMIT);
  const appTextItems = new Map<string, ClipboardPreviewItem>();
  for (const item of appPreviewItems) {
    if (item.contentType === "text" && !appTextItems.has(item.text)) appTextItems.set(item.text, item);
  }
  const recentSystemItems = systemItems
    .map((item) => ({
      ...item,
      id: item.id,
      text: item.text.trim(),
      contentHash: item.contentHash,
      contentType: item.contentType,
      direction: item.direction,
      syncStatus: item.syncStatus ?? "unsynced",
      createdAt: item.createdAt,
      fileTransferId: item.fileTransferId,
      fileTransferFileId: item.fileTransferFileId,
      clipboardBatchId: item.clipboardBatchId,
      fileTransferStatus: item.fileTransferStatus,
    }))
    .filter((item) => item.text.length > 0)
    .map((item) => appTextItems.get(item.text) ?? item)
    .slice(0, limit);
  const mergedItems: ClipboardPreviewItem[] = [...recentSystemItems];

  for (const item of recentSystemItems) {
    seen.add(floatingClipboardDedupKey(item));
  }

  for (const item of appPreviewItems) {
    const dedupKey = floatingClipboardDedupKey(item);
    if (seen.has(dedupKey)) {
      continue;
    }

    mergedItems.push(item);
    seen.add(dedupKey);
  }

  return mergedItems.sort((left, right) => {
    if (Boolean(left.isPinned) !== Boolean(right.isPinned)) return left.isPinned ? -1 : 1;
    return left.isPinned && right.isPinned
      ? (Date.parse(right.pinnedAt ?? "") || 0) - (Date.parse(left.pinnedAt ?? "") || 0)
      : 0;
  }).slice(0, limit);
}
