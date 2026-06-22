import assert from "node:assert/strict";

import { getWindowModeTransition } from "../src/lib/windowTransition.ts";

assert.deepEqual(getWindowModeTransition("main", "floating"), {
  exitPhase: "main-exit",
  enterPhase: "floating-enter",
});

assert.deepEqual(getWindowModeTransition("floating", "main"), {
  exitPhase: "floating-exit",
  enterPhase: "main-enter",
});

assert.equal(getWindowModeTransition("main", "main"), null);
