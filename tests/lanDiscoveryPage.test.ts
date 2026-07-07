import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const devicesPage = readFileSync("src/pages/Devices.vue", "utf8");
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
assert.match(devicesPage, /devicesStore\.refresh\(\)/);
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
