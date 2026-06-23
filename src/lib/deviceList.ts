import type { DeviceInfo } from "@/types/device";

export function upsertDevice(devices: DeviceInfo[], device: DeviceInfo): DeviceInfo[] {
  const existing = devices.find(
    (item) =>
      item.id === device.id ||
      sameDeviceEndpoint(item, device),
  );
  const merged = existing ? mergeDevice(existing, device) : device;
  const next = devices.filter(
    (item) =>
      item.id !== merged.id &&
      !sameDeviceEndpoint(item, merged),
  );

  return [merged, ...next];
}

export function dedupeDevices(devices: DeviceInfo[]): DeviceInfo[] {
  return devices.reduce<DeviceInfo[]>(
    (deduped, device) => upsertDevice(deduped, device),
    [],
  );
}

export function mergeRefreshedDevices(
  currentDevices: DeviceInfo[],
  refreshedDevices: DeviceInfo[],
): DeviceInfo[] {
  return dedupeDevices([...currentDevices, ...refreshedDevices]);
}

export function connectedTrustedDevices(devices: DeviceInfo[]): DeviceInfo[] {
  return dedupeDevices(devices).filter((device) => device.connected && device.trusted);
}

export function pendingTrustDevices(devices: DeviceInfo[]): DeviceInfo[] {
  return dedupeDevices(devices).filter((device) => device.connected && !device.trusted);
}

export function markDeviceTrusted(
  devices: DeviceInfo[],
  deviceKey: string,
): DeviceInfo[] {
  return devices.map((device) =>
    deviceMatchesKey(device, deviceKey) ? { ...device, trusted: true } : device,
  );
}

export function markDeviceDisconnected(
  devices: DeviceInfo[],
  deviceKey: string,
): DeviceInfo[] {
  return devices.map((device) =>
    deviceMatchesKey(device, deviceKey)
      ? { ...device, connected: false, status: "offline" }
      : device,
  );
}

export function applyDeviceDisconnected(
  devices: DeviceInfo[],
  disconnectedDevice: DeviceInfo,
): DeviceInfo[] {
  const existing = devices.find(
    (device) =>
      device.id === disconnectedDevice.id ||
      sameDeviceEndpoint(device, disconnectedDevice),
  );
  const deviceKey = existing?.id ?? disconnectedDevice.id;
  const trusted = existing?.trusted || disconnectedDevice.trusted;
  const next = upsertDevice(devices, {
    ...disconnectedDevice,
    connected: false,
    trusted,
    status: "offline",
  });

  return next.map((device) =>
    deviceMatchesKey(device, deviceKey)
      ? {
          ...device,
          connected: false,
          trusted: device.trusted || trusted,
          status: "offline",
          lastSeenAt: disconnectedDevice.lastSeenAt ?? device.lastSeenAt,
        }
      : device,
  );
}

export function getDeviceDisconnectNotice(device: DeviceInfo): string {
  const deviceName = device.name.trim() || device.ip;
  return `${deviceName} 已断开连接，状态已更新为离线`;
}

export function removeDeviceByKey(
  devices: DeviceInfo[],
  deviceKey: string,
): DeviceInfo[] {
  return devices.filter((device) => !deviceMatchesKey(device, deviceKey));
}

export function hasConnectedDeviceEndpoint(
  devices: DeviceInfo[],
  ip: string,
  port: number,
): boolean {
  const targetKey = endpointKeyFromAddress(ip, port);
  const targetHost = hostKeyFromAddress(ip);

  return dedupeDevices(devices).some(
    (device) =>
      device.connected &&
      (endpointKeys(device).includes(targetKey) || hostKeys(device).includes(targetHost)),
  );
}

export function shouldSkipManualConnect(
  devices: DeviceInfo[],
  ip: string,
  port: number,
  loading: boolean,
): boolean {
  return loading || hasConnectedDeviceEndpoint(devices, ip, port);
}

function endpointKey(device: DeviceInfo): string {
  return endpointKeyFromAddress(device.ip, device.port);
}

function endpointKeys(device: DeviceInfo): string[] {
  const keys = [endpointKey(device)];

  if (isEndpointAlias(device.id)) {
    keys.push(endpointKeyFromAddress(device.id, device.port));
  }

  return unique(keys);
}

function hostKeys(device: DeviceInfo): string[] {
  const keys = [hostKeyFromAddress(device.ip)];

  if (isEndpointAlias(device.id)) {
    keys.push(hostKeyFromAddress(device.id));
  }

  return unique(keys);
}

function sameDeviceEndpoint(left: DeviceInfo, right: DeviceInfo): boolean {
  return hasIntersection(endpointKeys(left), endpointKeys(right));
}

function deviceMatchesKey(device: DeviceInfo, deviceKey: string): boolean {
  if (device.id === deviceKey) {
    return true;
  }

  const keyEndpoint = endpointKeyFromAddress(deviceKey, device.port);
  const keyHost = hostKeyFromAddress(deviceKey);

  return (
    endpointKeys(device).includes(keyEndpoint) ||
    (device.connected && hostKeys(device).includes(keyHost))
  );
}

function mergeDevice(existing: DeviceInfo, incoming: DeviceInfo): DeviceInfo {
  if (existing.connected && !incoming.connected) {
    return {
      ...existing,
      trusted: existing.trusted || incoming.trusted,
      lastSeenAt: incoming.lastSeenAt ?? existing.lastSeenAt,
    };
  }

  if (existing.connected && existing.trusted && !incoming.trusted) {
    return {
      ...existing,
      connected: true,
      lastSeenAt: incoming.lastSeenAt ?? existing.lastSeenAt,
      status: "online",
    };
  }

  return {
    ...incoming,
    trusted: incoming.trusted || (existing.connected && existing.trusted),
    connected: incoming.connected || existing.connected,
    status: incoming.connected || existing.connected ? "online" : incoming.status,
  };
}

function endpointKeyFromAddress(value: string, fallbackPort: number): string {
  const trimmed = value.trim();

  try {
    const url = new URL(trimmed.includes("://") ? trimmed : `ws://${trimmed}`);
    return `${url.hostname}:${Number(url.port || fallbackPort)}`;
  } catch {
    const [host, maybePort] = trimmed.split(":");
    const parsedPort = Number(maybePort);
    const port = Number.isInteger(parsedPort) && parsedPort > 0 ? parsedPort : fallbackPort;

    return `${maybePort ? host : trimmed}:${port}`;
  }
}

function hostKeyFromAddress(value: string): string {
  const trimmed = value.trim();

  try {
    const url = new URL(trimmed.includes("://") ? trimmed : `ws://${trimmed}`);
    return url.hostname;
  } catch {
    const [host] = trimmed.split(":");
    return host || trimmed;
  }
}

function isEndpointAlias(value: string): boolean {
  const trimmed = value.trim();
  return trimmed.includes("://") || hasExplicitPort(trimmed);
}

function hasExplicitPort(value: string): boolean {
  const [host, port] = splitHostPort(value);
  return Boolean(host) && Number.isInteger(Number(port)) && Number(port) > 0;
}

function splitHostPort(value: string): [string, string | undefined] {
  const index = value.lastIndexOf(":");
  if (index <= 0 || index === value.length - 1) {
    return [value, undefined];
  }

  return [value.slice(0, index), value.slice(index + 1)];
}

function hasIntersection(left: string[], right: string[]): boolean {
  return left.some((item) => right.includes(item));
}

function unique(values: string[]): string[] {
  return values.filter((value, index) => values.indexOf(value) === index);
}
