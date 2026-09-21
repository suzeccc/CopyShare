import { defineStore } from "pinia";

import {
  ACTIVITY_LOG_STORAGE_KEY,
  activityFromDevice,
  activityFromFileTransfer,
  activityFromHistory,
  activityFromSyncError,
  activityFromSyncStatus,
  readPersistedActivityLog,
  seedActivitiesFromHistory,
  upsertActivity,
} from "@/lib/activityLog";
import { onAppEvent } from "@/lib/tauri";
import type { ActivityLogEntry } from "@/types/activityLog";
import type { DeviceInfo } from "@/types/device";
import type { FileTransferTask } from "@/types/fileTransfer";
import type { HistoryItem } from "@/types/history";
import type { AppStatus, SyncState } from "@/types/status";

function browserStorage(): Storage | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export const useActivityLogStore = defineStore("activityLog", {
  state: () => ({
    items: [] as ActivityLogEntry[],
    initialized: false,
    seededFromHistory: false,
    lastSyncState: null as SyncState | null,
    unlisteners: [] as (() => void)[],
  }),
  actions: {
    initialize(historyItems: HistoryItem[], status: AppStatus) {
      if (this.initialized) return;
      const persisted = readPersistedActivityLog(browserStorage()?.getItem(ACTIVITY_LOG_STORAGE_KEY) ?? null);
      this.items = persisted?.items ?? [];
      this.seededFromHistory = persisted?.seededFromHistory ?? false;
      if (!this.seededFromHistory) {
        this.items = seedActivitiesFromHistory(historyItems);
        this.seededFromHistory = true;
      }
      this.lastSyncState = status.state;
      this.initialized = true;
      this.persist();
    },
    record(entry: ActivityLogEntry | null) {
      if (!entry) return;
      this.items = upsertActivity(this.items, entry);
      this.persist();
    },
    recordFileTransfer(task: FileTransferTask) {
      this.record(activityFromFileTransfer(task));
    },
    clear() {
      this.items = [];
      this.seededFromHistory = true;
      this.persist();
    },
    persist() {
      const storage = browserStorage();
      if (!storage) return;
      try {
        storage.setItem(ACTIVITY_LOG_STORAGE_KEY, JSON.stringify({
          version: 1,
          seededFromHistory: this.seededFromHistory,
          items: this.items,
        }));
      } catch {
        // The activity log is best-effort and must never interrupt clipboard sync.
      }
    },
    async subscribe() {
      if (this.unlisteners.length) return;
      this.unlisteners = await Promise.all([
        onAppEvent<HistoryItem>("clipboard-synced", (item) => this.record(activityFromHistory(item))),
        onAppEvent<DeviceInfo>("device-connected", (device) => this.record(activityFromDevice(device, "connected"))),
        onAppEvent<DeviceInfo>("device-disconnected", (device) => this.record(activityFromDevice(device, "disconnected"))),
        onAppEvent<DeviceInfo>("device-rejected", (device) => this.record(activityFromDevice(device, "rejected"))),
        onAppEvent<FileTransferTask>("file-transfer-offer", (task) => this.recordFileTransfer(task)),
        onAppEvent<FileTransferTask>("file-transfer-updated", (task) => this.recordFileTransfer(task)),
        onAppEvent<FileTransferTask>("file-transfer-completed", (task) => this.recordFileTransfer(task)),
        onAppEvent<FileTransferTask>("file-transfer-failed", (task) => this.recordFileTransfer(task)),
        onAppEvent<AppStatus>("sync-status-changed", (status) => {
          if (status.state !== this.lastSyncState) {
            this.lastSyncState = status.state;
            this.record(activityFromSyncStatus(status));
          }
        }),
        onAppEvent<string>("sync-error", (message) => this.record(activityFromSyncError(message))),
      ]);
    },
    dispose() {
      for (const unlisten of this.unlisteners) unlisten();
      this.unlisteners = [];
    },
  },
});
