import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const deviceCard = readFileSync("src/components/devices/DeviceCard.vue", "utf8");
const devicesPage = readFileSync("src/pages/Devices.vue", "utf8");

assert.match(deviceCard, /showCancelAwaitingTrustButton/);
assert.match(deviceCard, /props\.mode === "status"/);
assert.match(deviceCard, /props\.device\.trusted/);
assert.match(deviceCard, /!props\.device\.remoteTrusted/);
assert.match(deviceCard, /data-device-cancel-awaiting-trust-button/);
assert.match(deviceCard, /@click="\$emit\('reject', device\.id\)"/);
assert.match(deviceCard, /取消等待/);
assert.match(deviceCard, /ShieldX/);
assert.match(deviceCard, /mt-4 flex/);
assert.match(devicesPage, /@reject="devicesStore\.reject"/);
