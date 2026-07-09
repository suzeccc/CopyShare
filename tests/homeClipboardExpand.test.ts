import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const home = readFileSync("src/pages/Clipboard.vue", "utf8");
const expandButtonClasses = [...home.matchAll(/data-clipboard-expand-button[\s\S]*?class="([^"]*)"/g)].map(
  (match) => match[1] ?? "",
);

assert.match(home, /expandedClipboardItemIds/);
assert.match(home, /isClipboardItemExpandable/);
assert.match(home, /toggleClipboardItemExpanded/);
assert.match(home, /data-clipboard-expand-button/);
assert.doesNotMatch(home, /data-clipboard-collapse-fade/);
assert.doesNotMatch(home, /from-\[\#303030\]\s+to-transparent/);
assert.doesNotMatch(home, /opacity-70/);
assert.doesNotMatch(home, /opacity-60/);
assert.match(home, /展开/);
assert.match(home, /收起/);
assert.match(home, /line-clamp-2/);
assert.match(home, /isClipboardItemExpanded\(item\)/);
assert.equal(expandButtonClasses.length, 2);
assert.equal(expandButtonClasses.every((className) => !className.includes("absolute bottom-2.5 right-3")), true);
