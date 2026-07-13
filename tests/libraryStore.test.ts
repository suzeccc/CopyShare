import assert from "node:assert/strict";
import test from "node:test";

import { createPinia, setActivePinia } from "pinia";

const fixtures = [
  {
    id: "saved-1",
    role: "saved",
    contentType: "text",
    title: "VPN URL",
    content: "https://vpn.example.test",
    summary: "https://vpn.example.test",
    assets: [],
    sourceHistoryId: "history-1",
    sourceContentHash: "hash-1",
    sourceDevice: "Office PC",
    contentHash: "library-hash-1",
    tags: ["Work"],
    note: "Internal",
    isPinned: false,
    pinOrder: null,
    createdAt: "2026-07-13T00:00:00Z",
    updatedAt: "2026-07-13T00:00:00Z",
  },
  {
    id: "snippet-1",
    role: "snippet",
    contentType: "text",
    title: "Reply",
    content: "Received, thanks.",
    summary: "Received, thanks.",
    assets: [],
    sourceHistoryId: null,
    sourceContentHash: null,
    sourceDevice: "",
    contentHash: "library-hash-2",
    tags: ["Reply"],
    note: "",
    isPinned: true,
    pinOrder: 0,
    createdAt: "2026-07-13T00:00:00Z",
    updatedAt: "2026-07-13T00:00:00Z",
  },
] as const;

test("library store filters items and tracks history membership", async () => {
  const { useLibraryStore } = await import("../src/stores/library.ts");
  setActivePinia(createPinia());
  const store = useLibraryStore();

  store.applySnapshot({ items: structuredClone(fixtures), warning: null });
  assert.equal(store.isHistoryItemSaved("history-1", "hash-1"), true);
  assert.equal(store.isHistoryItemPinned("history-1", "hash-1"), false);

  store.query = "vpn";
  assert.deepEqual(store.filteredItems.map((item) => item.id), ["saved-1"]);
  store.query = "";
  store.activeView = "snippets";
  assert.deepEqual(store.filteredItems.map((item) => item.id), ["snippet-1"]);
  store.activeView = "all";
  store.selectedTags = ["Work"];
  assert.deepEqual(store.filteredItems.map((item) => item.id), ["saved-1"]);

  store.beginItemAction("saved-1");
  assert.equal(store.isItemBusy("saved-1"), true);
  store.endItemAction("saved-1");
  assert.equal(store.isItemBusy("saved-1"), false);
});
