import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const css = readFileSync("src/style.css", "utf8");

assert.match(css, /body\s*\{[\s\S]*user-select:\s*none;/);
assert.match(css, /body\s*\{[\s\S]*-webkit-user-select:\s*none;/);
assert.match(css, /input,\s*\ntextarea,\s*\n\[contenteditable="true"\]/);
assert.match(css, /input,\s*\ntextarea,\s*\n\[contenteditable="true"\]\s*\{[\s\S]*user-select:\s*text;/);

const home = readFileSync("src/pages/Home.vue", "utf8");

assert.match(home, /data-home-recent-text/);
assert.match(home, /data-clipboard-history-text/);
assert.match(css, /\[data-home-recent-text\],\s*\n\[data-clipboard-history-text\]/);
assert.match(css, /\[data-home-recent-text\],\s*\n\[data-clipboard-history-text\]\s*\{[\s\S]*user-select:\s*text;/);
assert.match(css, /\[data-home-recent-text\],\s*\n\[data-clipboard-history-text\]\s*\{[\s\S]*-webkit-user-select:\s*text;/);
assert.match(css, /\[data-home-recent-text\],\s*\n\[data-clipboard-history-text\]\s*\{[\s\S]*cursor:\s*text;/);