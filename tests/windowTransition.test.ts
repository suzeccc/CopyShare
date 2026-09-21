import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { WINDOW_MODE_ENTER_MS, WINDOW_MODE_EXIT_MS } from "../src/lib/windowTransition.ts";
import { getMainWindowCenteredPosition } from "../src/lib/windowMode.ts";

assert.equal(WINDOW_MODE_EXIT_MS, 160);
assert.equal(WINDOW_MODE_ENTER_MS, 250);

assert.deepEqual(
  getMainWindowCenteredPosition({
    position: { x: 0, y: 0 },
    size: { width: 1920, height: 1080 },
    scaleFactor: 1.25,
  }),
  { x: 260, y: 90 },
);

const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const style = readFileSync("src/style.css", "utf8");

assert.match(appShell, /const animatePanels = previousMode !== nextMode[\s\S]*previousMode !== "ball"[\s\S]*nextMode !== "ball";/);
assert.doesNotMatch(appShell.match(/const animatePanels = ([\s\S]*?);/)?.[1] ?? "", /prefers-reduced-motion/);
assert.match(appShell, /panelTransitionPhase\.value = "exit";[\s\S]*WINDOW_MODE_EXIT_MS[\s\S]*isResizingWindow\.value = true;[\s\S]*await resizeWindow\(pointer\);[\s\S]*windowMode\.value = nextMode/);
assert.match(appShell, /panelTransitionPhase\.value = animatePanels && !windowModeFailed\.value \? "enter" : null;[\s\S]*isResizingWindow\.value = false;[\s\S]*WINDOW_MODE_ENTER_MS/);
assert.match(appShell, /:data-panel-transition="panelTransitionPhase"/);
assert.match(appShell, /v-show="!isResizingWindow && !windowModeFailed"/);

assert.match(style, /\.app-window-shell\.is-mode-resizing \{[\s\S]*background-color: transparent !important;[\s\S]*transition: none;/);
assert.match(style, /html\[data-window-mode="main"\] \.app-window-shell \{\s*--panel-exit-scale: \.82;\s*--panel-enter-scale: \.84;/);
assert.match(style, /html\[data-window-mode="floating"\] \.app-window-shell \{\s*--panel-exit-scale: \.92;\s*--panel-enter-scale: \.88;/);
assert.match(style, /\.app-window-shell\[data-panel-transition\] \{\s*transform-origin: top right;/);
assert.match(style, /\.app-window-shell\[data-panel-transition="exit"\] \{[\s\S]*panel-zoom-out 160ms/);
assert.match(style, /\.app-window-shell\[data-panel-transition="enter"\] \{[\s\S]*panel-zoom-in 250ms/);
assert.match(style, /@keyframes panel-zoom-out \{[\s\S]*transform: scale\(var\(--panel-exit-scale\)\)/);
assert.match(style, /@keyframes panel-zoom-in \{[\s\S]*transform: scale\(var\(--panel-enter-scale\)\);[\s\S]*transform: scale\(1\)/);
assert.match(style, /@keyframes panel-zoom-out \{\s*65% \{ opacity: \.85; \}/);
assert.match(style, /@keyframes panel-zoom-in \{[\s\S]*35% \{ opacity: \.88; \}/);
assert.doesNotMatch(style.match(/@media \(prefers-reduced-motion: reduce\) \{([\s\S]*?)\n\}/)?.[1] ?? "", /\.app-window-shell/);
assert.doesNotMatch(style, /window-phase-|window-panel-enter|main-shell-enter-bridge|floating-shell-enter-bridge/);
