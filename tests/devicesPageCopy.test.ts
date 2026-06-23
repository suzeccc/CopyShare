import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const devices = readFileSync("src/pages/Devices.vue", "utf8");

assert.match(devices, /历史连接设备列表/);
assert.doesNotMatch(devices, />设备列表</);
