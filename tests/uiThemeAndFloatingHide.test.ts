import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const floatingPanel = readFileSync(
  "src/components/layout/FloatingPanel.vue",
  "utf8",
);
const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const style = readFileSync("src/style.css", "utf8");
const windowMode = readFileSync("src/lib/windowMode.ts", "utf8");

assert.match(floatingPanel, /event:\s*"hide"/);
assert.match(floatingPanel, /<Minus\b/);
assert.match(floatingPanel, /@click="emit\('hide'\)"/);
assert.match(appShell, /hideMainWindow/);
assert.match(appShell, /@hide="hideMainWindow"/);
assert.doesNotMatch(floatingPanel, /statusMessage/);
assert.doesNotMatch(appShell, /:status-message=/);

assert.match(style, /--main-bg:\s*#10203a;/);
assert.match(style, /--main-bg-deep:\s*#0b172c;/);
assert.match(windowMode, /MAIN_WINDOW_BACKGROUND\s*=\s*"#10203a"/);

assert.match(style, /button\[data-window-control\]\s*\{[\s\S]*transition:/);
assert.match(style, /button\[data-window-control\]:hover\s*\{[\s\S]*transform:\s*translateY\(-1px\)\s*scale\(1\.03\);/);
assert.match(style, /button\[data-window-control\]:active\s*\{[\s\S]*transform:\s*translateY\(0\)\s*scale\(0\.98\);/);
