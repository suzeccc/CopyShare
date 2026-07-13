import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const pagePath = "src/pages/Ocr.vue";
const router = readFileSync("src/router/index.ts", "utf8");
const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");

assert.equal(existsSync(pagePath), true);
const page = readFileSync(pagePath, "utf8");

assert.match(router, /import Ocr from "@\/pages\/Ocr\.vue"/);
assert.match(router, /path: "\/ocr", name: "ocr", component: Ocr/);
assert.match(sidebar, /ScanText/);
assert.match(sidebar, /label: "图转文字", path: "\/ocr", icon: ScanText/);
assert.ok(sidebar.indexOf('label: "图片转文字"') < sidebar.indexOf('label: "翻译"'));

for (const hook of [
  "data-ocr-page",
  "data-ocr-paste-zone",
  "data-ocr-loading",
  "data-ocr-preview",
  "data-ocr-result",
  "data-ocr-copy",
  "data-ocr-clear",
]) {
  assert.match(page, new RegExp(hook));
}
assert.match(page, /@paste\.prevent="handlePaste"/);
assert.match(page, /recognizeClipboardImage/);
assert.match(page, /useOcrStore/);
assert.match(page, /v-model="resultText"/);
