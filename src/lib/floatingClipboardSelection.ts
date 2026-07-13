import type { ClipboardPreviewItem } from "@/lib/historyPreview";

export function resolveFloatingClipboardSelection(
  items: ClipboardPreviewItem[],
  selected: ClipboardPreviewItem | null,
): ClipboardPreviewItem | null {
  if (!selected) return null;
  return items.find((item) => item.id === selected.id) ?? null;
}
