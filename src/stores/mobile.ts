import { defineStore } from "pinia";

import {
  closeMobileSession,
  confirmMobileClipboardWrite,
  createMobileSession,
  getMobileSessionStatus,
} from "@/lib/tauri";
import type {
  MobileDeviceHistoryItem,
  MobileSessionPhase,
  MobileSessionView,
} from "@/types/mobile";

const MOBILE_DEVICE_HISTORY_KEY = "copyshare:mobile-device-history";

function readMobileDeviceHistory(): MobileDeviceHistoryItem[] {
  if (typeof localStorage === "undefined") {
    return [];
  }

  try {
    const parsed = JSON.parse(localStorage.getItem(MOBILE_DEVICE_HISTORY_KEY) ?? "null") as Partial<MobileDeviceHistoryItem> | null;
    if (parsed?.id === "mobile" && parsed.name === "移动端" && typeof parsed.lastSeenAt === "string") {
      return [parsed as MobileDeviceHistoryItem];
    }
  } catch {
    // Ignore malformed local history and start clean.
  }
  return [];
}

function saveMobileDeviceHistory(history: MobileDeviceHistoryItem[]) {
  if (typeof localStorage === "undefined") {
    return;
  }

  try {
    localStorage.setItem(MOBILE_DEVICE_HISTORY_KEY, JSON.stringify(history[0] ?? null));
  } catch {
    // Local storage is best-effort; the current session remains usable.
  }
}

function isFinished(phase: MobileSessionPhase) {
  return phase === "expired" || phase === "closed";
}

export const useMobileStore = defineStore("mobile", {
  state: () => ({
    session: null as MobileSessionView | null,
    history: readMobileDeviceHistory(),
    recordedSessionId: null as string | null,
    loading: false,
    writeLoading: false,
    error: null as string | null,
  }),
  getters: {
    canWrite: (state) => state.session?.phase === "submitted",
    hasActiveSession: (state) => Boolean(state.session && !isFinished(state.session.phase)),
  },
  actions: {
    rememberMobileDevice(lastSeenAt = new Date().toISOString()) {
      this.history = [{ id: "mobile", name: "移动端", lastSeenAt }];
      saveMobileDeviceHistory(this.history);
    },
    async createSession() {
      this.loading = true;
      this.error = null;
      try {
        this.session = await createMobileSession();
        this.recordedSessionId = null;
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
        if (
          this.session.phase !== "waiting" &&
          !isFinished(this.session.phase) &&
          this.recordedSessionId !== this.session.id
        ) {
          this.rememberMobileDevice();
          this.recordedSessionId = this.session.id;
        }
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
