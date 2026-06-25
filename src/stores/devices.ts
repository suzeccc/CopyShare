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
  historicalDevices,
  markDeviceDisconnected,
  markDeviceTrusted,
  mergeRefreshedDevices,
  pendingTrustDevices,
  removeDeviceByKey,
  shouldSkipManualConnect,
  upsertDevice,
} from "@/lib/deviceList";
import { connectionSuccessMessage, hasRealDeviceName } from "@/lib/deviceToast";
import { useStatusStore } from "@/stores/status";
import { useToastStore } from "@/stores/toasts";
import type { DeviceInfo } from "@/types/device";

export const useDevicesStore = defineStore("devices", {
  state: () => ({
    connectDraft: createManualConnectDraft(),
    devices: [] as DeviceInfo[],
    loading: false,
    error: null as string | null,
    disconnectNotice: null as string | null,
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
      this.devices = removeDeviceByKey(this.devices, deviceId);
      await useStatusStore().refresh();
    },
    upsert(device: DeviceInfo) {
      this.devices = upsertDevice(this.devices, device);
    },
    clearDisconnectNotice() {
      this.disconnectNotice = null;
    },
    async subscribe() {
      await Promise.all([
        onAppEvent<DeviceInfo>("device-discovered", (device) => this.upsert(device)),
        onAppEvent<DeviceInfo>("device-connected", (device) => {
          this.disconnectNotice = null;
          this.upsert(device);
          if (hasRealDeviceName(device)) {
            useToastStore().success(connectionSuccessMessage(device));
          }
        }),
        onAppEvent<DeviceInfo>("device-disconnected", (device) => {
          this.devices = applyDeviceDisconnected(this.devices, device);
          this.disconnectNotice = getDeviceDisconnectNotice(device);
          void useStatusStore().refresh();
        }),
      ]);
    },
  },
});
