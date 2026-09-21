import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const titleBar = readFileSync("src/components/layout/WindowTitleBar.vue", "utf8");
const shell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const tauri = readFileSync("src/lib/tauri.ts", "utf8");
const capability = readFileSync("src-tauri/capabilities/default.json", "utf8");

assert.match(
  titleBar,
  /class="h-full flex-1"[\s\S]*data-window-drag-region/,
);
assert.doesNotMatch(titleBar, /data-tauri-drag-region/);
assert.match(titleBar, /startWindowDragFromMouseEvent/);
assert.match(titleBar, /data-window-control[\s\S]*@dblclick\.stop/);
assert.match(titleBar, /aria-label="最小化"[\s\S]*@click="minimizeWindow\(\)"/);
assert.match(tauri, /export function minimizeWindow\(\): Promise<void> \{\s*return getCurrentWindow\(\)\.minimize\(\);/);
assert.match(capability, /"core:window:allow-minimize"/);
assert.match(shell, /<WindowTitleBar @close="handleCloseWindow"/);
assert.match(shell, /<FloatingPanel[\s\S]*@hide="switchToBallMode"/);
