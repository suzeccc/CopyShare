import { defineStore } from "pinia";

import { getConfig, onAppEvent, updateConfig } from "@/lib/tauri";
import type { AppConfig } from "@/types/config";

const defaultConfig: AppConfig = {
  configVersion: 6,
  deviceName: "CopyShare",
  deviceId: "",
  theme: "win11Dark",
  closeAction: "ask",
  port: 8765,
  autoStart: false,
  autoSync: true,
  saveHistory: true,
  trustedDevices: [],
  syncText: true,
  syncImage: true,
  syncFiles: true,
  deduplicateSyncContent: true,
  fileSaveDir: null,
  autoOpenFolderAfterSave: false,
  discoveryScanRanges: [],
  desktopNotifications: true,
  notifyClipboard: true,
  notifyTrustRequired: true,
  notifyFileTransfer: false,
  notifyDeviceStatus: true,
  notifySyncError: true,
  notificationClipboardPreview: true,
  translationEngine: "google",
  translationApiUrl: "",
  translationApiKey: "",
  translationModel: "gpt-4o-mini",
  translationProxy: "",
};

export const useConfigStore = defineStore("config", {
  state: () => ({
    config: defaultConfig,
    saving: false,
    error: null as string | null,
    unlisteners: [] as (() => void)[],
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
      if (this.unlisteners.length) {
        return;
      }
      this.unlisteners = await Promise.all([
        onAppEvent<AppConfig>("config-updated", (config) => {
          this.config = config;
        }),
      ]);
    },
  },
});
