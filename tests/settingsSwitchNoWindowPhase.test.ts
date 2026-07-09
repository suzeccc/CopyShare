import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");

assert.doesNotMatch(appShell, /window-phase-/);
assert.doesNotMatch(appShell, /transitionPhase/);
assert.doesNotMatch(appShell, /getWindowModeTransition/);
assert.doesNotMatch(appShell, /WINDOW_MODE_ENTER_MS|WINDOW_MODE_EXIT_MS/);
assert.doesNotMatch(appShell, /windowTransitionOrigin/);
assert.match(appShell, /if \(windowMode\.value === nextMode\) \{/);
assert.match(appShell, /await resizeWindow\(pointer\);[\s\S]*windowMode\.value = nextMode;/);
