import assert from "node:assert/strict";

import { getRefreshFeedbackView } from "../src/lib/refreshFeedback.ts";

const idle = getRefreshFeedbackView("idle");
assert.equal(idle.label, "刷新");
assert.equal(idle.disabled, false);
assert.equal(idle.iconClass.includes("animate-spin"), false);

const refreshing = getRefreshFeedbackView("refreshing");
assert.equal(refreshing.label, "刷新中");
assert.equal(refreshing.disabled, true);
assert.equal(refreshing.iconClass.includes("animate-spin"), true);
assert.equal(refreshing.buttonClass.includes("border-sky-300/60"), true);

const done = getRefreshFeedbackView("done");
assert.equal(done.label, "已刷新");
assert.equal(done.disabled, false);
assert.equal(done.buttonClass.includes("border-emerald-300/50"), true);
assert.equal(done.iconClass.includes("text-emerald-100"), true);
