import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const devicesPage = readFileSync("src/pages/Devices.vue", "utf8");

assert.match(
  devicesPage,
  /<Card\s+v-if="devicesStore\.connected\.length">\s*<div class="flex items-start justify-between gap-4">[\s\S]*?已连接设备/,
);
assert.match(devicesPage, /<Card>\s*<p class="text-sm font-semibold text-white">快速配置<\/p>/);
assert.doesNotMatch(devicesPage, /还没有已信任的连接设备。先在设备列表确认信任。/);
assert.match(devicesPage, /RefreshButton[\s\S]*devicesStore\.refresh\(\)/);
assert.match(
  devicesPage,
  /:class="\[\s*devicesStore\.connected\.length \? 'xl:grid-cols-\[0\.85fr_1\.15fr\]' : ''/,
);
