import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const settings = readFileSync("src/pages/Settings.vue", "utf8");

assert.match(settings, /async function saveSyncImage\(syncImage: boolean\)/);
assert.match(settings, /async function saveSyncFiles\(syncFiles: boolean\)/);
assert.match(settings, /async function saveSyncSetting/);
assert.match(settings, /:model-value="draft\.syncImage"/);
assert.match(settings, /@update:model-value="saveSyncImage"/);
assert.match(settings, /:model-value="draft\.syncFiles"/);
assert.match(settings, /@update:model-value="saveSyncFiles"/);
assert.match(settings, /:disabled="configMutationSaving"/);
assert.doesNotMatch(settings, /<Switch v-model="draft\.syncImage"/);
assert.doesNotMatch(settings, /syncFiles:\s*false/);
assert.doesNotMatch(settings, /getSaveFeedbackView\(configStore\.saving \? "saving" : saveFeedbackState\.value\)/);
