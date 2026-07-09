import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const deviceClasses = [...clipboardPage.matchAll(/data-clipboard-history-device[\s\S]*?class="([^"]*)"/g)].map(
  (match) => match[1] ?? "",
);
const actionBlocks = [...clipboardPage.matchAll(/<div data-clipboard-history-actions[\s\S]*?<\/div>/g)].map(
  (match) => match[0],
);

assert.match(clipboardPage, /data-clipboard-history-device/);
assert.match(clipboardPage, /data-clipboard-card-footer/);
assert.match(clipboardPage, /data-clipboard-card-footer[\s\S]*data-clipboard-card-time[\s\S]*data-clipboard-history-device/);
assert.match(clipboardPage, /data-clipboard-history-device[\s\S]*text-\[color:var\(--clipboard-card-footer-text\)\]/);
assert.equal(deviceClasses.length, 2);
assert.equal(deviceClasses.every((className) => !className.includes("absolute bottom-2.5 right-3")), true);
assert.equal(deviceClasses.every((className) => !className.includes("bg-white/[0.07]")), true);
assert.match(clipboardPage, /data-clipboard-history-actions[\s\S]*CopyTextButton/);
assert.equal(actionBlocks.length, 2);
assert.equal(actionBlocks.every((block) => block.includes("CopyTextButton")), true);
assert.equal(actionBlocks.every((block) => !block.includes("data-clipboard-history-sync-status")), true);
