import { defineStore } from "pinia";

import {
  getStatus,
  onAppEvent,
  startSync,
  stopSync,
  type AppEventName,
} from "@/lib/tauri";
import { useDevicesStore } from "@/stores/devices";
import { getConnectionStatusLabel } from "@/lib/statusBadge";
import type { AppStatus } from "@/types/status";

const stoppedStatus: AppStatus = {
  running: false,
  deviceName: "CopyShare",
  deviceId: "copyshare",
  localIp: null,
  port: 8765,
  connectedCount: 0,
  latencyMs: null,
  lastSyncAt: null,
  state: "stopped",
  message: "等待启动同步",
};

export const useStatusStore = defineStore("status", {
  state: () => ({
    status: stoppedStatus,
    loading: false,
    error: null as string | null,
    unlisten: null as (() => void) | null,
  }),
  getters: {
    statusLabel: (state) => getConnectionStatusLabel(state.status),
  },
  actions: {
    async refresh() {
      this.error = null;
      try {
        this.status = await getStatus();
      } catch (error) {
        this.error = String(error);
      }
    },
    async start() {
      this.loading = true;
      this.error = null;
      try {
        this.status = await startSync();
      } catch (error) {
        this.error = String(error);
      } finally {
        this.loading = false;
      }
    },
    async stop() {
      this.loading = true;
      this.error = null;
      try {
        this.status = await stopSync();
        await useDevicesStore().refresh();
      } catch (error) {
        this.error = String(error);
      } finally {
        this.loading = false;
      }
    },
    async subscribe() {
      if (this.unlisten) {
        return;
      }
      const eventName: AppEventName = "sync-status-changed";
      const unlisten = await onAppEvent<AppStatus>(eventName, (payload) => {
        this.status = payload;
      });
      this.unlisten = unlisten;
    },
  },
});
