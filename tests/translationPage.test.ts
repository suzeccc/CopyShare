import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const pagePath = "src/pages/Translate.vue";
const router = readFileSync("src/router/index.ts", "utf8");
const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");
const tauri = readFileSync("src/lib/tauri.ts", "utf8");
const configTypes = readFileSync("src/types/config.ts", "utf8");
const configStore = readFileSync("src/stores/config.ts", "utf8");
const settings = readFileSync("src/pages/Settings.vue", "utf8");
const models = readFileSync("src-tauri/src/models.rs", "utf8");
const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
const libRs = readFileSync("src-tauri/src/lib.rs", "utf8");
const cargoToml = readFileSync("src-tauri/Cargo.toml", "utf8");

assert.equal(existsSync(pagePath), true);
const page = readFileSync(pagePath, "utf8");

assert.match(router, /import Translate from "@\/pages\/Translate\.vue"/);
assert.match(router, /path: "\/translate"/);
assert.match(router, /name: "translate"/);

assert.match(sidebar, /Languages/);
assert.match(sidebar, /label: "翻译"/);
assert.match(sidebar, /path: "\/translate"/);

const expectedSidebarOrder = [
  "总览",
  "剪切板",
  "设备连接",
  "翻译",
  "日志",
];
const sidebarPositions = expectedSidebarOrder.map((label) =>
  sidebar.indexOf(`label: "${label}"`),
);
assert.equal(sidebarPositions.every((position) => position >= 0), true);
assert.deepEqual(
  [...sidebarPositions].sort((left, right) => left - right),
  sidebarPositions,
);

assert.match(tauri, /import type \{ TranslateResponse \} from "@\/types\/translation"/);
assert.match(tauri, /function translateText\(text: string, targetLang: string\): Promise<TranslateResponse>/);
assert.match(tauri, /invoke<TranslateResponse>\("translate_text"/);

assert.match(configTypes, /export type TranslationEngine = "google" \| "ai"/);
assert.match(configTypes, /translationEngine: TranslationEngine/);
assert.match(configTypes, /translationApiUrl: string/);
assert.match(configTypes, /translationApiKey: string/);
assert.match(configTypes, /translationModel: string/);
assert.match(configTypes, /translationProxy: string/);

assert.match(configStore, /configVersion: 6/);
assert.match(configStore, /translationEngine: "google"/);
assert.match(configStore, /translationModel: "gpt-4o-mini"/);

assert.match(settings, /data-translation-settings/);
assert.match(settings, /translationEngineOptions/);
assert.match(settings, /saveTranslationSetting/);
assert.match(settings, /draft\.translationApiUrl/);
assert.match(settings, /draft\.translationApiKey/);
assert.match(settings, /draft\.translationModel/);
assert.doesNotMatch(settings, /v-model="draft\.translationProxy"/);

assert.match(page, /data-translate-page/);
assert.match(page, /data-translate-input/);
assert.match(page, /data-translate-target-lang/);
assert.match(page, /data-translate-submit/);
assert.match(page, /data-translate-result/);
assert.match(page, /data-translate-copy/);
assert.match(page, /translateText/);

assert.match(models, /pub enum TranslationEngine/);
assert.match(models, /pub struct TranslateResponse/);
assert.match(models, /pub translation_engine: TranslationEngine/);
assert.match(models, /pub translation_api_url: String/);
assert.match(models, /pub translation_api_key: String/);
assert.match(models, /pub translation_model: String/);
assert.match(models, /pub translation_proxy: String/);

assert.match(commands, /pub async fn translate_text/);
assert.match(commands, /translator::translate_text_with_config/);
assert.match(libRs, /mod translator;/);
assert.match(libRs, /commands::translate_text/);
assert.match(cargoToml, /reqwest = \{ version = "0\.12", features = \["json"\] \}/);
