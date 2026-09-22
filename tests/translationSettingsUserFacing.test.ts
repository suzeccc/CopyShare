import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const settings = readFileSync("src/pages/Settings.vue", "utf8");

test("Google translation is presented as ready without configuration", () => {
  assert.match(settings, /data-translation-engine-picker/);
  assert.match(settings, /label: "Google 翻译"/);
  assert.match(settings, /hint: "免费 · 无需配置"/);
  assert.match(settings, /v-if="draft\.translationEngine === 'google'"/);
  assert.match(settings, /data-translation-google-ready/);
  assert.match(settings, /无需 API Key 或额外设置/);
  assert.doesNotMatch(settings, /v-model="draft\.translationProxy"/);
  assert.doesNotMatch(settings, />Google 代理</);
});

test("AI configuration is only rendered for the AI engine", () => {
  assert.match(settings, /label: "AI 翻译"/);
  assert.match(settings, /hint: "使用自有 API"/);
  assert.match(settings, /v-else\s+data-translation-ai-settings/);

  const aiSettingsStart = settings.indexOf("data-translation-ai-settings");
  assert.notEqual(aiSettingsStart, -1);
  assert.equal(settings.indexOf('v-model="draft.translationApiUrl"') > aiSettingsStart, true);
  assert.equal(settings.indexOf('v-model="draft.translationApiKey"') > aiSettingsStart, true);
  assert.equal(settings.indexOf('v-model="draft.translationModel"') > aiSettingsStart, true);
});

test("translation settings stack controls in narrow windows", () => {
  const translationStart = settings.indexOf("<section data-translation-settings");
  const storageStart = settings.indexOf("<section class=\"grid gap-2\">", translationStart + 1);
  const translationSection = settings.slice(translationStart, storageStart);

  assert.match(translationSection, /lg:flex-row/);
  assert.doesNotMatch(translationSection, /sm:flex-row/);
});

test("translation engine picker gives each service a clear responsive choice", () => {
  const pickerStart = settings.indexOf("data-translation-engine-picker");
  const pickerEnd = settings.indexOf("</div>", pickerStart);
  const picker = settings.slice(pickerStart, pickerEnd);

  assert.match(picker, /role="group"/);
  assert.match(picker, /class="translation-engine-picker"/);
  assert.match(picker, /class="translation-engine-option"/);
  assert.match(picker, /:aria-pressed="draft\.translationEngine === option\.value"/);
  assert.match(settings, /\.translation-engine-picker \{[^}]*grid-template-columns: repeat\(2, minmax\(0, 1fr\)\)/);
  assert.match(settings, /@media \(max-width: 560px\) \{\s*\.translation-engine-picker \{ grid-template-columns: 1fr; \}/);
  assert.doesNotMatch(settings, /\.translation-engine-option\.is-selected::before/);
  assert.match(settings, /\.translation-engine-option:focus-visible/);
  assert.match(settings, /background: linear-gradient\(105deg, var\(--accent-soft\), var\(--field-bg\) 70%\)/);
});

test("translation setting interactions do not show success toasts for unchanged values", () => {
  const saveTranslationSettingStart = settings.indexOf("async function saveTranslationSetting");
  const saveTranslationEngineStart = settings.indexOf("async function saveTranslationEngine");
  const saveTranslationSetting = settings.slice(saveTranslationSettingStart, saveTranslationEngineStart);

  assert.match(saveTranslationSetting, /saveBasicSettings\(normalizedPatch\);/);
  assert.match(settings, /if \(translationApiUrl === configStore\.config\.translationApiUrl\) return;/);
  assert.match(settings, /if \(translationApiKey === configStore\.config\.translationApiKey\) return;/);
  assert.match(settings, /if \(translationModel === configStore\.config\.translationModel\) return;/);
});

test("sync direction is grouped with sync content and shortcuts follow translation", () => {
  assert.doesNotMatch(settings, /data-history-settings|保存同步记录/);
  assert.doesNotMatch(settings, /保存同步摘要/);
  assert.doesNotMatch(settings, /只保存摘要，不保存完整敏感剪贴板内容/);

  const syncStart = settings.indexOf(">同步内容<");
  const translationStart = settings.indexOf("<section data-translation-settings");
  const shortcutStart = settings.indexOf("<section data-global-shortcut-settings");
  const syncSection = settings.slice(syncStart, translationStart);
  assert.equal(syncStart >= 0, true);
  assert.equal(translationStart >= 0, true);
  assert.equal(shortcutStart >= 0, true);
  assert.match(syncSection, /data-sync-direction-setting/);
  assert.doesNotMatch(settings, /<p class="text-\[13px\] font-bold text-\[color:var\(--subtle-text\)\]">历史记录<\/p>/);
  assert.equal(translationStart < shortcutStart, true);
});
