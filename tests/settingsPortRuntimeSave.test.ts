import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const settings = readFileSync("src/pages/Settings.vue", "utf8");

assert.match(settings, /import \{ useStatusStore \} from "@\/stores\/status";/);
assert.match(settings, /const statusStore = useStatusStore\(\);/);
assert.match(settings, /async function savePort\(\)/);
assert.match(settings, /if \(configStore\.saving \|\| basicSettingsSaving\.value\) return;/);
assert.match(settings, /const wasRunning = statusStore\.status\.running;/);
assert.match(settings, /if \(wasRunning\) \{[\s\S]*await statusStore\.stop\(\);[\s\S]*\}/);
assert.match(settings, /await saveBasicSettings\(\{ port \}, \{ keepSaving: true \}\);/);
assert.match(settings, /if \(wasRunning && !configStore\.error\) \{[\s\S]*await statusStore\.start\(\);[\s\S]*\}/);
assert.match(settings, /finally \{[\s\S]*basicSettingsSaving\.value = false;[\s\S]*\}/);
assert.match(settings, /@change="savePort"/);
assert.match(settings, /@blur="savePort"/);
