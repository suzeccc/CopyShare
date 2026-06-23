import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const logs = readFileSync("src/pages/Logs.vue", "utf8");
const deviceCard = readFileSync("src/components/devices/DeviceCard.vue", "utf8");

assert.match(logs, /mode="status"/);
assert.match(logs, /:show-actions="false"/);
assert.doesNotMatch(logs, /@trust=/);
assert.doesNotMatch(logs, /@reject=/);
assert.doesNotMatch(logs, /@disconnect=/);

assert.match(deviceCard, /mode\?: "pending" \| "connected" \| "status"/);
assert.match(deviceCard, /showActions\?: boolean/);
assert.match(deviceCard, /v-if="showActions"/);
assert.match(deviceCard, /已离线/);
assert.match(deviceCard, /连接正常/);
