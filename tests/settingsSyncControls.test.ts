import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const settings = readFileSync("src/pages/Settings.vue", "utf8");

assert.match(settings, /async function saveSyncImage\(syncImage: boolean\)/);
assert.match(settings, /async function saveSyncFiles\(syncFiles: boolean\)/);
assert.match(settings, /async function saveSyncSetting/);
assert.match(settings, /async function saveSyncDirection\(syncDirection: SyncDirection\)/);
assert.match(settings, /data-sync-direction-setting/);
assert.match(settings, /:value="draft\.syncDirection"/);
assert.match(settings, /value as SyncDirection/);
assert.match(settings, /:model-value="draft\.syncImage"/);
assert.match(settings, /@update:model-value="saveSyncImage"/);
assert.match(settings, /:model-value="draft\.syncFiles"/);
assert.match(settings, /@update:model-value="saveSyncFiles"/);
assert.match(settings, /:disabled="configMutationSaving"/);
assert.match(settings, /复制截图或图片后同步到其他设备；关闭不影响本机复制/);
assert.match(settings, /复制文件后同步到其他设备；关闭不影响本机复制/);
assert.doesNotMatch(settings, />支持截图和图片复制</);
assert.doesNotMatch(settings, />复制文件后同步到对方历史</);
assert.doesNotMatch(settings, /<Switch v-model="draft\.syncImage"/);
assert.doesNotMatch(settings, /syncFiles:\s*false/);
assert.doesNotMatch(settings, /getSaveFeedbackView\(configStore\.saving \? "saving" : saveFeedbackState\.value\)/);
assert.doesNotMatch(settings, /已信任设备/);
