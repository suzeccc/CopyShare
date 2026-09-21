import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";
import { FLOATING_WINDOW_BOUNDS, getFloatingWindowTopRightPosition } from "../src/lib/windowMode.ts";

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
const showWindowSource = commandsRs.match(/pub async fn show_main_window\([\s\S]*?\n\}/)?.[0];
assert.ok(showWindowSource);
assert.doesNotMatch(showWindowSource, /window\.center\(/, "showing the startup window must preserve its chosen top-right position");

// Run the real positioning branch with native window calls replaced.
const enterSource = tauriApi.match(/export async function enterFloatingWindow\([^)]*\): Promise<void> \{[\s\S]*?\n\}/)?.[0];
assert.ok(enterSource);
const createEnter = new Function("invoke", "getCurrentWindow", "currentMonitor", "LogicalSize", "PhysicalPosition", "FLOATING_WINDOW_BOUNDS", "TRANSPARENT_WINDOW_BACKGROUND", "getFloatingWindowTopRightPosition", "moveFloatingWindowToCursor", "console", "waitForWindowSize", "let activeWindowMode = null; const savedWindowGeometry = {}; const rememberWindowGeometry = async () => {}; " + ts.transpile(enterSource.replace("export ", "")) + "; return enterFloatingWindow;");
for (const scenario of [
  { position: "top-right", monitor: { position: { x: -2560, y: 0 }, size: { width: 2560, height: 1440 }, workArea: { position: { x: -2560, y: 40 }, size: { width: 2560, height: 1400 } }, scaleFactor: 2 }, expected: { x: -872, y: 424 }, cursorCalls: 0 },
  { position: "top-right", monitor: { position: { x: 1920, y: 0 }, size: { width: 1920, height: 1080 }, scaleFactor: 1 }, expected: { x: 3404, y: 192 }, cursorCalls: 0 },
  { position: "top-right", monitor: null, expected: null, cursorCalls: 0 },
  { position: undefined, monitor: null, expected: null, cursorCalls: 1 },
]) {
  let coordinates: { x: number; y: number } | null = null;
  let cursorCalls = 0;
  let focused = false;
  const native = {
    setBackgroundColor: async () => {}, setAlwaysOnTop: async () => {},
    setResizable: async (value: boolean) => { assert.equal(value, true); },
    setMinSize: async (size: { x: number; y: number }) => { assert.deepEqual({ width: size.x, height: size.y }, { width: 300, height: 320 }); },
    setMaxSize: async (size: unknown) => { assert.equal(size, null); }, setSize: async () => {}, setShadow: async () => {},
    setPosition: async (p: { x: number; y: number }) => { coordinates = { x: p.x, y: p.y }; },
    setFocus: async () => { focused = true; },
  };
  class Coordinates {
    x: number;
    y: number;
    constructor(x: number, y: number) { this.x = x; this.y = y; }
  }
  const enter = createEnter(async () => {}, () => native, async () => scenario.monitor, Coordinates, Coordinates, FLOATING_WINDOW_BOUNDS, "#00000000", getFloatingWindowTopRightPosition, async () => { cursorCalls++; }, console, async (size: unknown) => { assert.equal(size, FLOATING_WINDOW_BOUNDS); });
  await enter(scenario.position);
  assert.deepEqual(coordinates, scenario.expected);
  assert.equal(cursorCalls, scenario.cursorCalls);
  assert.equal(focused, true);
}
