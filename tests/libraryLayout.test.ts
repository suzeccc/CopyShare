import assert from "node:assert/strict";
import test from "node:test";

import {
  LIBRARY_LAYOUT_STORAGE_KEY,
  readLibraryLayout,
  writeLibraryLayout,
} from "../src/lib/libraryLayout.ts";

function storage(initial: string | null = null) {
  let value = initial;
  return {
    getItem(key: string) {
      assert.equal(key, LIBRARY_LAYOUT_STORAGE_KEY);
      return value;
    },
    setItem(key: string, next: string) {
      assert.equal(key, LIBRARY_LAYOUT_STORAGE_KEY);
      value = next;
    },
    value: () => value,
  };
}

test("library layout defaults to grid", () => {
  assert.equal(readLibraryLayout(storage()), "grid");
});

test("library layout restores valid values and rejects invalid values", () => {
  assert.equal(readLibraryLayout(storage("list")), "list");
  assert.equal(readLibraryLayout(storage("invalid")), "grid");
});

test("library layout persists the selected value", () => {
  const target = storage();
  writeLibraryLayout("list", target);
  assert.equal(target.value(), "list");
});
