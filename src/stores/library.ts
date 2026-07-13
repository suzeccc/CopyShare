import { defineStore } from "pinia";

import {
  collectHistoryItem as collectHistoryItemApi,
  convertLibraryItemToSnippet,
  copyLibraryItem,
  createTextSnippet,
  getLibrary,
  onAppEvent,
  removeLibraryItem,
  reorderPinnedLibraryItems,
  setLibraryItemPinned,
  updateLibraryItem,
} from "../lib/tauri.ts";
import type {
  CreateSnippetInput,
  LibraryContentFilter,
  LibraryItem,
  LibraryItemUpdate,
  LibrarySnapshot,
  LibraryView,
} from "@/types/library";

export const useLibraryStore = defineStore("library", {
  state: () => ({
    items: [] as LibraryItem[],
    warning: null as string | null,
    loading: false,
    loaded: false,
    query: "",
    activeView: "snippets" as LibraryView,
    contentTypeFilter: "all" as LibraryContentFilter,
    selectedTags: [] as string[],
    busyItemIds: new Set<string>(),
    unlisten: null as null | (() => void),
    subscriptionUsers: 0,
    subscriptionPending: null as Promise<void> | null,
  }),
  getters: {
    filteredItems(state): LibraryItem[] {
      const query = state.query.trim().toLocaleLowerCase();
      return state.items.filter((item) => {
        if (state.activeView === "pinned" && !item.isPinned) return false;
        if (state.activeView === "snippets" && item.role !== "snippet") return false;
        if (
          state.contentTypeFilter !== "all"
          && item.contentType !== state.contentTypeFilter
        ) return false;
        if (!state.selectedTags.every((tag) => item.tags.includes(tag))) return false;
        if (!query) return true;
        return [item.title, item.content, item.summary, item.note, ...item.tags]
          .join("\n")
          .toLocaleLowerCase()
          .includes(query);
      });
    },
    availableTags(state): string[] {
      return [...new Set(state.items.flatMap((item) => item.tags))]
        .sort((left, right) => left.localeCompare(right, "zh-CN"));
    },
  },
  actions: {
    applySnapshot(snapshot: LibrarySnapshot) {
      this.items = snapshot.items;
      this.warning = snapshot.warning;
      this.loaded = true;
    },
    beginItemAction(id: string) {
      this.busyItemIds = new Set(this.busyItemIds).add(id);
    },
    endItemAction(id: string) {
      const next = new Set(this.busyItemIds);
      next.delete(id);
      this.busyItemIds = next;
    },
    isItemBusy(id: string) {
      return this.busyItemIds.has(id);
    },
    savedItemForHistory(historyId: string) {
      return this.items.find((item) =>
        item.role === "saved"
        && item.sourceHistoryId === historyId);
    },
    isHistoryItemSaved(historyId: string) {
      return Boolean(this.savedItemForHistory(historyId));
    },
    isHistoryItemPinned(historyId: string) {
      return Boolean(this.savedItemForHistory(historyId)?.isPinned);
    },
    async load() {
      if (this.loading) return;
      this.loading = true;
      try {
        this.applySnapshot(await getLibrary());
      } finally {
        this.loading = false;
      }
    },
    async subscribe() {
      this.subscriptionUsers += 1;
      if (this.unlisten) return;
      if (!this.subscriptionPending) {
        this.subscriptionPending = onAppEvent<LibrarySnapshot>(
          "library-updated",
          (snapshot) => this.applySnapshot(snapshot),
        ).then((unlisten) => {
          if (this.subscriptionUsers === 0) unlisten();
          else this.unlisten = unlisten;
        }).finally(() => {
          this.subscriptionPending = null;
        });
      }
      await this.subscriptionPending;
    },
    disposeSubscription() {
      if (this.subscriptionUsers > 0) this.subscriptionUsers -= 1;
      if (this.subscriptionUsers > 0) return;
      this.unlisten?.();
      this.unlisten = null;
    },
    async withItemAction(
      id: string,
      action: () => Promise<LibrarySnapshot | void>,
    ) {
      if (this.isItemBusy(id)) return;
      this.beginItemAction(id);
      try {
        const snapshot = await action();
        if (snapshot) this.applySnapshot(snapshot);
      } finally {
        this.endItemAction(id);
      }
    },
    async collectHistoryItem(historyId: string, pin: boolean) {
      await this.withItemAction(historyId, async () =>
        collectHistoryItemApi(historyId, pin));
    },
    async createSnippet(input: CreateSnippetInput) {
      this.applySnapshot(await createTextSnippet(input));
    },
    async updateItem(id: string, update: LibraryItemUpdate) {
      await this.withItemAction(id, async () =>
        updateLibraryItem(id, update));
    },
    async convertToSnippet(id: string) {
      await this.withItemAction(id, async () =>
        convertLibraryItemToSnippet(id));
    },
    async setPinned(id: string, pinned: boolean) {
      await this.withItemAction(id, async () =>
        setLibraryItemPinned(id, pinned));
    },
    async reorderPinned(ids: string[]) {
      this.applySnapshot(await reorderPinnedLibraryItems(ids));
    },
    async removeItem(id: string) {
      await this.withItemAction(id, async () =>
        removeLibraryItem(id));
    },
    async copyItem(id: string) {
      await this.withItemAction(id, async () => copyLibraryItem(id));
    },
  },
});
