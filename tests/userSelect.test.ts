import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const css = readFileSync("src/style.css", "utf8");

assert.match(css, /body\s*\{[\s\S]*user-select:\s*none;/);
assert.match(css, /body\s*\{[\s\S]*-webkit-user-select:\s*none;/);
assert.match(css, /input,\s*\ntextarea,\s*\n\[contenteditable="true"\]/);
assert.match(css, /input,\s*\ntextarea,\s*\n\[contenteditable="true"\]\s*\{[\s\S]*user-select:\s*text;/);
