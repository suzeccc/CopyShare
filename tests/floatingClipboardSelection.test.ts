import assert from "node:assert/strict";
import test from "node:test";

import { resolveFloatingClipboardSelection } from "../src/lib/floatingClipboardSelection.ts";
import type { ClipboardPreviewItem } from "../src/lib/historyPreview.ts";

function item(id: string, text: string): ClipboardPreviewItem {
  return {
    id,
    text,
    contentHash: `hash-${id}`,
    contentType: "text",
    syncStatus: "synced",
  };
}

test("floating clipboard selection rebinds to the refreshed item with the same id", () => {
  const selected = item("history-1", "before");
  const refreshed = item("history-1", "after");

  assert.equal(resolveFloatingClipboardSelection([refreshed], selected), refreshed);
});

test("floating clipboard selection closes only after the selected item disappears", () => {
  assert.equal(
    resolveFloatingClipboardSelection(
      [item("history-2", "other")],
      item("history-1", "selected"),
    ),
    null,
  );
});

test("floating clipboard selection remains empty when no item is selected", () => {
  assert.equal(resolveFloatingClipboardSelection([item("history-1", "text")], null), null);
});
