import { defineStore } from "pinia";

import { clearHistory, getHistory, onAppEvent } from "@/lib/tauri";
import type { HistoryItem } from "@/types/history";

export const useHistoryStore = defineStore("history", {
  state: () => ({
    items: [] as HistoryItem[],
    loading: false,
    error: null as string | null,
  }),
  actions: {
    async refresh() {
      this.error = null;
      try {
        this.items = await getHistory();
      } catch (error) {
        this.error = String(error);
      }
    },
    async clear() {
      this.loading = true;
      this.error = null;
      try {
        await clearHistory();
        this.items = [];
      } catch (error) {
        this.error = String(error);
      } finally {
        this.loading = false;
      }
    },
    async subscribe() {
      await onAppEvent<HistoryItem>("clipboard-synced", (item) => {
        this.items = [item, ...this.items].slice(0, 100);
      });
    },
  },
});
