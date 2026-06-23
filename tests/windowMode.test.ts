import assert from "node:assert/strict";

import { FLOATING_WINDOW_BOUNDS, MAIN_WINDOW_BACKGROUND } from "../src/lib/windowMode.ts";

assert.equal(FLOATING_WINDOW_BOUNDS.width, 340);
assert.equal(FLOATING_WINDOW_BOUNDS.height, 320);
assert.equal(MAIN_WINDOW_BACKGROUND, "#10203a");
