import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { getRecentClipboardItems } from "../src/lib/historyPreview.ts";
import type { HistoryItem } from "../src/types/history.ts";

const item: HistoryItem = {
  id: "history-1",
  direction: "remote",
  sourceDevice: "Office-PC",
  summary: "hello",
  content: "hello",
  contentType: "text",
  syncStatus: "synced",
  success: true,
  createdAt: "2026-06-24T00:00:00.000Z",
};

assert.equal(getRecentClipboardItems([item], 1)[0]?.sourceDevice, "Office-PC");

const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");

assert.match(clipboardPage, /data-clipboard-history-device/);
assert.match(clipboardPage, /item\.sourceDevice/);
