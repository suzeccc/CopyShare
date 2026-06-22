import type { HistoryItem } from "@/types/history";

export type ClipboardPreviewItem = {
  id: string;
  text: string;
};

export const CLIPBOARD_PREVIEW_LIMIT = 5;

export function getRecentClipboardItems(
  items: HistoryItem[],
  limit = CLIPBOARD_PREVIEW_LIMIT,
): ClipboardPreviewItem[] {
  return items
    .map((item) => ({
      id: item.id,
      text: (item.content || item.summary).trim(),
    }))
    .filter((item) => item.text.length > 0)
    .slice(0, limit);
}

export function getFloatingClipboardItems(
  systemItems: ClipboardPreviewItem[],
  appItems: HistoryItem[],
  limit = CLIPBOARD_PREVIEW_LIMIT,
): ClipboardPreviewItem[] {
  const seen = new Set<string>();
  const recentSystemItems = systemItems
    .map((item) => ({ id: item.id, text: item.text.trim() }))
    .filter((item) => item.text.length > 0)
    .slice(0, limit);
  const mergedItems = [...recentSystemItems];

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
