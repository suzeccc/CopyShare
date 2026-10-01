import { defineStore } from "pinia";

import {
  collectHistoryItem as collectHistoryItemApi,
  copyLibraryItem,
  createTextSnippet,
  getLibrary,
  getLibraryItemContent,
  onAppEvent,
  removeLibraryItem,
  reorderPinnedLibraryItems,
  setLibraryItemPinned,
  searchLibraryContent,
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
    searchQuery: "",
    matchingContentIds: new Set<string>(),
    searchRequestId: 0,
    searching: false,
    searchError: null as string | null,
    busyItemIds: new Set<string>(),
    unlisten: null as null | (() => void),
    subscriptionUsers: 0,
    subscriptionPending: null as Promise<void> | null,
  }),
  getters: {
    filteredItems(state): LibraryItem[] {
      const query = state.query.trim().toLocaleLowerCase();
      return state.items.filter((item) => {
        if (state.activeView === "snippets" && item.role !== "snippet") return false;
        if (state.activeView === "all" && item.role !== "saved") return false;
        if (
          state.contentTypeFilter !== "all"
          && item.contentType !== state.contentTypeFilter
        ) return false;
        if (!state.selectedTags.every((tag) => item.tags.includes(tag))) return false;
        if (!query) return true;
        return [item.title, item.content, item.summary, item.note, ...item.tags]
          .join("\n")
          .toLocaleLowerCase()
          .includes(query)
          || (state.searchQuery === query && state.matchingContentIds.has(item.id));
      });
    },
    availableTags(state): string[] {
      return [...new Set(state.items.flatMap((item) => item.tags))]
        .sort((left, right) => left.localeCompare(right, "zh-CN"));
    },
    savedItemsByHistoryId(state): ReadonlyMap<string, LibraryItem> {
      const savedItems = new Map<string, LibraryItem>();
      for (const item of state.items) {
        if (item.role === "saved" && item.sourceHistoryId !== null) {
          savedItems.set(item.sourceHistoryId, item);
        }
      }
      return savedItems;
    },
  },
  actions: {
    applySnapshot(snapshot: LibrarySnapshot) {
      this.items = snapshot.items;
      this.warning = snapshot.warning;
      this.loaded = true;
      this.searchRequestId += 1;
      this.searchQuery = "";
      this.matchingContentIds = new Set();
      this.searching = false;
    },
    async searchContent(query: string) {
      const normalized = query.trim().toLocaleLowerCase();
      const requestId = ++this.searchRequestId;
      this.searchError = null;
      if (!normalized) {
        this.searchQuery = "";
        this.matchingContentIds = new Set();
        this.searching = false;
        return;
      }
      this.searching = true;
      try {
        const ids = await searchLibraryContent(query);
        if (requestId !== this.searchRequestId) return;
        this.searchQuery = normalized;
        this.matchingContentIds = new Set(ids);
        this.searching = false;
      } catch (error) {
        if (requestId === this.searchRequestId) {
          this.searching = false;
          this.searchError = String(error);
        }
      }
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
      return this.savedItemsByHistoryId.get(historyId);
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
    async addToSnippets(item: LibraryItem) {
      await this.withItemAction(item.id, async () => createTextSnippet({
        title: item.title,
        content: await getLibraryItemContent(item.id),
        tags: item.tags,
        note: item.note,
      }));
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
