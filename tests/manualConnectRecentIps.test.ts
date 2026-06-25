import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const devicesPage = readFileSync("src/pages/Devices.vue", "utf8");
const manualConnectForm = readFileSync("src/components/devices/ManualConnectForm.vue", "utf8");

assert.match(devicesPage, /recentIps\s*=\s*computed/);
assert.match(devicesPage, /devicesStore\.history/);
assert.match(devicesPage, /:recent-ips="recentIps"/);

assert.match(manualConnectForm, /recentIps\?: string\[\]/);
assert.match(manualConnectForm, /showRecentIps/);
assert.match(manualConnectForm, /data-recent-ip-button/);
assert.match(manualConnectForm, /data-recent-ip-list/);
assert.match(manualConnectForm, /v-for="recentIp in recentIps"/);
assert.match(manualConnectForm, /emit\(["']update:ip["'], recentIp\)/);
assert.match(manualConnectForm, /List/);
