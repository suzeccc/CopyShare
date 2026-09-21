import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const shell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const main = readFileSync("src/main.ts", "utf8");
const store = readFileSync("src/stores/config.ts", "utf8");
const settings = readFileSync("src/pages/Settings.vue", "utf8");
const nativeConfig = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));

assert.equal(nativeConfig.app.windows[0].visible, false);
assert.match(store, /startupWindowMode: "floating"/);
assert.match(shell, /configStore.config.startupWindowMode === "ball" \? "floating"/);
assert.ok(main.indexOf("useConfigStore(pinia).config = initialConfig") < main.indexOf('.mount("#app")'));
assert.match(settings, /saveBasicSettings\(\{ startupWindowMode: option.value \}\)/);
assert.doesNotMatch(settings, /\{ value: "ball", label: "浮窗球" \}/);

// Execute the real startup routine with native calls replaced, including its failure fallback.
const body = shell.match(/async function initializeStartupWindow\(\) \{([\s\S]*?)\n\}\n\nasync function refreshSystemClipboardHistory/);
assert.ok(body);
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;
const initialize = new AsyncFunction("window", "windowMode", "enterFloatingWindow", "enterBallWindow", "restoreMainWindow", "showMainWindow", "console", "windowModeFailed", "props", "configStore", body[1]);

for (const scenario of [
  { mode: "floating", runtime: true, fail: false, failRestore: false, expected: ["floating", "show"] },
  { mode: "main", runtime: true, fail: false, failRestore: false, expected: ["main", "show"] },
  { mode: "ball", runtime: true, fail: false, failRestore: false, expected: ["ball", "show"] },
  { mode: "floating", runtime: false, fail: false, failRestore: false, expected: [] },
  { mode: "floating", runtime: true, fail: true, failRestore: false, expected: ["floating", "main", "show"] },
  { mode: "floating", runtime: true, fail: true, failRestore: true, expected: ["floating", "main", "show"] },
  { mode: "floating", runtime: true, fail: false, failRestore: false, firstRun: true, expected: ["main", "show"] },
  { mode: "ball", runtime: true, fail: false, failRestore: false, firstRun: true, expected: ["main", "show"] },
]) {
  const calls: string[] = [];
  const mode = { value: scenario.mode };
  const failed = { value: false };
  await initialize(
    scenario.runtime ? { __TAURI_INTERNALS__: {} } : {},
    mode,
    async (position: string) => { assert.equal(position, "top-right"); calls.push("floating"); if (scenario.fail) throw new Error("resize failed"); },
    async (position: string) => { assert.equal(position, "top-right"); calls.push("ball"); },
    async () => { calls.push("main"); if (scenario.failRestore) throw new Error("restore failed"); },
    async () => { calls.push("show"); },
    { error() {} },
    failed,
    { startupAnimationComplete: Promise.resolve() },
    { config: { onboardingCompleted: !("firstRun" in scenario && scenario.firstRun) } },
  );
  assert.deepEqual(calls, scenario.expected);
  assert.equal(mode.value, ("firstRun" in scenario && scenario.firstRun) || (scenario.fail && !scenario.failRestore) ? "main" : scenario.mode);
  assert.equal(failed.value, scenario.fail && scenario.failRestore);
}

// Native positioning must not begin until the loading animation has actually left.
let finishAnimation!: () => void;
const animationComplete = new Promise<void>((resolve) => { finishAnimation = resolve; });
const startupCalls: string[] = [];
const startupPending = initialize(
  { __TAURI_INTERNALS__: {} }, { value: "floating" },
  async () => { startupCalls.push("floating"); }, async () => {},
  async () => {},
  async () => { startupCalls.push("show"); }, { error() {} },
  { value: false }, { startupAnimationComplete: animationComplete },
  { config: { onboardingCompleted: true } },
);
await new Promise((resolve) => setImmediate(resolve));
assert.deepEqual(startupCalls, []);
finishAnimation();
await startupPending;
assert.deepEqual(startupCalls, ["floating", "show"]);
