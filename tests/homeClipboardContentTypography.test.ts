import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");

assert.match(
  clipboardPage,
  /data-clipboard-history-text[\s\S]*text-\[13px\][\s\S]*font-medium[\s\S]*leading-\[19px\]/,
);
assert.doesNotMatch(clipboardPage, /data-clipboard-history-text[\s\S]*text-sm font-semibold leading-5/);
