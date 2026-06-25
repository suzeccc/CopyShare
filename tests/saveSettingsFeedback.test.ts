import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { getSaveFeedbackView } from "../src/lib/saveFeedback.ts";

const idle = getSaveFeedbackView("idle");
assert.equal(idle.label, "保存设置");
assert.equal(idle.disabled, false);
assert.equal(idle.iconClass, "");
assert.equal(idle.buttonClass, "");

const saving = getSaveFeedbackView("saving");
assert.equal(saving.label, "保存中");
assert.equal(saving.disabled, true);
assert.equal(saving.iconClass.includes("animate-spin"), true);

const saved = getSaveFeedbackView("saved");
assert.equal(saved.label, "已保存");
assert.equal(saved.disabled, false);
assert.equal(saved.buttonClass.includes("emerald"), true);

const failed = getSaveFeedbackView("error");
assert.equal(failed.label, "保存失败");
assert.equal(failed.disabled, false);
assert.equal(failed.buttonClass.includes("red"), true);

const settings = readFileSync("src/pages/Settings.vue", "utf8");
assert.match(settings, /saveFeedbackView/);
assert.doesNotMatch(settings, /saveMessage/);
assert.match(settings, /getSaveFeedbackView/);
assert.doesNotMatch(settings, /设置已保存。/);
assert.doesNotMatch(settings, /saveFeedbackState === "saved"[\s\S]*border-emerald-300/);
