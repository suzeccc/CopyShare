import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";
import { FLOATING_BALL_BOUNDS, FLOATING_STARTUP_OFFSET, FLOATING_WINDOW_BOUNDS, MAIN_WINDOW_BOUNDS, getFloatingBallDockPosition } from "../src/lib/windowMode.ts";

const bridge = readFileSync("src/lib/tauri.ts", "utf8");
const nativeWindow = readFileSync("src-tauri/src/window_position.rs", "utf8");
assert.match(nativeWindow, /pub fn set_floating_ball_shape[\s\S]*?window\.set_skip_taskbar\(enabled\)/);
assert.match(nativeWindow, /#\[cfg\(any\(target_os = "windows", target_os = "linux"\)\)\]\s*window\.set_skip_taskbar/);
const start = bridge.indexOf("type WindowGeometry =");
const source = bridge.slice(start, bridge.indexOf("export function onAppEvent", start));
const createSession = new Function("invoke", "getCurrentWindow", "LogicalSize", "PhysicalPosition", "FLOATING_BALL_BOUNDS", "FLOATING_STARTUP_OFFSET", "FLOATING_WINDOW_BOUNDS", "MAIN_WINDOW_BOUNDS", "TRANSPARENT_WINDOW_BACKGROUND", "currentMonitor", "availableMonitors", "moveFloatingWindowToCursor", "moveMainWindowToCenter", "getFloatingWindowTopRightPosition", "getFloatingBallDockPosition", "waitForWindowSize", "localStorage", ts.transpile(source.replace(/export /g, "")) + "; return { enterBallWindow, dockBallWindow, enterFloatingWindow, restoreMainWindow }; ");
class Size {
  width: number; height: number;
  constructor(width: number, height: number) { this.width = width; this.height = height; }
}
class Position {
  x: number; y: number;
  constructor(x: number, y: number) { this.x = x; this.y = y; }
}
let size = { width: 2240, height: 1440 };
let position = { x: 100, y: 100 };
let failResize = false;
let maximized = false;
const storedPosition = new Map<string, string>();
const shapeChanges: boolean[] = [];
const monitor = { position: { x: 0, y: 0 }, size: { width: 3840, height: 2160 }, workArea: { position: { x: 0, y: 0 }, size: { width: 3840, height: 2160 } }, scaleFactor: 2 };
const native = {
  innerSize: async () => size, outerSize: async () => size, outerPosition: async () => position, scaleFactor: async () => 2,
  isMaximized: async () => maximized, toggleMaximize: async () => { maximized = !maximized; },
  setBackgroundColor: async () => {}, setAlwaysOnTop: async () => {}, setMaxSize: async () => {},
  setMinSize: async () => {}, setResizable: async () => {}, setShadow: async () => {}, setFocus: async () => {},
  setSize: async (next: Size) => {
    size = { width: next.width * 2, height: next.height * 2 };
    if (failResize) { failResize = false; throw new Error("resize failed"); }
  },
  setPosition: async (next: Position) => { position = { x: next.x, y: next.y }; },
};
const session = () => createSession(async (command: string, args: { enabled: boolean }) => { assert.equal(command, "set_floating_ball_shape"); shapeChanges.push(args.enabled); }, () => native, Size, Position, FLOATING_BALL_BOUNDS, FLOATING_STARTUP_OFFSET, FLOATING_WINDOW_BOUNDS, MAIN_WINDOW_BOUNDS, "#00000000", async () => monitor, async () => [monitor], async () => { position = { x: 50, y: 50 }; }, async () => { position = { x: 100, y: 100 }; }, () => ({}), getFloatingBallDockPosition, async (bounds: Size) => {
  assert.deepEqual(size, { width: bounds.width * 2, height: bounds.height * 2 });
}, { getItem: (key: string) => storedPosition.get(key) ?? null, setItem: (key: string, value: string) => { storedPosition.set(key, value); } });
const api = session();
await api.restoreMainWindow();
await api.enterFloatingWindow();
size = { width: 760, height: 840 };
position = { x: -1500, y: 200 };
await api.restoreMainWindow();
assert.deepEqual(size, { width: 2240, height: 1480 });
size = { width: 2600, height: 1600 };
position = { x: 200, y: 100 };
for (let i = 0; i < 3; i++) {
  await api.enterFloatingWindow();
  assert.deepEqual(size, { width: 760, height: 840 });
  assert.deepEqual(position, { x: -1500, y: 200 });
  await api.restoreMainWindow();
  assert.deepEqual(size, { width: 2600, height: 1600 });
  assert.deepEqual(position, { x: 200, y: 100 });
}
await api.enterFloatingWindow();
failResize = true;
await assert.rejects(api.restoreMainWindow(), /resize failed/);
await api.enterFloatingWindow();
assert.deepEqual(size, { width: 760, height: 840 }, "rollback must restore the saved floating size");
const restarted = session();
await restarted.enterFloatingWindow();
assert.deepEqual(size, { width: 680, height: 784 }, "a new session must use default floating bounds");
assert.deepEqual(position, { x: 50, y: 50 });
await restarted.restoreMainWindow();
assert.deepEqual(size, { width: 2240, height: 1480 }, "a new session must use default main bounds");
await restarted.enterBallWindow("top-right");
assert.equal(shapeChanges.at(-1), true);
assert.deepEqual(size, { width: 128, height: 128 });
assert.deepEqual(position, { x: 3680, y: 384 });
position = { x: 180, y: 700 };
await restarted.dockBallWindow();
assert.deepEqual(position, { x: 32, y: 700 });
assert.deepEqual(JSON.parse(storedPosition.get("copyshare:floating-ball-position")!), position);
await restarted.restoreMainWindow();
assert.equal(shapeChanges.at(-1), false);
assert.deepEqual(size, { width: 2240, height: 1480 }, "ball restores the prior main bounds");
maximized = true;
size = { width: 3840, height: 2160 };
position = { x: -16, y: -16 };
await restarted.enterBallWindow();
assert.equal(maximized, false, "the ball leaves the maximized state before shrinking");
await restarted.restoreMainWindow();
assert.deepEqual(size, { width: 2240, height: 1480 }, "maximized screen size is not reused as normal bounds");
assert.deepEqual(position, { x: 100, y: 100 }, "maximized screen position is not reused as normal position");
assert.equal(maximized, true, "restoring the main panel reapplies maximization");
