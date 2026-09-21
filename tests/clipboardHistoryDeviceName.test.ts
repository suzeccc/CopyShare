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

const historyModalSource =
  clipboardPage.match(
    /<section\s+class="flex h-full max-h-full w-full max-w-4xl[\s\S]*?<\/section>/,
  )?.[0] ?? "";

assert.ok(historyModalSource, "clipboard history modal must keep a stable height");
assert.match(
  historyModalSource,
  /v-if="filteredAllClipboardItems\.length" class="min-h-0 flex-1 overflow-x-hidden overflow-y-auto p-5"/,
);
assert.match(
  historyModalSource,
  /v-else class="m-5 grid min-h-0 flex-1 place-items-center/,
);
