import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const shell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const api = readFileSync("src/lib/tauri.ts", "utf8");
const polling = shell.slice(
  shell.indexOf("async function refreshSystemClipboardHistory("),
  shell.indexOf("\nwatch(", shell.indexOf("async function refreshSystemClipboardHistory(")),
);
const visibility = api.slice(
  api.indexOf("export async function isMainWindowVisible("),
  api.indexOf("export function onMainWindowFocusChanged("),
).replace("export ", "");
let visible = true;
let minimized = false;
let reads = 0;
let nextTimer = 0;
const timers = new Map<number, () => void>();
const context = vm.createContext({
  isFloating: { value: true },
  systemClipboardItems: { value: [{ content: "old" }] },
  getCurrentWindow: () => ({
    isVisible: async () => visible,
    isMinimized: async () => minimized,
  }),
  getClipboardHistory: async () => { reads += 1; return [{ content: "new" }]; },
  window: {
    clearInterval: (id: number) => timers.delete(id),
    setInterval: (callback: () => void, delay: number) => {
      assert.equal(delay, 1200);
      timers.set(++nextTimer, callback);
      return nextTimer;
    },
  },
});
vm.runInContext(ts.transpile(`
  let clipboardHistoryTimer;
  let clipboardHistoryPollingGeneration = 0;
  let clipboardHistoryPollingDisposed = false;
  let clipboardHistoryRefreshCount = 0;
  ${visibility}
  ${polling}
`), context);
const flush = () => new Promise<void>((resolve) => setImmediate(resolve));

await vm.runInContext("startClipboardHistoryPolling()", context);
await flush();
assert.equal(reads, 1);
assert.equal(timers.size, 1);

// Native visibility, not focus, determines whether polling may run.
visible = false;
timers.values().next().value!();
await flush();
assert.equal(reads, 1);
assert.equal(timers.size, 0);
assert.equal(context.systemClipboardItems.value[0].content, "new");
await vm.runInContext("startClipboardHistoryPolling()", context);
assert.equal(timers.size, 0);

visible = true;
await vm.runInContext("startClipboardHistoryPolling()", context);
await flush();
assert.equal(reads, 2);
assert.equal(timers.size, 1);
minimized = true;
timers.values().next().value!();
await flush();
assert.equal(reads, 2);
assert.equal(timers.size, 0);

// Explicit quick-panel requests may refresh while the main window is hidden.
await vm.runInContext("refreshSystemClipboardHistory()", context);
assert.equal(reads, 3);
minimized = false;
await Promise.all([
  vm.runInContext("startClipboardHistoryPolling()", context),
  vm.runInContext("startClipboardHistoryPolling()", context),
]);
await flush();
assert.equal(timers.size, 1);
context.isFloating.value = false;
await vm.runInContext("startClipboardHistoryPolling()", context);
assert.equal(timers.size, 0);

// Unmount/mode changes invalidate an in-flight visibility check.
context.isFloating.value = true;
const starting = vm.runInContext("startClipboardHistoryPolling()", context);
vm.runInContext("stopClipboardHistoryPolling()", context);
await starting;
assert.equal(timers.size, 0);
assert.match(shell, /onMainWindowFocusChanged\(\(\) => \{\s*void startClipboardHistoryPolling\(\)/);
assert.match(shell, /onBeforeUnmount\([\s\S]*?stopClipboardHistoryPolling\(\);\s*windowFocusUnlisten\?\.\(\)/);
vm.runInContext("clipboardHistoryPollingDisposed = true", context);
await vm.runInContext("startClipboardHistoryPolling()", context);
assert.equal(timers.size, 0);
