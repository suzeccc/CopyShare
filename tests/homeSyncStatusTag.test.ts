import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const actionBlocks = [...clipboardPage.matchAll(/<div data-clipboard-history-actions[\s\S]*?<\/div>/g)].map(
  (match) => match[0],
);

assert.match(clipboardPage, /data-clipboard-history-sync-status/);
assert.match(clipboardPage, /syncStatusLabel/);
assert.match(clipboardPage, /syncStatusClass/);
assert.match(clipboardPage, /已同步/);
assert.match(clipboardPage, /未同步/);
assert.match(clipboardPage, /data-clipboard-history-row[\s\S]*min-h-\[86px\]/);
assert.equal(actionBlocks.length, 2);
for (const block of actionBlocks) {
  assert.match(block, /CopyTextButton/);
  assert.doesNotMatch(block, /data-clipboard-history-sync-status/);
}

const syncStatusClasses = [...clipboardPage.matchAll(/data-clipboard-history-sync-status[\s\S]*?class="([^"]*)"/g)].map(
  (match) => match[1] ?? "",
);
assert.equal(syncStatusClasses.length, 2);
assert.equal(syncStatusClasses.every((className) => className.includes("absolute bottom-2.5 right-3")), true);
