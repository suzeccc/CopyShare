import assert from "node:assert/strict";

import {
  FLOATING_WINDOW_BOUNDS,
  FLOATING_WINDOW_MARGIN,
  MAIN_WINDOW_BACKGROUND,
  getFloatingBallDockPosition,
  getFloatingWindowPointerPosition,
  getFloatingWindowTopRightPosition,
} from "../src/lib/windowMode.ts";

assert.equal(FLOATING_WINDOW_BOUNDS.width, 340);
assert.equal(FLOATING_WINDOW_BOUNDS.height, 392);
assert.equal(FLOATING_WINDOW_MARGIN, 16);
assert.equal(MAIN_WINDOW_BACKGROUND, "#0b100e");
const ballArea = { position: { x: -2560, y: 40 }, size: { width: 2560, height: 1400 }, scaleFactor: 2 };
assert.deepEqual(getFloatingBallDockPosition(ballArea, { x: -2500, y: -200 }), { x: -2528, y: 72 });
assert.deepEqual(getFloatingBallDockPosition(ballArea, { x: -50, y: 2000 }), { x: -160, y: 1280 });
assert.deepEqual(
  getFloatingWindowTopRightPosition({ position: { x: 0, y: 0 }, size: { width: 500, height: 500 }, scaleFactor: 1 }),
  { x: 64, y: 92 },
);

assert.deepEqual(
  getFloatingWindowTopRightPosition({
    position: { x: 0, y: 0 },
    size: { width: 1920, height: 1080 },
    scaleFactor: 1,
  }),
  { x: 1484, y: 192 },
);

assert.deepEqual(
  getFloatingWindowTopRightPosition({
    position: { x: 1920, y: 0 },
    size: { width: 2560, height: 1440 },
    scaleFactor: 2,
  }),
  { x: 3608, y: 384 },
);

assert.deepEqual(
  getFloatingWindowPointerPosition(
    {
      position: { x: 0, y: 0 },
      size: { width: 1920, height: 1080 },
      scaleFactor: 1,
    },
    { screenX: 960, screenY: 540 },
  ),
  { x: 790, y: 344 },
);

assert.deepEqual(
  getFloatingWindowPointerPosition(
    {
      position: { x: 0, y: 0 },
      size: { width: 1920, height: 1080 },
      scaleFactor: 1,
    },
    { screenX: 5, screenY: 5 },
  ),
  { x: 16, y: 16 },
);
