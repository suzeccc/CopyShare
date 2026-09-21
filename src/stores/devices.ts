import { defineStore } from "pinia";

import {
  connectDevice,
  disconnectDevice,
  getDevices,
  onAppEvent,
  rejectDevice,
  trustDevice,
} from "@/lib/tauri";
import {
  createManualConnectDraft,
  setManualConnectDraftIp,
  setManualConnectDraftPort,
} from "@/lib/manualConnectDraft";
import {
  applyDeviceDisconnected,
  connectedTrustedDevices,
  getDeviceDisconnectNotice,
  getDeviceRejectedNotice,
  historicalDevices,
  markDeviceDisconnected,
  markDeviceRejected,
  markDeviceTrusted,
  mergeRefreshedDevices,
  pendingTrustDevices,
  shouldSkipManualConnect,
  upsertDevice,
} from "@/lib/deviceList";
import { connectionSuccessMessage, shouldShowConnectionSuccess } from "@/lib/deviceToast";
import { useStatusStore } from "@/stores/status";
import { useToastStore } from "@/stores/toasts";
import type { DeviceInfo, LanDiscoveryProgress } from "@/types/device";

const LAN_DISCOVERY_SETTLE_TIMEOUT_MS = 9000;
const LAN_DISCOVERY_RESPONSE_GRACE_MS = 600;

export const useDevicesStore = defineStore("devices", {
  state: () => ({
    connectDraft: createManualConnectDraft(),
    devices: [] as DeviceInfo[],
    loading: false,
    error: null as string | null,
    disconnectNotice: null as string | null,
    lanDiscoveryProgress: null as LanDiscoveryProgress | null,
    unlisteners: [] as (() => void)[],
  }),
  getters: {
    connected: (state) => connectedTrustedDevices(state.devices),
    history: (state) => historicalDevices(state.devices),
    pendingTrust: (state) => pendingTrustDevices(state.devices),
    trusted: (state) => state.devices.filter((device) => device.trusted),
  },
  actions: {
    async refresh() {
      this.error = null;
      try {
        this.devices = mergeRefreshedDevices(this.devices, await getDevices());
      } catch (error) {
        this.error = String(error);
      }
    },
    async scanLanDevices() {
      const knownIds = new Set(this.history.map((device) => device.id));
      const previousScanId = this.lanDiscoveryProgress?.scanId ?? null;
      await this.refresh();
      if (this.error) throw new Error(this.error);

      const deadline = Date.now() + LAN_DISCOVERY_SETTLE_TIMEOUT_MS;
      let finishedAtSeenAt: number | null = null;
      while (Date.now() < deadline) {
        if (this.history.some((device) => !device.connected && device.status === "online" && !knownIds.has(device.id))) break;
        const progress = this.lanDiscoveryProgress;
        if (progress && progress.scanId !== previousScanId && !progress.running) {
          finishedAtSeenAt ??= Date.now();
          if (Date.now() - finishedAtSeenAt >= LAN_DISCOVERY_RESPONSE_GRACE_MS) break;
        }
        await new Promise<void>((resolve) => setTimeout(resolve, 120));
      }

      const discovered = this.history.filter((device) => !device.connected && device.status === "online");
      return {
        total: discovered.length,
        newCount: discovered.filter((device) => !knownIds.has(device.id)).length,
      };
    },
    async connect(ip: string, port: number) {
      this.error = null;
      if (shouldSkipManualConnect(this.devices, ip, port, this.loading)) {
        return;
      }

      this.loading = true;
      try {
        const device = await connectDevice(ip, port);
        this.upsert(device);
        await useStatusStore().refresh();
      } catch (error) {
        this.error = String(error);
        useToastStore().error("连接失败");
      } finally {
        this.loading = false;
      }
    },
    setConnectDraftIp(ip: string) {
      this.connectDraft = setManualConnectDraftIp(this.connectDraft, ip);
    },
    setConnectDraftPort(port: number) {
      this.connectDraft = setManualConnectDraftPort(this.connectDraft, port);
    },
    async disconnect(deviceId: string) {
      this.error = null;
      await disconnectDevice(deviceId);
      this.devices = markDeviceDisconnected(this.devices, deviceId);
      await useStatusStore().refresh();
    },
    async trust(deviceId: string) {
      this.error = null;
      await trustDevice(deviceId);
      this.devices = markDeviceTrusted(this.devices, deviceId);
      await useStatusStore().refresh();
    },
    async reject(deviceId: string) {
      this.error = null;
      await rejectDevice(deviceId);
      this.devices = markDeviceRejected(this.devices, deviceId);
      await useStatusStore().refresh();
    },
    upsert(device: DeviceInfo) {
      this.devices = upsertDevice(this.devices, device);
    },
    clearDisconnectNotice() {
      this.disconnectNotice = null;
    },
    async subscribe() {
      if (this.unlisteners.length) {
        return;
      }
      this.unlisteners = await Promise.all([
        onAppEvent<DeviceInfo>("device-discovered", (device) => this.upsert(device)),
        onAppEvent<LanDiscoveryProgress>("lan-discovery-progress", (progress) => {
          this.lanDiscoveryProgress = progress;
        }),
        onAppEvent<DeviceInfo>("device-connected", (device) => {
          this.disconnectNotice = null;
          this.upsert(device);
          if (shouldShowConnectionSuccess(device)) {
            useToastStore().success(connectionSuccessMessage(device));
          }
        }),
        onAppEvent<DeviceInfo>("device-disconnected", (device) => {
          this.devices = applyDeviceDisconnected(this.devices, device);
          this.disconnectNotice = getDeviceDisconnectNotice(device);
          void useStatusStore().refresh();
        }),
        onAppEvent<DeviceInfo>("device-rejected", (device) => {
          this.devices = applyDeviceDisconnected(this.devices, device);
          this.disconnectNotice = getDeviceRejectedNotice(device);
          void useStatusStore().refresh();
        }),
      ]);
    },
  },
});
