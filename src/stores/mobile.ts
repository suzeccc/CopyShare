import { defineStore } from "pinia";

import {
  closeMobileSession,
  confirmMobileClipboardWrite,
  createMobileSession,
  getMobileSessionStatus,
} from "@/lib/tauri";
import type { MobileSessionPhase, MobileSessionView } from "@/types/mobile";

function isFinished(phase: MobileSessionPhase) {
  return phase === "expired" || phase === "closed";
}

export const useMobileStore = defineStore("mobile", {
  state: () => ({
    session: null as MobileSessionView | null,
    loading: false,
    writeLoading: false,
    error: null as string | null,
  }),
  getters: {
    canWrite: (state) => state.session?.phase === "submitted",
    hasActiveSession: (state) => Boolean(state.session && !isFinished(state.session.phase)),
  },
  actions: {
    async createSession() {
      this.loading = true;
      this.error = null;
      try {
        this.session = await createMobileSession();
        return this.session;
      } catch (error) {
        this.error = String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async refreshSession() {
      if (!this.session || isFinished(this.session.phase)) {
        return this.session;
      }

      try {
        this.session = await getMobileSessionStatus(this.session.id);
        this.error = null;
      } catch (error) {
        this.error = String(error);
      }
      return this.session;
    },
    async closeSession() {
      if (!this.session || isFinished(this.session.phase)) {
        return this.session;
      }

      this.loading = true;
      this.error = null;
      try {
        this.session = await closeMobileSession(this.session.id);
        return this.session;
      } catch (error) {
        this.error = String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async writeReceivedContent() {
      if (!this.session || this.writeLoading) {
        return this.session;
      }

      this.writeLoading = true;
      this.error = null;
      try {
        this.session = await confirmMobileClipboardWrite(this.session.id);
        return this.session;
      } catch (error) {
        this.error = String(error);
        throw error;
      } finally {
        this.writeLoading = false;
      }
    },
  },
});
