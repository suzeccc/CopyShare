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
assert.match(settings, /saveFeedbackState/);
assert.match(settings, /saveFeedbackView/);
assert.match(settings, /saveFeedbackIcon/);
assert.doesNotMatch(settings, /设置已保存。/);
