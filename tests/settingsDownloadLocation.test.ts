import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const settings = readFileSync("src/pages/Settings.vue", "utf8");
const tauri = readFileSync("src/lib/tauri.ts", "utf8");
const configTypes = readFileSync("src/types/config.ts", "utf8");

assert.match(configTypes, /fileSaveDir:\s*string\s*\|\s*null/);
assert.match(tauri, /selectTransferSaveDir/);
assert.match(tauri, /resetTransferSaveDir/);
assert.match(tauri, /openTransferFolder/);

assert.match(settings, /selectTransferSaveDir/);
assert.match(settings, /resetTransferSaveDir/);
assert.match(settings, /openTransferFolder/);
assert.match(settings, /draft\.fileSaveDir/);
assert.match(settings, /data-download-location-setting/);
assert.match(settings, /\u4e0b\u8f7d\u4f4d\u7f6e/);
assert.match(settings, /\u66f4\u6539\u4f4d\u7f6e/);
assert.match(settings, /\u6062\u590d\u9ed8\u8ba4/);
assert.match(settings, /\u6253\u5f00\u6587\u4ef6\u5939/);
