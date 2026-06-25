import assert from "node:assert/strict";

import {
  firstNamedTrustDevice,
  hasDisplayDeviceName,
  namedTrustDevices,
} from "../src/lib/trustPrompt.ts";
import type { DeviceInfo } from "../src/types/device.ts";

function device(overrides: Partial<DeviceInfo>): DeviceInfo {
  return {
    id: "device-remote",
    name: "Remote PC",
    ip: "10.194.33.156",
    port: 8765,
    connected: true,
    trusted: false,
    lastSeenAt: null,
    status: "online",
    ...overrides,
  };
}

assert.equal(hasDisplayDeviceName("Sz"), true);
assert.equal(hasDisplayDeviceName("ws://10.194.33.156:8765/"), false);
assert.equal(hasDisplayDeviceName("10.194.33.156:8765"), false);

assert.deepEqual(
  firstNamedTrustDevice([
    device({
      id: "ws://10.194.33.156:8765/",
      name: "ws://10.194.33.156:8765/",
    }),
    device({
      id: "device-sz",
      name: "Sz",
    }),
  ])?.name,
  "Sz",
);

assert.deepEqual(
  namedTrustDevices([
    device({
      id: "ws://10.194.33.156:8765/",
      name: "ws://10.194.33.156:8765/",
    }),
    device({
      id: "device-sz",
      name: "Sz",
    }),
  ]).map((item) => item.name),
  ["Sz"],
);

assert.equal(
  firstNamedTrustDevice([
    device({
      id: "ws://10.194.33.156:8765/",
      name: "ws://10.194.33.156:8765/",
    }),
  ]),
  null,
);
