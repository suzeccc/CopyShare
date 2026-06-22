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

const items = getRecentClipboardItems([
  historyItem({ id: "1", summary: "摘要一", content: "  完整内容一  " }),
  historyItem({ id: "2", summary: "摘要二" }),
  historyItem({ id: "3", summary: "   ", content: "完整内容三" }),
  historyItem({ id: "4", summary: "摘要四", content: "完整内容四" }),
  historyItem({ id: "5", summary: "摘要五", content: "完整内容五" }),
  historyItem({ id: "6", summary: "摘要六", content: "完整内容六" }),
]);

assert.deepEqual(items, [
  { id: "1", text: "完整内容一" },
  { id: "2", text: "摘要二" },
  { id: "3", text: "完整内容三" },
  { id: "4", text: "完整内容四" },
  { id: "5", text: "完整内容五" },
]);

assert.deepEqual(getRecentClipboardItems([], 3), []);
assert.deepEqual(getRecentClipboardItems([historyItem({ id: "empty", summary: " " })], 3), []);

assert.deepEqual(
  getFloatingClipboardItems(
    [
      { id: "system-1", text: "Win+V 第一条" },
      { id: "system-2", text: "Win+V 第二条" },
      { id: "system-3", text: "Win+V 第三条" },
      { id: "system-4", text: "Win+V 第四条" },
      { id: "system-5", text: "Win+V 第五条" },
      { id: "system-6", text: "Win+V 第六条" },
    ],
    [historyItem({ id: "app-1", summary: "程序历史" })],
  ),
  [
    { id: "system-1", text: "Win+V 第一条" },
    { id: "system-2", text: "Win+V 第二条" },
    { id: "system-3", text: "Win+V 第三条" },
    { id: "system-4", text: "Win+V 第四条" },
    { id: "system-5", text: "Win+V 第五条" },
  ],
);

assert.deepEqual(
  getFloatingClipboardItems([], [historyItem({ id: "app-1", summary: "程序历史" })]),
  [{ id: "app-1", text: "程序历史" }],
);

assert.deepEqual(
  getFloatingClipboardItems(
    [
      { id: "system-1", text: "系统一" },
      { id: "system-2", text: "系统二" },
    ],
    [
      historyItem({ id: "app-1", summary: "程序一" }),
      historyItem({ id: "app-2", summary: "系统二" }),
    ],
  ),
  [
    { id: "system-1", text: "系统一" },
    { id: "system-2", text: "系统二" },
    { id: "app-1", text: "程序一" },
  ],
);
