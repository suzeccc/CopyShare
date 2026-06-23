import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  applyDeviceDisconnected,
  getDeviceDisconnectNotice,
} from "../src/lib/deviceList.ts";
import type { DeviceInfo } from "../src/types/device.ts";

function trustedDevice(connected: boolean): DeviceInfo {
  return {
    id: "device-remote",
    name: "Office PC",
    ip: "10.194.33.156",
    port: 8765,
    connected,
    trusted: true,
    lastSeenAt: connected ? "2026-06-23T18:51:11Z" : "2026-06-23T18:52:20Z",
    status: connected ? "online" : "offline",
  };
}

const disconnected = applyDeviceDisconnected(
  [trustedDevice(true)],
  trustedDevice(false),
);

assert.equal(disconnected.length, 1);
assert.equal(disconnected[0].id, "device-remote");
assert.equal(disconnected[0].trusted, true);
assert.equal(disconnected[0].connected, false);
assert.equal(disconnected[0].status, "offline");
assert.equal(disconnected[0].lastSeenAt, "2026-06-23T18:52:20Z");

assert.match(getDeviceDisconnectNotice(trustedDevice(false)), /Office PC/);
assert.match(getDeviceDisconnectNotice(trustedDevice(false)), /已断开连接/);

const devicesStore = readFileSync("src/stores/devices.ts", "utf8");
const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");

assert.match(devicesStore, /disconnectNotice/);
assert.match(devicesStore, /applyDeviceDisconnected/);
assert.match(devicesStore, /getDeviceDisconnectNotice/);
assert.match(
  devicesStore,
  /onAppEvent<DeviceInfo>\("device-disconnected",\s*\(device\) => \{[\s\S]*useStatusStore\(\)\.refresh\(\)/,
);
assert.match(appShell, /data-device-disconnect-notice/);
assert.match(appShell, /devicesStore\.disconnectNotice/);
assert.match(appShell, /devicesStore\.clearDisconnectNotice\(\)/);
