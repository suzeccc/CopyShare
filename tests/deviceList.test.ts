import assert from "node:assert/strict";

import {
  connectedTrustedDevices,
  dedupeDevices,
  hasConnectedDeviceEndpoint,
  historicalDevices,
  markDeviceDisconnected,
  markDeviceTrusted,
  mergeRefreshedDevices,
  pendingTrustDevices,
  removeDeviceByKey,
  shouldSkipManualConnect,
  upsertDevice,
} from "../src/lib/deviceList.ts";
import type { DeviceInfo } from "../src/types/device.ts";

function device(id: string, connected: boolean): DeviceInfo {
  return {
    id,
    name: "CopyShare",
    ip: "10.194.33.156",
    port: 8765,
    connected,
    trusted: false,
    lastSeenAt: "2026-06-23T00:19:42Z",
    status: connected ? "online" : "offline",
  };
}

function trustedDevice(id: string, connected: boolean): DeviceInfo {
  return {
    ...device(id, connected),
    trusted: true,
  };
}

const duplicateByAddress = upsertDevice(
  [device("10.194.33.156:51234", false)],
  device("device-remote", true),
);

assert.equal(duplicateByAddress.length, 1);
assert.equal(duplicateByAddress[0].id, "device-remote");
assert.equal(duplicateByAddress[0].connected, true);

const existingTrusted = trustedDevice("device-remote", true);
const reconnectPlaceholder = device("ws://10.194.33.156:8765/", true);
const reconnectResult = upsertDevice([existingTrusted], reconnectPlaceholder);

assert.equal(reconnectResult.length, 1);
assert.equal(reconnectResult[0].id, "device-remote");
assert.equal(reconnectResult[0].trusted, true);
assert.equal(reconnectResult[0].connected, true);

const connectedPlaceholder = device("ws://10.194.33.156:8765/", true);
const trustedOfflineRefresh = trustedDevice("device-remote", false);
const trustedRefreshResult = upsertDevice([connectedPlaceholder], trustedOfflineRefresh);

assert.equal(trustedRefreshResult.length, 1);
assert.equal(trustedRefreshResult[0].connected, true);
assert.equal(trustedRefreshResult[0].trusted, true);

assert.deepEqual(
  pendingTrustDevices([existingTrusted, reconnectPlaceholder]).map((item) => item.id),
  [],
);
assert.deepEqual(
  connectedTrustedDevices([existingTrusted, reconnectPlaceholder]).map((item) => item.id),
  ["device-remote"],
);

const connectedTrustedHost = trustedDevice("device-remote", true);
const repeatedPendingSameHost = {
  ...device("ws://10.194.33.156:51234/", true),
  port: 51234,
};

assert.deepEqual(
  pendingTrustDevices([connectedTrustedHost, repeatedPendingSameHost]).map(
    (item) => item.id,
  ),
  ["ws://10.194.33.156:51234/"],
);
assert.deepEqual(
  connectedTrustedDevices([connectedTrustedHost, repeatedPendingSameHost]).map(
    (item) => item.id,
  ),
  ["device-remote"],
);
assert.equal(
  hasConnectedDeviceEndpoint([connectedTrustedHost], "10.194.33.156", 51234),
  true,
);

const repeatedPendingEndpointAlias = {
  ...device("ws://10.194.33.156:8765/", true),
  ip: "CopyShare",
};

assert.deepEqual(
  pendingTrustDevices([connectedTrustedHost, repeatedPendingEndpointAlias]).map(
    (item) => item.id,
  ),
  [],
);
assert.deepEqual(
  connectedTrustedDevices([connectedTrustedHost, repeatedPendingEndpointAlias]).map(
    (item) => item.id,
  ),
  ["device-remote"],
);

assert.equal(
  hasConnectedDeviceEndpoint([existingTrusted], "10.194.33.156", 8765),
  true,
);
assert.equal(
  hasConnectedDeviceEndpoint([existingTrusted], "ws://10.194.33.156:8765/", 8765),
  true,
);
assert.equal(
  hasConnectedDeviceEndpoint([trustedDevice("device-remote", false)], "10.194.33.156", 8765),
  false,
);
assert.equal(
  shouldSkipManualConnect([existingTrusted], "10.194.33.156", 8765, false),
  true,
);
assert.equal(
  shouldSkipManualConnect([], "10.194.33.156", 8765, true),
  true,
);
assert.equal(
  shouldSkipManualConnect([], "10.194.33.156", 8765, false),
  false,
);

const staleTrustedSameHost = trustedDevice("old-device", false);
const newUntrustedSameHost = {
  ...device("new-device", true),
  port: 8766,
};
const sameHostTrustResult = upsertDevice([staleTrustedSameHost], newUntrustedSameHost);

assert.deepEqual(
  pendingTrustDevices(sameHostTrustResult).map((item) => item.id),
  ["new-device"],
);
assert.deepEqual(
  connectedTrustedDevices(sameHostTrustResult).map((item) => item.id),
  [],
);

const duplicateRefresh = dedupeDevices([
  device("10.194.33.156:51234", false),
  device("device-remote", true),
  { ...device("another-device", false), ip: "10.194.33.157" },
]);

assert.equal(duplicateRefresh.length, 2);
assert.deepEqual(
  duplicateRefresh.map((item) => item.id),
  ["another-device", "device-remote"],
);

const awaitingTrust = { ...device("awaiting-trust", true), ip: "10.194.33.158" };
const trustedConnected = trustedDevice("trusted-connected", true);
const trustedOffline = { ...trustedDevice("trusted-offline", false), ip: "10.194.33.160" };

assert.deepEqual(
  pendingTrustDevices([awaitingTrust, trustedConnected, trustedOffline]).map(
    (item) => item.id,
  ),
  ["awaiting-trust"],
);

assert.deepEqual(
  connectedTrustedDevices([awaitingTrust, trustedConnected, trustedOffline]).map(
    (item) => item.id,
  ),
  ["trusted-connected"],
);

assert.deepEqual(
  historicalDevices([
    awaitingTrust,
    trustedConnected,
    trustedOffline,
    { ...device("ws://10.194.33.156:51234/", true), port: 51234 },
  ]).map((item) => item.id),
  ["trusted-connected", "trusted-offline", "awaiting-trust"],
);

const aliasTrusted = markDeviceTrusted(
  [device("device-remote", true)],
  "ws://10.194.33.156:8765/",
);

assert.equal(aliasTrusted[0].id, "device-remote");
assert.equal(aliasTrusted[0].trusted, true);

const aliasDisconnected = markDeviceDisconnected(
  [trustedDevice("device-remote", true)],
  "ws://10.194.33.156:8765/",
);

assert.equal(aliasDisconnected[0].id, "device-remote");
assert.equal(aliasDisconnected[0].connected, false);
assert.equal(aliasDisconnected[0].status, "offline");

assert.deepEqual(
  removeDeviceByKey(
    [trustedDevice("device-remote", true)],
    "ws://10.194.33.156:8765/",
  ),
  [],
);

const connectedBeforeRefresh = trustedDevice("device-remote", true);
assert.deepEqual(
  mergeRefreshedDevices([connectedBeforeRefresh], []),
  [connectedBeforeRefresh],
);

const discoveredDuringRefresh = {
  ...device("new-device", true),
  ip: "10.194.33.159",
};
assert.deepEqual(
  mergeRefreshedDevices([connectedBeforeRefresh], [discoveredDuringRefresh]).map(
    (item) => item.id,
  ),
  ["new-device", "device-remote"],
);
