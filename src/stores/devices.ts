import { defineStore } from "pinia";

import {
  connectDevice,
  disconnectDevice,
  getDevices,
  onAppEvent,
  trustDevice,
} from "@/lib/tauri";
import type { DeviceInfo } from "@/types/device";

export const useDevicesStore = defineStore("devices", {
  state: () => ({
    devices: [] as DeviceInfo[],
    loading: false,
    error: null as string | null,
  }),
  getters: {
    connected: (state) => state.devices.filter((device) => device.connected),
    trusted: (state) => state.devices.filter((device) => device.trusted),
  },
  actions: {
    async refresh() {
      this.error = null;
      try {
        this.devices = await getDevices();
      } catch (error) {
        this.error = String(error);
      }
    },
    async connect(ip: string, port: number) {
      this.loading = true;
      this.error = null;
      try {
        const device = await connectDevice(ip, port);
        this.upsert(device);
      } catch (error) {
        this.error = String(error);
      } finally {
        this.loading = false;
      }
    },
    async disconnect(deviceId: string) {
      this.error = null;
      await disconnectDevice(deviceId);
      this.devices = this.devices.map((device) =>
        device.id === deviceId ? { ...device, connected: false, status: "offline" } : device,
      );
    },
    async trust(deviceId: string) {
      this.error = null;
      await trustDevice(deviceId);
      this.devices = this.devices.map((device) =>
        device.id === deviceId ? { ...device, trusted: true } : device,
      );
    },
    upsert(device: DeviceInfo) {
      const index = this.devices.findIndex((item) => item.id === device.id);
      if (index >= 0) {
        this.devices[index] = device;
      } else {
        this.devices.unshift(device);
      }
    },
    async subscribe() {
      await Promise.all([
        onAppEvent<DeviceInfo>("device-discovered", (device) => this.upsert(device)),
        onAppEvent<DeviceInfo>("device-connected", (device) => this.upsert(device)),
        onAppEvent<DeviceInfo>("device-disconnected", (device) => this.upsert(device)),
      ]);
    },
  },
});
