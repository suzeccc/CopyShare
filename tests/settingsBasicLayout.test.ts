import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const settings = readFileSync("src/pages/Settings.vue", "utf8");

assert.match(settings, /data-basic-settings-row/);
assert.match(settings, /sm:grid-cols-\[minmax\(0,1fr\)_160px\]/);
assert.match(settings, /<label class="min-w-0">[\s\S]*v-model="draft\.deviceName"/);
assert.match(settings, /<label class="min-w-\[140px\]">[\s\S]*v-model\.number="draft\.port"/);
assert.match(settings, /data-close-action-options/);
assert.match(settings, /data-close-action-options[\s\S]*grid gap-2 rounded-2xl/);
assert.match(settings, /sm:grid-cols-3/);
assert.match(settings, /min-h-\[58px\]/);
assert.doesNotMatch(settings, /min-h-\[72px\]/);
assert.doesNotMatch(settings, /sm:grid-cols-\[120px_minmax\(0,1fr\)\]/);
