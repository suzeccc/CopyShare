import assert from "node:assert/strict";

import { getConnectionBadgeView } from "../src/lib/statusBadge.ts";

const running = getConnectionBadgeView("running");
assert.equal(running.containerClass.includes("border-emerald-400/50"), true);
assert.equal(running.containerClass.includes("text-emerald-200"), true);
assert.equal(running.dotClass.includes("bg-emerald-300"), true);

const stopped = getConnectionBadgeView("stopped");
assert.equal(stopped.containerClass.includes("border-white/55"), true);
assert.equal(stopped.containerClass.includes("bg-white/[0.08]"), true);
assert.equal(stopped.containerClass.includes("text-white"), true);
assert.equal(stopped.dotClass.includes("bg-white"), true);

const error = getConnectionBadgeView("error");
assert.equal(error.containerClass.includes("border-red-400/50"), true);
assert.equal(error.dotClass.includes("bg-red-300"), true);
