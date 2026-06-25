import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const titleBar = readFileSync("src/components/layout/TitleBar.vue", "utf8");
const style = readFileSync("src/style.css", "utf8");

assert.match(titleBar, /event: "switch-floating", pointer/);
assert.match(titleBar, /clientX: event\.clientX/);
assert.match(titleBar, /clientY: event\.clientY/);

assert.match(appShell, /windowTransitionOrigin/);
assert.match(appShell, /getWindowTransitionOrigin/);
assert.match(appShell, /@switch-floating="switchToFloatingMode"/);
assert.match(appShell, /--window-transition-origin/);

assert.match(style, /transform-origin: var\(--window-transition-origin, center\)/);
