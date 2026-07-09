import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { getSaveFeedbackView } from "../src/lib/saveFeedback.ts";

const idle = getSaveFeedbackView("idle");
assert.equal(idle.label, "保存设置");
assert.equal(idle.disabled, false);

const saving = getSaveFeedbackView("saving");
assert.equal(saving.label, "保存中");
assert.equal(saving.disabled, true);

const saved = getSaveFeedbackView("saved");
assert.equal(saved.label, "已保存");
assert.equal(saved.disabled, false);

const failed = getSaveFeedbackView("error");
assert.equal(failed.label, "保存失败");
assert.equal(failed.disabled, false);

const settings = readFileSync("src/pages/Settings.vue", "utf8");
assert.match(settings, /async function saveBasicSettings/);
assert.doesNotMatch(settings, /saveFeedbackState/);
assert.doesNotMatch(settings, /saveFeedbackView/);
assert.doesNotMatch(settings, /saveFeedbackIcon/);
assert.doesNotMatch(settings, /getSaveFeedbackView/);
assert.doesNotMatch(settings, />\s*\{\{\s*saveFeedbackView\.label\s*\}\}\s*</);
assert.doesNotMatch(settings, /设置已保存。/);
