import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  CLIPBOARD_PREVIEW_LIMIT,
  FLOATING_CLIPBOARD_HISTORY_LIMIT,
  FLOATING_CLIPBOARD_PREVIEW_LIMIT,
  CLIPBOARD_CATEGORIES,
  filterClipboardItems,
  getClipboardLinkUrl,
  getClipboardDisplayType,
  getFloatingClipboardItems,
  getRecentClipboardItems,
  shouldShowClipboardItemMore,
  splitClipboardFileSummary,
} from "../src/lib/historyPreview.ts";
import type { HistoryItem } from "../src/types/history.ts";

function historyItem(partial: Partial<HistoryItem>): HistoryItem {
  return {
    id: partial.id ?? crypto.randomUUID(),
    direction: partial.direction ?? "local",
    sourceDevice: partial.sourceDevice ?? "Device",
    summary: partial.summary ?? "",
    content: partial.content,
    contentType: partial.contentType ?? "text",
    syncStatus: partial.syncStatus ?? "synced",
    success: true,
    createdAt: partial.createdAt ?? new Date().toISOString(),
  };
}

function systemItem(index: number) {
  return { id: `system-${index}`, text: `System ${index}`, contentType: "text" as const, syncStatus: "unsynced" as const };
}

function stripCreatedAt<T>(value: T): T {
  return JSON.parse(JSON.stringify(value, (key, item) =>
    key === "createdAt" || key === "direction" ? undefined : item,
  ));
}

const historyPreviewSource = readFileSync("src/lib/historyPreview.ts", "utf8");

assert.equal(CLIPBOARD_PREVIEW_LIMIT, 20);
assert.equal(FLOATING_CLIPBOARD_PREVIEW_LIMIT, 10);
assert.equal(FLOATING_CLIPBOARD_HISTORY_LIMIT, 50);
assert.doesNotMatch(historyPreviewSource, /FLOATING_CLIPBOARD_MORE_LIMIT/);
assert.deepEqual(CLIPBOARD_CATEGORIES, ["全部", "文本", "图片", "视频", "链接", "文件"]);

assert.deepEqual(splitClipboardFileSummary("茶话间.lnk 897 B"), {
  name: "茶话间.lnk",
  size: "897 B",
});
assert.deepEqual(splitClipboardFileSummary("安装包.zip 1.25 GB"), {
  name: "安装包.zip",
  size: "1.25 GB",
});
assert.deepEqual(splitClipboardFileSummary("没有大小的文件.txt"), {
  name: "没有大小的文件.txt",
  size: null,
});

assert.equal(
  shouldShowClipboardItemMore({ id: "short", text: "短文本", contentType: "text", syncStatus: "synced" }),
  false,
);
assert.equal(
  shouldShowClipboardItemMore({ id: "multi", text: "第一行\n第二行", contentType: "text", syncStatus: "synced" }),
  true,
);
assert.equal(
  shouldShowClipboardItemMore({
    id: "long-file",
    text: "very-long-copyshare-preview-file-name-that-will-be-clipped.png 25.2 KB",
    contentType: "image",
    syncStatus: "synced",
  }),
  false,
);
assert.equal(
  shouldShowClipboardItemMore({
    id: "compact-long-text",
    text: "定位20条上限应该落在哪个数据流，再写失败测试复现当前超过显示限制的文本",
    contentType: "text",
    syncStatus: "synced",
  }, { textLimit: 18 }),
  true,
);

assert.equal(getClipboardLinkUrl("https://copyshare.example/download"), "https://copyshare.example/download");
assert.equal(
  getClipboardLinkUrl("See https://copyshare.example/docs now"),
  "https://copyshare.example/docs",
);
assert.equal(getClipboardLinkUrl("plain clipboard text"), null);

const items = getRecentClipboardItems([
  historyItem({ id: "1", summary: "Summary one", content: "  Full one  ", sourceDevice: "Office-PC" }),
  historyItem({ id: "2", summary: "Summary two", sourceDevice: "Laptop" }),
  historyItem({ id: "3", summary: "   ", content: "Full three", sourceDevice: "Desktop" }),
  historyItem({ id: "4", summary: "Summary four", content: "Full four", sourceDevice: "Phone" }),
  historyItem({ id: "5", summary: "Summary five", content: "Full five", sourceDevice: "Tablet" }),
  historyItem({ id: "6", summary: "Summary six", content: "Full six", sourceDevice: "NAS" }),
  historyItem({ id: "7", summary: "Summary seven", content: "Full seven", sourceDevice: "Mini" }),
  historyItem({ id: "8", summary: "Summary eight", content: "Full eight", sourceDevice: "Studio" }),
  historyItem({ id: "9", summary: "Summary nine", content: "Full nine", sourceDevice: "Air" }),
  historyItem({ id: "10", summary: "Summary ten", content: "Full ten", sourceDevice: "Pro" }),
  historyItem({ id: "11", summary: "Summary eleven", content: "Full eleven", sourceDevice: "Spare" }),
]);

assert.equal(items[0]?.direction, "local");
assert.equal(
  getRecentClipboardItems([
    historyItem({ id: "remote-file", direction: "remote", summary: "remote.zip 1 MB", contentType: "fileList" }),
  ])[0]?.direction,
  "remote",
);

assert.deepEqual(stripCreatedAt(items), [
  { id: "1", text: "Full one", contentType: "text", sourceDevice: "Office-PC", syncStatus: "synced" },
  { id: "2", text: "Summary two", contentType: "text", sourceDevice: "Laptop", syncStatus: "synced" },
  { id: "3", text: "Full three", contentType: "text", sourceDevice: "Desktop", syncStatus: "synced" },
  { id: "4", text: "Full four", contentType: "text", sourceDevice: "Phone", syncStatus: "synced" },
  { id: "5", text: "Full five", contentType: "text", sourceDevice: "Tablet", syncStatus: "synced" },
  { id: "6", text: "Full six", contentType: "text", sourceDevice: "NAS", syncStatus: "synced" },
  { id: "7", text: "Full seven", contentType: "text", sourceDevice: "Mini", syncStatus: "synced" },
  { id: "8", text: "Full eight", contentType: "text", sourceDevice: "Studio", syncStatus: "synced" },
  { id: "9", text: "Full nine", contentType: "text", sourceDevice: "Air", syncStatus: "synced" },
  { id: "10", text: "Full ten", contentType: "text", sourceDevice: "Pro", syncStatus: "synced" },
  { id: "11", text: "Full eleven", contentType: "text", sourceDevice: "Spare", syncStatus: "synced" },
]);

assert.deepEqual(
  stripCreatedAt(getRecentClipboardItems([
    historyItem({ id: "image-1", summary: "图片 1089 KB", content: "base64", contentType: "image" }),
  ])),
  [{ id: "image-1", text: "图片 1089 KB", contentType: "image", sourceDevice: "Device", syncStatus: "synced" }],
);

assert.equal(
  getRecentClipboardItems([historyItem({ id: "offline", summary: "Offline copy", syncStatus: "unsynced" })], 1)[0]?.syncStatus,
  "unsynced",
);

assert.deepEqual(
  stripCreatedAt(getRecentClipboardItems([
    historyItem({ id: "image-clean", summary: "\u56fe\u7247 2 KB", contentType: "image" }),
  ])),
  [{ id: "image-clean", text: "\u56fe\u7247 2 KB", contentType: "image", sourceDevice: "Device", syncStatus: "synced" }],
);

assert.deepEqual(
  stripCreatedAt(getRecentClipboardItems([
    historyItem({ id: "file-1", summary: "setup.exe 554 B", contentType: "fileList" }),
    historyItem({ id: "files-2", summary: "2 \u4e2a\u6587\u4ef6 3 MB", contentType: "fileList" }),
  ], 2)),
  [
    { id: "file-1", text: "setup.exe 554 B", contentType: "fileList", sourceDevice: "Device", syncStatus: "synced" },
    { id: "files-2", text: "2 \u4e2a\u6587\u4ef6 3 MB", contentType: "fileList", sourceDevice: "Device", syncStatus: "synced" },
  ],
);

assert.deepEqual(getRecentClipboardItems([], 3), []);
assert.deepEqual(getRecentClipboardItems([historyItem({ id: "empty", summary: " " })], 3), []);

assert.equal(getFloatingClipboardItems(Array.from({ length: 11 }, (_, index) => systemItem(index + 1)), []).length, 10);
assert.equal(
  getFloatingClipboardItems(
    Array.from({ length: 51 }, (_, index) => systemItem(index + 1)),
    [],
    FLOATING_CLIPBOARD_HISTORY_LIMIT,
  ).length,
  50,
);

assert.deepEqual(
  stripCreatedAt(getFloatingClipboardItems(
    [
      { id: "system-1", text: "WinV one", contentType: "text", syncStatus: "unsynced" },
      { id: "system-2", text: "WinV two", contentType: "text", syncStatus: "unsynced" },
      { id: "system-3", text: "WinV three", contentType: "text", syncStatus: "unsynced" },
      { id: "system-4", text: "WinV four", contentType: "text", syncStatus: "unsynced" },
      { id: "system-5", text: "WinV five", contentType: "text", syncStatus: "unsynced" },
      { id: "system-6", text: "WinV six", contentType: "text", syncStatus: "unsynced" },
    ],
    [historyItem({ id: "app-1", summary: "App history", sourceDevice: "Office-PC" })],
  )),
  [
    { id: "system-1", text: "WinV one", contentType: "text", syncStatus: "unsynced" },
    { id: "system-2", text: "WinV two", contentType: "text", syncStatus: "unsynced" },
    { id: "system-3", text: "WinV three", contentType: "text", syncStatus: "unsynced" },
    { id: "system-4", text: "WinV four", contentType: "text", syncStatus: "unsynced" },
    { id: "system-5", text: "WinV five", contentType: "text", syncStatus: "unsynced" },
    { id: "system-6", text: "WinV six", contentType: "text", syncStatus: "unsynced" },
    { id: "app-1", text: "App history", contentType: "text", sourceDevice: "Office-PC", syncStatus: "synced" },
  ],
);

const linkItem = getRecentClipboardItems([
  historyItem({ id: "link", content: "https://ping123.app/zh/", summary: "https://ping123.app/zh/" }),
])[0];
const imageItem = getRecentClipboardItems([
  historyItem({ id: "image", summary: "图片 55 KB", contentType: "image" }),
])[0];
const fileItem = getRecentClipboardItems([
  historyItem({ id: "file", summary: "C:\\Users\\SuZe\\Desktop\\setup.exe", contentType: "fileList" }),
])[0];
const videoItem = getRecentClipboardItems([
  historyItem({ id: "video", summary: "ca693499c52f61d3e1cdb505140927af.mp4 907.1 KB", contentType: "fileList" }),
])[0];
const apiKeyItem = getRecentClipboardItems([
  historyItem({ id: "api-key", content: "sk-copyshare-1234567890abcdef", summary: "sk-copyshare-1234567890abcdef" }),
])[0];
const textItem = getRecentClipboardItems([
  historyItem({ id: "text", content: "plain note", summary: "plain note" }),
])[0];

assert.equal(getClipboardDisplayType(linkItem).label, "链接");
assert.equal(getClipboardDisplayType(imageItem).label, "图片");
assert.equal(getClipboardDisplayType(fileItem).label, "文件");
assert.equal(getClipboardDisplayType(videoItem).label, "视频");
assert.equal(getClipboardDisplayType(apiKeyItem).label, "文本");
assert.equal(getClipboardDisplayType(textItem).label, "文本");

assert.deepEqual(
  filterClipboardItems([linkItem, imageItem, fileItem, videoItem, apiKeyItem, textItem], "链接", "").map((item) => item.id),
  ["link"],
);
assert.deepEqual(
  filterClipboardItems([linkItem, imageItem, fileItem, videoItem, apiKeyItem, textItem], "视频", "").map((item) => item.id),
  ["video"],
);
assert.deepEqual(
  filterClipboardItems([linkItem, imageItem, fileItem, videoItem, apiKeyItem, textItem], "文件", "").map((item) => item.id),
  ["file"],
);
assert.deepEqual(
  filterClipboardItems([linkItem, imageItem, fileItem, videoItem, apiKeyItem, textItem], "全部", "ping123").map((item) => item.id),
  ["link"],
);

assert.deepEqual(
  stripCreatedAt(getFloatingClipboardItems([], [historyItem({ id: "app-1", summary: "App history", sourceDevice: "Office-PC" })])),
  [{ id: "app-1", text: "App history", contentType: "text", sourceDevice: "Office-PC", syncStatus: "synced" }],
);

assert.deepEqual(
  stripCreatedAt(getFloatingClipboardItems(
    [
      { id: "system-1", text: "System one", contentType: "text", syncStatus: "unsynced" },
      { id: "system-2", text: "System two", contentType: "text", syncStatus: "unsynced" },
    ],
    [
      historyItem({ id: "app-1", summary: "App one", sourceDevice: "Office-PC" }),
      historyItem({ id: "app-2", summary: "System two", sourceDevice: "Office-PC" }),
    ],
  )),
  [
    { id: "system-1", text: "System one", contentType: "text", syncStatus: "unsynced" },
    { id: "system-2", text: "System two", contentType: "text", syncStatus: "unsynced" },
    { id: "app-1", text: "App one", contentType: "text", sourceDevice: "Office-PC", syncStatus: "synced" },
  ],
);
