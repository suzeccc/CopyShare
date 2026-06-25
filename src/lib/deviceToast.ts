import { deviceAddress } from "./format";
import type { DeviceInfo } from "../types/device";

export function connectionSuccessMessage(device: DeviceInfo): string {
  return `${displayDeviceName(device)} · ${deviceAddress(device.ip, device.port)} · 连接成功`;
}

export function displayDeviceName(device: DeviceInfo): string {
  const name = device.name.trim();

  return name && !isConnectionAddress(name) ? name : "对方设备";
}

export function hasRealDeviceName(device: DeviceInfo): boolean {
  return displayDeviceName(device) === device.name.trim();
}

function isConnectionAddress(value: string): boolean {
  return value.includes("://") || /^[a-zA-Z0-9.-]+:\d+$/.test(value);
}
