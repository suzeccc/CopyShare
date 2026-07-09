import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const settings = readFileSync("src/pages/Settings.vue", "utf8");

assert.match(settings, /options: \{[\s\S]*silent\?: boolean;[\s\S]*\}/);
assert.match(settings, /if \(!options\.silent\) \{[\s\S]*toastStore\.success\("保存成功"\);[\s\S]*\}/);
assert.match(settings, /await saveBasicSettings\(\{ autoStart \}, \{ silent: true \}\);/);
assert.match(settings, /await saveBasicSettings\(\{ autoSync \}, \{ silent: true \}\);/);
assert.match(settings, /await saveBasicSettings\(\{ saveHistory \}, \{ silent: true \}\);/);
assert.match(settings, /async function saveSyncSetting\([\s\S]*options: \{ silent\?: boolean \} = \{ silent: true \}/);
assert.match(settings, /async function saveNotificationSetting\([\s\S]*options: \{ silent\?: boolean \} = \{ silent: true \}/);
