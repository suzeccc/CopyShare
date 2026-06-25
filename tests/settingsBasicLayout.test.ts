import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const settings = readFileSync("src/pages/Settings.vue", "utf8");

assert.match(settings, /data-basic-settings-row/);
assert.match(settings, /sm:grid-cols-\[minmax\(0,1fr\)_160px\]/);
assert.match(settings, /<label class="min-w-0">[\s\S]*v-model="draft\.deviceName"/);
assert.match(settings, /<label class="min-w-\[140px\]">[\s\S]*v-model\.number="draft\.port"/);
