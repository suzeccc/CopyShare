import assert from "node:assert/strict";

import { getRefreshFeedbackView } from "../src/lib/refreshFeedback.ts";

const idle = getRefreshFeedbackView("idle");
assert.equal(idle.label, "刷新");
assert.equal(idle.disabled, false);

const refreshing = getRefreshFeedbackView("refreshing");
assert.equal(refreshing.label, "刷新中");
assert.equal(refreshing.disabled, true);

const done = getRefreshFeedbackView("done");
assert.equal(done.label, "已刷新");
assert.equal(done.disabled, false);
