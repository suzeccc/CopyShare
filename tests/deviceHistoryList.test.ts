import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const devicesPage = readFileSync("src/pages/Devices.vue", "utf8");
const devicesStore = readFileSync("src/stores/devices.ts", "utf8");

assert.match(devicesStore, /history:\s*\(state\) => historicalDevices\(state\.devices\)/);
assert.match(devicesPage, /devicesStore\.history\.length/);
assert.match(devicesPage, /v-for="device in devicesStore\.history"/);
assert.match(devicesPage, /mode="status"/);
assert.match(devicesPage, /:show-actions="device\.connected"/);
assert.doesNotMatch(devicesPage, /v-for="device in devicesStore\.pendingTrust"/);
