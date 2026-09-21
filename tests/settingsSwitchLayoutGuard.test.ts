import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const style = readFileSync("src/style.css", "utf8");
const switchControl = readFileSync("src/components/ui/Switch.vue", "utf8");

assert.doesNotMatch(appShell, /window-phase-/);
assert.doesNotMatch(appShell, /is-window-mode-transitioning/);
assert.match(style, /\.app-window-shell\[data-panel-transition="exit"\] \{\s*animation: panel-zoom-out/);
assert.doesNotMatch(style, /window-phase-/);
assert.match(switchControl, /@click\.stop/);
assert.match(switchControl, /@pointerdown\.stop/);
assert.match(switchControl, /<label\s+v-if="controlOnly"\s+class="[^"]*\brelative\b/);
assert.match(switchControl, /<label\s+v-else\s+class="[^"]*\brelative\b/);
assert.match(switchControl, /class="peer sr-only"/);
