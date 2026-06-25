import { defineStore } from "pinia";

import {
  createToast,
  limitToastQueue,
  TOAST_TIMEOUT_MS,
  type ToastKind,
  type ToastMessage,
} from "@/lib/toasts";

const toastTimers = new Map<string, number>();

export const useToastStore = defineStore("toasts", {
  state: () => ({
    items: [] as ToastMessage[],
  }),
  actions: {
    show(kind: ToastKind, message: string) {
      const toast = createToast(kind, message);
      this.items = limitToastQueue([...this.items, toast]);

      if (typeof window !== "undefined") {
        const timer = window.setTimeout(() => {
          this.remove(toast.id);
        }, TOAST_TIMEOUT_MS);
        toastTimers.set(toast.id, timer);
      }

      return toast.id;
    },
    success(message: string) {
      return this.show("success", message);
    },
    error(message: string) {
      return this.show("error", message);
    },
    info(message: string) {
      return this.show("info", message);
    },
    remove(id: string) {
      const timer = toastTimers.get(id);
      if (timer !== undefined && typeof window !== "undefined") {
        window.clearTimeout(timer);
      }
      toastTimers.delete(id);
      this.items = this.items.filter((toast) => toast.id !== id);
    },
    clear() {
      if (typeof window !== "undefined") {
        for (const timer of toastTimers.values()) {
          window.clearTimeout(timer);
        }
      }
      toastTimers.clear();
      this.items = [];
    },
  },
});
