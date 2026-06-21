import { defineStore } from "pinia";

import { getConfig, onAppEvent, updateConfig } from "@/lib/tauri";
import type { AppConfig } from "@/types/config";

const defaultConfig: AppConfig = {
  deviceName: "Copy-Sharer",
  port: 8765,
  autoStart: false,
  autoSync: true,
  saveHistory: true,
  trustedDevices: [],
  syncText: true,
  syncImage: false,
  syncFiles: false,
};

export const useConfigStore = defineStore("config", {
  state: () => ({
    config: defaultConfig,
    saving: false,
    error: null as string | null,
  }),
  actions: {
    async refresh() {
      this.error = null;
      try {
        this.config = await getConfig();
      } catch (error) {
        this.error = String(error);
      }
    },
    async save(nextConfig: AppConfig) {
      this.saving = true;
      this.error = null;
      try {
        this.config = await updateConfig(nextConfig);
      } catch (error) {
        this.error = String(error);
      } finally {
        this.saving = false;
      }
    },
    async subscribe() {
      await onAppEvent<AppConfig>("config-updated", (config) => {
        this.config = config;
      });
    },
  },
});
