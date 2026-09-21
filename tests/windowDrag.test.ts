import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  shouldStartWindowDrag,
  startWindowDragFromMouseEvent,
} from "../src/lib/windowDrag.ts";

const style = readFileSync("src/style.css", "utf8");

assert.match(
  style,
  /\.floating-window-surface \{[\s\S]*contain: paint;[\s\S]*backdrop-filter: none;[\s\S]*-webkit-backdrop-filter: none;/,
);
assert.match(
  style,
  /html\[data-window-mode="floating"\] \.app-window-shell \{ filter: none; \}[\s\S]*html\[data-window-mode="floating"\] \.floating-window-surface \{[\s\S]*transform: none;[\s\S]*will-change: auto;/,
);
assert.match(
  style,
  /html\[data-window-mode="floating"\] \.floating-clipboard-row:hover \{\s*transform: none;/,
);

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

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((nextResolve) => {
    resolve = nextResolve;
  });

  return { promise, resolve };
}

const pendingDrag = deferred();
let startCount = 0;
const dragEvent = {
  button: 0,
  target: target({ dragRegion: true }),
  preventDefaultCalls: 0,
  stopPropagationCalls: 0,
  preventDefault() {
    this.preventDefaultCalls += 1;
  },
  stopPropagation() {
    this.stopPropagationCalls += 1;
  },
};

assert.equal(
  startWindowDragFromMouseEvent(dragEvent, () => {
    startCount += 1;
    return pendingDrag.promise;
  }),
  true,
);
assert.equal(
  startWindowDragFromMouseEvent(dragEvent, () => {
    startCount += 1;
    return Promise.resolve();
  }),
  false,
);
assert.equal(startCount, 1);
assert.equal(dragEvent.preventDefaultCalls, 2);
assert.equal(dragEvent.stopPropagationCalls, 2);

pendingDrag.resolve();
await pendingDrag.promise;
await new Promise((resolve) => setTimeout(resolve, 0));

assert.equal(
  startWindowDragFromMouseEvent(dragEvent, () => {
    startCount += 1;
    return Promise.resolve();
  }),
  true,
);
assert.equal(startCount, 2);
