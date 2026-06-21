import { defineStore } from "pinia";

import {
  getStatus,
  onAppEvent,
  startSync,
  stopSync,
  type AppEventName,
} from "@/lib/tauri";
import type { AppStatus } from "@/types/status";

const stoppedStatus: AppStatus = {
  running: false,
  deviceName: "Copy-Sharer",
  deviceId: "copy-sharer",
  localIp: null,
  port: 8765,
  connectedCount: 0,
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
    statusLabel: (state) => {
      if (state.status.state === "running") {
        return "同步中";
      }
      if (state.status.state === "error") {
        return "连接异常";
      }
      return "已停止";
    },
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
