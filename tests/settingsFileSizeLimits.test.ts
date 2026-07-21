import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  FILE_SIZE_LIMIT_MAX_MIB,
  FILE_SIZE_LIMIT_MIN_MIB,
  adjustFileSizeLimitFromWheel,
  clampFileSizeLimitMib,
  formatFileSizeLimit,
} from "../src/lib/fileSizeLimit.ts";

assert.equal(FILE_SIZE_LIMIT_MIN_MIB, 100);
assert.equal(FILE_SIZE_LIMIT_MAX_MIB, 2048);
assert.equal(clampFileSizeLimitMib(50), 100);
assert.equal(clampFileSizeLimitMib(512.8), 513);
assert.equal(clampFileSizeLimitMib(4096), 2048);

assert.equal(adjustFileSizeLimitFromWheel(500, -1), 600);
assert.equal(adjustFileSizeLimitFromWheel(500, 1), 400);
assert.equal(adjustFileSizeLimitFromWheel(2000, -1), 2048);
assert.equal(adjustFileSizeLimitFromWheel(2048, -1), 2048);
assert.equal(adjustFileSizeLimitFromWheel(100, 1), 100);
assert.equal(adjustFileSizeLimitFromWheel(500, 0), 500);

assert.equal(formatFileSizeLimit(500), "500 MiB");
assert.equal(formatFileSizeLimit(1536), "1.5 GiB");
assert.equal(formatFileSizeLimit(2048), "2 GiB");

const settings = readFileSync("src/pages/Settings.vue", "utf8");
const slider = readFileSync("src/components/settings/FileSizeLimitSlider.vue", "utf8");
const configType = readFileSync("src/types/config.ts", "utf8");
const configStore = readFileSync("src/stores/config.ts", "utf8");

assert.match(configType, /maxSendFileSizeMib: number/);
assert.match(configType, /maxReceiveFileSizeMib: number/);
assert.match(configStore, /maxSendFileSizeMib:\s*2048/);
assert.match(configStore, /maxReceiveFileSizeMib:\s*2048/);
assert.match(settings, /v-model="draft\.maxSendFileSizeMib"/);
assert.match(settings, /v-model="draft\.maxReceiveFileSizeMib"/);
assert.match(settings, /单任务总上限仍为 5 GiB/);
assert.match(slider, /type="range"/);
assert.match(slider, /@wheel="onWheel"/);
assert.match(slider, /:aria-valuetext="formattedValue"/);
assert.match(slider, /@change="onChange"/);
