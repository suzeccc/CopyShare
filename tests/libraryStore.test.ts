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
    isPinned: true,
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

  assert.equal(store.activeView, "snippets");
  store.applySnapshot({ items: structuredClone(fixtures), warning: null });
  assert.equal(store.isHistoryItemSaved("history-1"), true);
  assert.equal(store.isHistoryItemPinned("history-1"), true);
  assert.equal(store.isHistoryItemSaved("history-duplicate", "hash-1"), false);
  assert.equal(store.isHistoryItemPinned("history-duplicate", "hash-1"), false);

  store.activeView = "all";
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

test("library store shares pending subscriptions and releases after async install", async () => {
  const listenResolvers: Array<(eventId: number) => void> = [];
  let listenCalls = 0;
  let unlistenCalls = 0;
  const previousWindow = (globalThis as any).window;
  (globalThis as any).window = {
    __TAURI_EVENT_PLUGIN_INTERNALS__: {
      unregisterListener: () => {},
    },
    __TAURI_INTERNALS__: {
      transformCallback: () => 1,
      invoke(command: string) {
        if (command === "plugin:event|listen") {
          listenCalls += 1;
          return new Promise<number>((resolve) => listenResolvers.push(resolve));
        }
        if (command === "plugin:event|unlisten") {
          unlistenCalls += 1;
          return Promise.resolve();
        }
        throw new Error(`unexpected command: ${command}`);
      },
    },
  };

  try {
    const { useLibraryStore } = await import("../src/stores/library.ts");
    setActivePinia(createPinia());
    const store = useLibraryStore();
    const first = store.subscribe();
    const second = store.subscribe();
    store.disposeSubscription();
    store.disposeSubscription();
    listenResolvers.forEach((resolve, index) => resolve(index + 1));
    await Promise.all([first, second]);
    await Promise.resolve();

    assert.equal(listenCalls, 1);
    assert.equal(unlistenCalls, 1);
    assert.equal(store.unlisten, null);
  } finally {
    (globalThis as any).window = previousWindow;
  }
});
