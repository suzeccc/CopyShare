import type { DeviceInfo } from "@/types/device";

export function firstNamedTrustDevice(devices: DeviceInfo[]): DeviceInfo | null {
  return namedTrustDevices(devices)[0] ?? null;
}

export function namedTrustDevices(devices: DeviceInfo[]): DeviceInfo[] {
  return devices.filter((device) => hasDisplayDeviceName(device.name));
}

export function hasDisplayDeviceName(name: string): boolean {
  const value = name.trim();

  return Boolean(value) && !isConnectionAddress(value);
}

function isConnectionAddress(value: string): boolean {
  return value.includes("://") || /^[a-zA-Z0-9.-]+:\d+$/.test(value);
}
