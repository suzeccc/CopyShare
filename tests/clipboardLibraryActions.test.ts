import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

test("recent and modal clipboard cards expose favorite and pin actions", () => {
  const page = readFileSync("src/pages/Clipboard.vue", "utf8");
  assert.match(page, /useLibraryStore/);
  assert.equal((page.match(/data-history-favorite/g) ?? []).length, 2);
  assert.equal((page.match(/data-history-pin/g) ?? []).length, 2);
  assert.match(page, /libraryStore\.isHistoryItemSaved\(item\.id, item\.contentHash\)/);
  assert.match(page, /libraryStore\.isHistoryItemPinned\(item\.id, item\.contentHash\)/);
  assert.match(page, /isHistoryActionBusy\(item\)/);
  assert.equal((page.match(/@click\.stop="toggleHistoryFavorite\(item\)"/g) ?? []).length, 2);
  assert.equal((page.match(/@click\.stop="toggleHistoryPin\(item\)"/g) ?? []).length, 2);
  assert.match(page, /libraryStore\.collectHistoryItem\(item\.id, false\)/);
  assert.match(page, /libraryStore\.collectHistoryItem\(item\.id, true\)/);
});
