import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const logsPage = readFileSync("src/pages/Logs.vue", "utf8");
const deviceCard = readFileSync("src/components/devices/DeviceCard.vue", "utf8");

assert.match(logsPage, /连接状态/);
assert.match(logsPage, /连接失败和断开状态会同步到设备列表/);
assert.match(logsPage, /devicesStore\.devices/);
assert.match(logsPage, /historyStore\.items/);
assert.match(logsPage, /DeviceCard/);
assert.match(logsPage, /mode="status"/);
assert.match(logsPage, /v-for="device in devicesStore\.devices"/);
assert.match(logsPage, /v-for="item in historyStore\.items"/);

assert.match(deviceCard, /已离线/);
assert.match(deviceCard, /设备已断开连接/);
assert.match(deviceCard, /const status = computed/);
