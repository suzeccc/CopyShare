import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const titleBar = readFileSync("src/components/layout/WindowTitleBar.vue", "utf8");

assert.match(
  titleBar,
  /class="h-full flex-1"[\s\S]*data-tauri-drag-region[\s\S]*data-window-drag-region/,
);
assert.match(titleBar, /data-window-control[\s\S]*@dblclick\.stop/);
