import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const tauriApi = readFileSync("src/lib/tauri.ts", "utf8");
const commandsRs = readFileSync("src-tauri/src/commands.rs", "utf8");
const libRs = readFileSync("src-tauri/src/lib.rs", "utf8");

assert.match(tauriApi, /moveFloatingWindowToCursor/);
assert.match(tauriApi, /invoke<void>\("move_floating_window_to_cursor"\)/);
assert.match(tauriApi, /moveMainWindowToCenter/);
assert.match(tauriApi, /invoke<void>\("move_main_window_to_center"\)/);
assert.doesNotMatch(tauriApi, /moveFloatingWindowNearPointer/);
assert.doesNotMatch(tauriApi, /getFloatingWindowPointerPosition/);
assert.match(commandsRs, /move_floating_window_to_cursor/);
assert.match(commandsRs, /move_main_window_to_center/);
assert.match(libRs, /commands::move_floating_window_to_cursor/);
assert.match(libRs, /commands::move_main_window_to_center/);
assert.match(tauriApi, /catch \(error\)/);
