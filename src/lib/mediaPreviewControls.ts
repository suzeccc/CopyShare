import { isClipboardVideoFile, type ClipboardPreviewItem } from "./historyPreview.ts";

export function mediaPreviewItems(items: ClipboardPreviewItem[], kind: "image" | "video") {
  return items.filter(item => kind === "image" ? item.contentType === "image"
    : isClipboardVideoFile(item) && (item.direction !== "remote" || item.fileTransferStatus === "completed"));
}

export function adjacentMediaPreviewItem(items: ClipboardPreviewItem[], id: string, direction: -1 | 1) {
  const index = items.findIndex(item => item.id === id);
  return index < 0 ? undefined : items[index + direction];
}

export function nextVideoPlaybackRate(rate: number, direction: -1 | 1) {
  return Math.min(3, Math.max(0.25, Math.round((rate + direction * 0.25) * 100) / 100));
}
