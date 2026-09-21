import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const settings = readFileSync("src/pages/Settings.vue", "utf8");
assert.match(settings, /async function saveBasicSettings/);
assert.doesNotMatch(settings, /saveFeedbackState/);
assert.doesNotMatch(settings, /saveFeedbackView/);
assert.doesNotMatch(settings, /saveFeedbackIcon/);
assert.doesNotMatch(settings, /getSaveFeedbackView/);
assert.doesNotMatch(settings, />\s*\{\{\s*saveFeedbackView\.label\s*\}\}\s*</);
assert.doesNotMatch(settings, /设置已保存。/);
