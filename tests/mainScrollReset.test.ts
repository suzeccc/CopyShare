import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");

assert.match(appShell, /useRoute/);
assert.match(appShell, /const route = useRoute\(\)/);
assert.match(appShell, /mainScrollRef/);
assert.match(appShell, /\(\)\s*=>\s*route\.fullPath/);
assert.match(appShell, /await nextTick\(\)/);
assert.match(appShell, /scrollTop\s*=\s*0/);
assert.match(appShell, /scrollLeft\s*=\s*0/);
assert.match(appShell, /ref="mainScrollRef"/);
assert.match(appShell, /data-main-scroll-container/);
assert.match(appShell, /overflow-auto/);
