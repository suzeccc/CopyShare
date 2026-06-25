import assert from "node:assert/strict";

import {
  FLOATING_WINDOW_BOUNDS,
  FLOATING_WINDOW_MARGIN,
  MAIN_WINDOW_BACKGROUND,
  getFloatingWindowTopRightPosition,
} from "../src/lib/windowMode.ts";

assert.equal(FLOATING_WINDOW_BOUNDS.width, 340);
assert.equal(FLOATING_WINDOW_BOUNDS.height, 320);
assert.equal(FLOATING_WINDOW_MARGIN, 16);
assert.equal(MAIN_WINDOW_BACKGROUND, "#10203a");

assert.deepEqual(
  getFloatingWindowTopRightPosition({
    position: { x: 0, y: 0 },
    size: { width: 1920, height: 1080 },
    scaleFactor: 1,
  }),
  { x: 1564, y: 16 },
);

assert.deepEqual(
  getFloatingWindowTopRightPosition({
    position: { x: 1920, y: 0 },
    size: { width: 2560, height: 1440 },
    scaleFactor: 2,
  }),
  { x: 3768, y: 32 },
);
