import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const devicesPage = readFileSync("src/pages/Devices.vue", "utf8");
const deviceCard = readFileSync("src/components/devices/DeviceCard.vue", "utf8");

assert.match(devicesPage, /历史连接设备/);
assert.match(devicesPage, /devicesStore\.history/);
assert.match(devicesPage, /v-for="device in devicesStore\.history"/);
assert.match(devicesPage, /mode="status"/);
assert.match(devicesPage, /@reconnect="devicesStore\.connect"/);
assert.match(devicesPage, /连接成功、等待确认和已断开的设备都会保留在这里/);
assert.doesNotMatch(devicesPage, /同一设备只显示一张卡片/);

assert.match(deviceCard, /mode\?: "pending" \| "connected" \| "status"/);
assert.match(deviceCard, /v-if="mode === 'pending'"/);
assert.match(deviceCard, /v-else-if="mode === 'status' && !device\.connected"/);
assert.match(deviceCard, /重新连接/);
