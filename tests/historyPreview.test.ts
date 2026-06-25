import assert from "node:assert/strict";

import { getFloatingClipboardItems, getRecentClipboardItems } from "../src/lib/historyPreview.ts";
import type { HistoryItem } from "../src/types/history.ts";

function historyItem(partial: Partial<HistoryItem>): HistoryItem {
  return {
    id: partial.id ?? crypto.randomUUID(),
    direction: partial.direction ?? "local",
    sourceDevice: partial.sourceDevice ?? "Device",
    summary: partial.summary ?? "",
    content: partial.content,
    contentType: partial.contentType ?? "text",
    success: true,
    createdAt: partial.createdAt ?? new Date().toISOString(),
  };
}

function systemItem(index: number) {
  return { id: `system-${index}`, text: `System ${index}`, contentType: "text" as const };
}

const items = getRecentClipboardItems([
  historyItem({ id: "1", summary: "Summary one", content: "  Full one  ", sourceDevice: "Office-PC" }),
  historyItem({ id: "2", summary: "Summary two", sourceDevice: "Laptop" }),
  historyItem({ id: "3", summary: "   ", content: "Full three", sourceDevice: "Desktop" }),
  historyItem({ id: "4", summary: "Summary four", content: "Full four", sourceDevice: "Phone" }),
  historyItem({ id: "5", summary: "Summary five", content: "Full five", sourceDevice: "Tablet" }),
  historyItem({ id: "6", summary: "Summary six", content: "Full six", sourceDevice: "NAS" }),
]);

assert.deepEqual(items, [
  { id: "1", text: "Full one", contentType: "text", sourceDevice: "Office-PC" },
  { id: "2", text: "Summary two", contentType: "text", sourceDevice: "Laptop" },
  { id: "3", text: "Full three", contentType: "text", sourceDevice: "Desktop" },
  { id: "4", text: "Full four", contentType: "text", sourceDevice: "Phone" },
  { id: "5", text: "Full five", contentType: "text", sourceDevice: "Tablet" },
]);

assert.deepEqual(
  getRecentClipboardItems([
    historyItem({ id: "image-1", summary: "图片 1089 KB", content: "base64", contentType: "image" }),
  ]),
  [{ id: "image-1", text: "图片 1089 KB", contentType: "image", sourceDevice: "Device" }],
);

assert.deepEqual(getRecentClipboardItems([], 3), []);
assert.deepEqual(getRecentClipboardItems([historyItem({ id: "empty", summary: " " })], 3), []);

assert.equal(getFloatingClipboardItems(Array.from({ length: 11 }, (_, index) => systemItem(index + 1)), []).length, 10);

assert.deepEqual(
  getFloatingClipboardItems(
    [
      { id: "system-1", text: "WinV one", contentType: "text" },
      { id: "system-2", text: "WinV two", contentType: "text" },
      { id: "system-3", text: "WinV three", contentType: "text" },
      { id: "system-4", text: "WinV four", contentType: "text" },
      { id: "system-5", text: "WinV five", contentType: "text" },
      { id: "system-6", text: "WinV six", contentType: "text" },
    ],
    [historyItem({ id: "app-1", summary: "App history", sourceDevice: "Office-PC" })],
  ),
  [
    { id: "system-1", text: "WinV one", contentType: "text" },
    { id: "system-2", text: "WinV two", contentType: "text" },
    { id: "system-3", text: "WinV three", contentType: "text" },
    { id: "system-4", text: "WinV four", contentType: "text" },
    { id: "system-5", text: "WinV five", contentType: "text" },
    { id: "system-6", text: "WinV six", contentType: "text" },
    { id: "app-1", text: "App history", contentType: "text", sourceDevice: "Office-PC" },
  ],
);

assert.deepEqual(
  getFloatingClipboardItems([], [historyItem({ id: "app-1", summary: "App history", sourceDevice: "Office-PC" })]),
  [{ id: "app-1", text: "App history", contentType: "text", sourceDevice: "Office-PC" }],
);

assert.deepEqual(
  getFloatingClipboardItems(
    [
      { id: "system-1", text: "System one", contentType: "text" },
      { id: "system-2", text: "System two", contentType: "text" },
    ],
    [
      historyItem({ id: "app-1", summary: "App one", sourceDevice: "Office-PC" }),
      historyItem({ id: "app-2", summary: "System two", sourceDevice: "Office-PC" }),
    ],
  ),
  [
    { id: "system-1", text: "System one", contentType: "text" },
    { id: "system-2", text: "System two", contentType: "text" },
    { id: "app-1", text: "App one", contentType: "text", sourceDevice: "Office-PC" },
  ],
);
