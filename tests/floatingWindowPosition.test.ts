import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const tauriApi = readFileSync("src/lib/tauri.ts", "utf8");

assert.match(tauriApi, /PhysicalPosition/);
assert.match(tauriApi, /getFloatingWindowTopRightPosition/);
assert.match(tauriApi, /currentMonitor\(\)/);
assert.match(tauriApi, /setPosition\(new PhysicalPosition/);
assert.match(tauriApi, /moveFloatingWindowToTopRight/);
assert.match(tauriApi, /catch \(error\)/);
