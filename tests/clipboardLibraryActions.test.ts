import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

test("recent and modal clipboard cards expose favorite and pin actions", () => {
  const page = readFileSync("src/pages/Clipboard.vue", "utf8");
  assert.match(page, /useLibraryStore/);
  assert.equal((page.match(/data-history-favorite/g) ?? []).length, 2);
  assert.equal((page.match(/data-history-pin/g) ?? []).length, 2);
  assert.match(page, /libraryStore\.isHistoryItemSaved\(item\.id\)/);
  assert.match(page, /libraryStore\.isHistoryItemPinned\(item\.id\)/);
  assert.doesNotMatch(page, /isHistoryItem(?:Saved|Pinned)\(item\.id, item\.contentHash\)/);
  assert.match(page, /isHistoryActionBusy\(item\)/);
  assert.equal((page.match(/@click\.stop="toggleHistoryFavorite\(item\)"/g) ?? []).length, 2);
  assert.equal((page.match(/@click\.stop="toggleHistoryPin\(item\)"/g) ?? []).length, 2);
  assert.equal((page.match(/clipboard-card-favorite-action/g) ?? []).length, 4);
  assert.match(
    page,
    /\.clipboard-card-favorite-action:hover:not\(:disabled\)[\s\S]*?background:\s*transparent/,
  );
  assert.match(
    page,
    /\.clipboard-card-favorite-action\.active[\s\S]*?background:\s*transparent[\s\S]*?color:\s*white/,
  );
  assert.equal((page.match(/clipboard-card-pin-action/g) ?? []).length, 4);
  assert.match(
    page,
    /\.clipboard-card-pin-action:hover:not\(:disabled\)[\s\S]*?background:\s*transparent/,
  );
  assert.match(
    page,
    /\.clipboard-card-pin-action\.active[\s\S]*?background:\s*transparent[\s\S]*?color:\s*white/,
  );
  assert.equal((page.match(/class="clipboard-card-copy-action"/g) ?? []).length, 2);
  assert.match(
    page,
    /\.clipboard-card-library-action:hover:not\(:disabled\)[\s\S]*?transform:\s*translateY\(-1px\)\s*scale\(1\.08\)/,
  );
  assert.match(
    page,
    /\.clipboard-card-library-action:active:not\(:disabled\)[\s\S]*?transform:\s*scale\(0\.95\)/,
  );
  assert.match(
    page,
    /\.clipboard-card-copy-action:hover:not\(:disabled\)[\s\S]*?transform:\s*translateY\(-1px\)\s*scale\(1\.08\)/,
  );
  assert.match(
    page,
    /\.clipboard-card-copy-action:active:not\(:disabled\)[\s\S]*?transform:\s*scale\(0\.95\)/,
  );
  assert.match(page, /libraryStore\.collectHistoryItem\(item\.id, false\)/);
  assert.match(page, /libraryStore\.collectHistoryItem\(item\.id, true\)/);
});

test("clipboard starts the shared library subscription before awaiting initial load", () => {
  const page = readFileSync("src/pages/Clipboard.vue", "utf8");
  const mounted = page.match(/onMounted\(async \(\) => \{[\s\S]*?\n}\);/)?.[0] ?? "";

  assert.match(
    mounted,
    /Promise\.all\(\[[\s\S]*?libraryStore\.load\(\)[\s\S]*?libraryStore\.subscribe\(\)[\s\S]*?\]\)/,
  );
});
