import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const settings = readFileSync("src/pages/Settings.vue", "utf8");

assert.match(settings, /options: \{ keepSaving\?: boolean \} = \{\}/);
assert.match(settings, /toastStore\.success\("保存成功"\);/);
assert.match(settings, /await saveBasicSettings\(\{ autoStart \}\);/);
assert.match(settings, /await saveBasicSettings\(\{ autoSync \}\);/);
assert.doesNotMatch(settings, /saveHistorySetting|await saveBasicSettings\(\{ saveHistory \}/);
assert.match(settings, /async function saveSyncSetting\(/);
assert.match(settings, /async function saveNotificationSetting\(/);
assert.doesNotMatch(settings, /silent/);
