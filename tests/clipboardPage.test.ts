import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const clipboardPagePath = "src/pages/Clipboard.vue";
const router = readFileSync("src/router/index.ts", "utf8");
const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");
const home = readFileSync("src/pages/Home.vue", "utf8");

assert.equal(existsSync(clipboardPagePath), true);

const clipboardPage = readFileSync(clipboardPagePath, "utf8");

assert.match(router, /Clipboard/);
assert.match(router, /path:\s*"\/clipboard"/);
assert.match(router, /name:\s*"clipboard"/);
assert.match(sidebar, /剪切板/);
assert.match(sidebar, /path:\s*"\/clipboard"/);
assert.match(clipboardPage, /data-clipboard-page/);
assert.match(clipboardPage, /最近同步内容/);
assert.match(clipboardPage, /data-clipboard-search-input/);
assert.match(clipboardPage, /data-clipboard-category-button/);
assert.match(clipboardPage, /data-clipboard-history-row/);
assert.match(clipboardPage, /bg-\[color:var\(--clipboard-card-bg\)\]/);
assert.match(clipboardPage, /border-\[color:var\(--clipboard-card-line\)\]/);
assert.match(clipboardPage, /text-\[color:var\(--clipboard-card-text\)\]/);
assert.doesNotMatch(clipboardPage, /bg-\[#303030\]/);
assert.doesNotMatch(clipboardPage, /border-\[#3A3A3A\]/);
assert.doesNotMatch(clipboardPage, /hover:bg-\[#343434\]/);
assert.match(clipboardPage, /data-clipboard-expand-button/);
assert.doesNotMatch(home, /data-home-recent-row/);
assert.doesNotMatch(home, /data-more-clipboard-button/);
assert.doesNotMatch(home, /data-clipboard-history-modal/);
