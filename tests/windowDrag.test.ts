import assert from "node:assert/strict";

import { shouldStartWindowDrag } from "../src/lib/windowDrag.ts";

type MockTarget = {
  closest: (selector: string) => Element | null;
};

function target(options: {
  dragRegion?: boolean;
  windowControl?: boolean;
  button?: boolean;
}): MockTarget {
  return {
    closest(selector: string) {
      if (options.button && selector.includes("button")) {
        return {} as Element;
      }

      if (options.windowControl && selector.includes("[data-window-control]")) {
        return {} as Element;
      }

      if (
        options.dragRegion &&
        (selector.includes("[data-tauri-drag-region]") ||
          selector.includes("[data-window-drag-region]"))
      ) {
        return {} as Element;
      }

      return null;
    },
  };
}

assert.equal(shouldStartWindowDrag({ button: 0, target: target({ dragRegion: true }) }), true);
assert.equal(shouldStartWindowDrag({ button: 0, target: target({}) }), false);
assert.equal(
  shouldStartWindowDrag({
    button: 0,
    target: target({ dragRegion: true, windowControl: true }),
  }),
  false,
);
assert.equal(
  shouldStartWindowDrag({ button: 0, target: target({ dragRegion: true, button: true }) }),
  false,
);
assert.equal(shouldStartWindowDrag({ button: 1, target: target({ dragRegion: true }) }), false);
