import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const devicesPage = readFileSync("src/pages/Devices.vue", "utf8");
const devicesStore = readFileSync("src/stores/devices.ts", "utf8");
const wizard = readFileSync("src/components/onboarding/FirstRunWizard.vue", "utf8");
const deviceCard = readFileSync("src/components/devices/DeviceCard.vue", "utf8");

assert.match(devicesPage, /data-lan-discovery-scan-button/);
assert.ok(
  devicesPage.indexOf("data-lan-discovery-scan-button") <
    devicesPage.indexOf("<ManualConnectForm"),
);
assert.match(devicesPage, /scanLanDevices/);
assert.match(devicesPage, /lanDiscoveryScanning/);
assert.match(devicesPage, /toastStore\.info/);
assert.match(devicesPage, /toastStore\.success/);
assert.match(devicesPage, /await devicesStore\.scanLanDevices\(\)/);
assert.match(wizard, /await devicesStore\.scanLanDevices\(\)/);
assert.match(devicesStore, /LAN_DISCOVERY_RESPONSE_GRACE_MS/);
assert.match(devicesStore, /finishedAtSeenAt/);
assert.ok(devicesStore.indexOf("await this.refresh()") < devicesStore.indexOf("const discovered = this.history.filter"));
assert.doesNotMatch(devicesPage, /data-lan-discovery-progress/);
assert.doesNotMatch(devicesPage, /scanProgressText/);
assert.doesNotMatch(devicesPage, /扫描完成：/);
assert.match(devicesPage, /未发现局域网设备，请确认对方已启动 CopyShare 并允许防火墙访问/);
assert.doesNotMatch(devicesPage, /data-lan-discovery-ranges/);
assert.doesNotMatch(devicesPage, /data-lan-discovery-range-input/);
assert.doesNotMatch(devicesPage, /data-lan-discovery-range-add-button/);
assert.doesNotMatch(devicesPage, /discoveryScanRanges/);

assert.match(deviceCard, /待连接/);
assert.match(deviceCard, /连接|重新连接/);

const scanStart = devicesStore.indexOf("    async scanLanDevices() {");
const scanEnd = devicesStore.indexOf("    async connect(", scanStart);
const scanMethod = devicesStore.slice(scanStart, scanEnd).trim().replace(/,\s*$/, "");
const scan = vm.runInNewContext(
  ts.transpile(`({ ${scanMethod} }).scanLanDevices`, { target: ts.ScriptTarget.ES2022 }),
  { Date, Set, Promise, setTimeout, LAN_DISCOVERY_SETTLE_TIMEOUT_MS: 9000, LAN_DISCOVERY_RESPONSE_GRACE_MS: 600 },
) as (this: object) => Promise<{ total: number; newCount: number }>;
const existing = { id: "old", connected: false, status: "online" };
const newlyFound = { id: "new", connected: false, status: "online" };
let discovered = [existing];
const scanState = {
  get history() { return discovered; },
  lanDiscoveryProgress: null,
  error: null,
  async refresh() { discovered = [existing, newlyFound]; },
};
assert.deepEqual(JSON.parse(JSON.stringify(await scan.call(scanState))), { total: 2, newCount: 1 });
