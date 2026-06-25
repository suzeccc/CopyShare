import assert from "node:assert/strict";

import {
  getWindowModeTransition,
  getWindowTransitionOrigin,
} from "../src/lib/windowTransition.ts";

assert.deepEqual(getWindowModeTransition("main", "floating"), {
  exitPhase: "main-exit",
  enterPhase: "floating-enter",
});

assert.deepEqual(getWindowModeTransition("floating", "main"), {
  exitPhase: "floating-exit",
  enterPhase: "main-enter",
});

assert.equal(getWindowModeTransition("main", "main"), null);

const rect = { left: 100, top: 40, width: 900, height: 600 };

assert.equal(
  getWindowTransitionOrigin({ clientX: 820, clientY: 88 }, rect),
  "720px 48px",
);
assert.equal(
  getWindowTransitionOrigin({ clientX: 20, clientY: 900 }, rect),
  "0px 600px",
);
