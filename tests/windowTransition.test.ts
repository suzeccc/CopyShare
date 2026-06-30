import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  getWindowModeTransition,
  getWindowTransitionOrigin,
  WINDOW_MODE_ENTER_MS,
  WINDOW_MODE_EXIT_MS,
} from "../src/lib/windowTransition.ts";
import { getMainWindowCenteredPosition } from "../src/lib/windowMode.ts";

assert.deepEqual(getWindowModeTransition("main", "floating"), {
  exitPhase: "main-exit",
  enterPhase: "floating-enter",
});

assert.deepEqual(getWindowModeTransition("floating", "main"), {
  exitPhase: "floating-exit",
  enterPhase: "main-enter",
});

assert.equal(getWindowModeTransition("main", "main"), null);

const rect = { left: 100, top: 40, width: 900, height: 600 };

assert.equal(
  getWindowTransitionOrigin({ clientX: 820, clientY: 88 }, rect),
  "720px 48px",
);
assert.equal(
  getWindowTransitionOrigin({ clientX: 20, clientY: 900 }, rect),
  "0px 600px",
);

assert.equal(WINDOW_MODE_EXIT_MS, 220);
assert.equal(WINDOW_MODE_ENTER_MS, 220);

const style = readFileSync("src/style.css", "utf8");
const tauriBridge = readFileSync("src/lib/tauri.ts", "utf8");
const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");

assert.deepEqual(
  getMainWindowCenteredPosition({
    position: { x: 0, y: 0 },
    size: { width: 1920, height: 1080 },
    scaleFactor: 1.25,
  }),
  { x: 260, y: 90 },
);

const restoreMainWindowSource =
  tauriBridge.match(/export async function restoreMainWindow\(\): Promise<void> \{[\s\S]*?\n\}/)?.[0] ?? "";

assert.match(
  restoreMainWindowSource,
  /await moveMainWindowToCenter\(window\);[\s\S]*await window\.setSize\(/,
);

assert.doesNotMatch(
  restoreMainWindowSource,
  /window\.center\(\)/,
);

assert.doesNotMatch(
  appShell,
  /<Transition[^>]*window-panel/,
);

assert.doesNotMatch(
  appShell,
  /<Transition[^>]*:css=/,
);

assert.match(appShell, /<FloatingPanel[\s\S]*v-if="isFloating"/);
assert.match(appShell, /<div v-else class="main-window-content/);

assert.match(
  tauriBridge,
  /restoreMainWindow\(\): Promise<void> \{[\s\S]*setBackgroundColor\(TRANSPARENT_WINDOW_BACKGROUND\);[\s\S]*setFocus\(\);[\s\S]*\}/,
);

assert.doesNotMatch(
  tauriBridge,
  /restoreMainWindow\(\): Promise<void> \{[\s\S]*setBackgroundColor\(MAIN_WINDOW_BACKGROUND\);[\s\S]*\}/,
);

assert.match(
  style,
  /\.window-phase-main-exit\.app-window-shell,\s*\.window-phase-main-enter\.app-window-shell \{[\s\S]*background-color: transparent !important;[\s\S]*border-color: transparent !important;[\s\S]*box-shadow: none;[\s\S]*\}/,
);

assert.match(
  style,
  /\.window-phase-floating-enter\.app-window-shell::before \{[\s\S]*background: var\(--floating-surface-bg\);[\s\S]*animation: floating-shell-enter-bridge 180ms cubic-bezier\(0\.16, 1, 0\.3, 1\) both;[\s\S]*\}/,
);

assert.match(
  style,
  /\.window-phase-main-enter\.app-window-shell::before \{[\s\S]*background: var\(--main-bg\);[\s\S]*animation: main-shell-enter-bridge 180ms cubic-bezier\(0\.16, 1, 0\.3, 1\) both;[\s\S]*\}/,
);

assert.match(
  style,
  /@media \(prefers-reduced-motion: reduce\) \{[\s\S]*\.app-window-shell::before,[\s\S]*animation: none !important;[\s\S]*\}/,
);

assert.match(
  style,
  /\.window-phase-main-exit \.main-window-content,\s*\.window-phase-main-enter \.main-window-content \{[\s\S]*background: var\(--main-bg\);[\s\S]*border-radius: 18px;[\s\S]*\}/,
);

assert.match(
  style,
  /\.window-phase-main-exit \.main-window-content \{[\s\S]*opacity: 0\.22;[\s\S]*filter: blur\(4px\) saturate\(0\.82\);[\s\S]*transform: translate\(26px, -16px\) scale\(0\.22\) rotate\(3deg\);[\s\S]*\}/,
);

assert.match(
  style,
  /@keyframes floating-window-enter \{[\s\S]*0% \{[\s\S]*opacity: 0\.22;[\s\S]*transform: scale\(0\.72\) translate\(28px, -18px\);[\s\S]*68% \{[\s\S]*transform: scale\(1\.045\) translate\(0, 0\);[\s\S]*100% \{[\s\S]*transform: scale\(1\) translateY\(0\);[\s\S]*\}/,
);

assert.match(
  style,
  /\.window-phase-floating-exit \.floating-window-surface \{[\s\S]*opacity: 0\.24;[\s\S]*filter: blur\(4px\) saturate\(0\.82\);[\s\S]*transform: translate\(18px, -14px\) scale\(0\.36\) rotate\(-2deg\);[\s\S]*\}/,
);

assert.match(
  style,
  /\.window-phase-main-enter \.main-window-content \{[\s\S]*transform-origin: top right;[\s\S]*animation: main-window-enter 220ms cubic-bezier\(0\.16, 1, 0\.3, 1\) both;[\s\S]*\}/,
);

assert.match(
  style,
  /\.window-phase-floating-enter \.floating-window-surface \{[\s\S]*animation: floating-window-enter 220ms cubic-bezier\(0\.16, 1, 0\.3, 1\) both;[\s\S]*\}/,
);

assert.match(
  style,
  /@keyframes main-window-enter \{[\s\S]*0% \{[\s\S]*opacity: 0\.2;[\s\S]*transform: scale\(0\.72\) translate\(34px, -22px\);[\s\S]*68% \{[\s\S]*transform: scale\(1\.018\) translate\(0, 0\);[\s\S]*100% \{[\s\S]*transform: scale\(1\) translateY\(0\);[\s\S]*\}/,
);
