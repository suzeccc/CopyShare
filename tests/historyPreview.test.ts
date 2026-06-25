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
    contentType: "text",
    success: true,
    createdAt: partial.createdAt ?? new Date().toISOString(),
  };
}

function systemItem(index: number) {
  return { id: `system-${index}`, text: `System ${index}` };
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
  { id: "1", text: "Full one", sourceDevice: "Office-PC" },
  { id: "2", text: "Summary two", sourceDevice: "Laptop" },
  { id: "3", text: "Full three", sourceDevice: "Desktop" },
  { id: "4", text: "Full four", sourceDevice: "Phone" },
  { id: "5", text: "Full five", sourceDevice: "Tablet" },
]);

assert.deepEqual(getRecentClipboardItems([], 3), []);
assert.deepEqual(getRecentClipboardItems([historyItem({ id: "empty", summary: " " })], 3), []);

assert.equal(getFloatingClipboardItems(Array.from({ length: 11 }, (_, index) => systemItem(index + 1)), []).length, 10);

assert.deepEqual(
  getFloatingClipboardItems(
    [
      { id: "system-1", text: "WinV one" },
      { id: "system-2", text: "WinV two" },
      { id: "system-3", text: "WinV three" },
      { id: "system-4", text: "WinV four" },
      { id: "system-5", text: "WinV five" },
      { id: "system-6", text: "WinV six" },
    ],
    [historyItem({ id: "app-1", summary: "App history", sourceDevice: "Office-PC" })],
  ),
  [
    { id: "system-1", text: "WinV one" },
    { id: "system-2", text: "WinV two" },
    { id: "system-3", text: "WinV three" },
    { id: "system-4", text: "WinV four" },
    { id: "system-5", text: "WinV five" },
    { id: "system-6", text: "WinV six" },
    { id: "app-1", text: "App history", sourceDevice: "Office-PC" },
  ],
);

assert.deepEqual(
  getFloatingClipboardItems([], [historyItem({ id: "app-1", summary: "App history", sourceDevice: "Office-PC" })]),
  [{ id: "app-1", text: "App history", sourceDevice: "Office-PC" }],
);

assert.deepEqual(
  getFloatingClipboardItems(
    [
      { id: "system-1", text: "System one" },
      { id: "system-2", text: "System two" },
    ],
    [
      historyItem({ id: "app-1", summary: "App one", sourceDevice: "Office-PC" }),
      historyItem({ id: "app-2", summary: "System two", sourceDevice: "Office-PC" }),
    ],
  ),
  [
    { id: "system-1", text: "System one" },
    { id: "system-2", text: "System two" },
    { id: "app-1", text: "App one", sourceDevice: "Office-PC" },
  ],
);
