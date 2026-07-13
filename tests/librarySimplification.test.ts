import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

test("library keeps pin actions without a standalone pinned view", () => {
  const types = readFileSync("src/types/library.ts", "utf8");
  const store = readFileSync("src/stores/library.ts", "utf8");
  const page = readFileSync("src/pages/Library.vue", "utf8");
  const card = readFileSync("src/components/library/LibraryCard.vue", "utf8");

  assert.match(types, /LibraryView = "all" \| "snippets"/);
  assert.doesNotMatch(types, /LibraryView[^\n]*pinned/);
  assert.doesNotMatch(store, /activeView === "pinned"/);
  assert.doesNotMatch(page, /value: "pinned"/);
  assert.doesNotMatch(page, /data-library-view-pinned/);
  assert.match(card, /data-library-pin/);
  assert.match(card, /item\.isPinned/);
});

test("library storage refresh follows snapshots and ignores stale responses", () => {
  const page = readFileSync("src/pages/Library.vue", "utf8");

  assert.match(page, /watch\(/);
  assert.match(page, /storageRefreshId/);
  assert.match(page, /requestId !== storageRefreshId/);
  assert.match(page, /getLibraryStorageSize\(\)/);
});

test("snippet metadata and OCR sidebar use the compact labels", () => {
  const card = readFileSync("src/components/library/LibraryCard.vue", "utf8");
  const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");

  assert.match(card, /item\.role === "snippet" \? formatTime\(item\.createdAt\)/);
  assert.match(sidebar, /label: "图转文字", path: "\/ocr"/);
  assert.doesNotMatch(sidebar, /label: "图片转文字", path: "\/ocr"/);
});
