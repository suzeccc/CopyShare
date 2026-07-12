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

test("translation engine picker uses a polished segmented control style", () => {
  const pickerStart = settings.indexOf("data-translation-engine-picker");
  const pickerEnd = settings.indexOf("</div>", pickerStart);
  const picker = settings.slice(pickerStart, pickerEnd);

  assert.match(picker, /p-1\.5/);
  assert.match(picker, /gap-1\.5/);
  assert.match(picker, /rounded-\[14px\]/);
  assert.match(picker, /shadow-\[inset_0_1px_0_rgba\(255,255,255,0\.04\)\]/);
  assert.match(settings, /bg-\[linear-gradient\(135deg,rgba\(79,167,203,0\.24\),rgba\(79,167,203,0\.10\)\)\]/);
  assert.match(settings, /shadow-\[0_10px_26px_rgba\(0,0,0,0\.24\),inset_0_1px_0_rgba\(255,255,255,0\.08\)\]/);
  assert.match(settings, /hover:bg-\[rgba\(255,255,255,0\.035\)\]/);
  assert.doesNotMatch(picker, /divide-x/);
});

test("translation setting interactions do not show success toasts for unchanged values", () => {
  const saveTranslationSettingStart = settings.indexOf("async function saveTranslationSetting");
  const saveTranslationEngineStart = settings.indexOf("async function saveTranslationEngine");
  const saveTranslationSetting = settings.slice(saveTranslationSettingStart, saveTranslationEngineStart);

  assert.match(saveTranslationSetting, /saveBasicSettings\([\s\S]*\{ silent: true \}/);
  assert.match(settings, /if \(translationApiUrl === configStore\.config\.translationApiUrl\) return;/);
  assert.match(settings, /if \(translationApiKey === configStore\.config\.translationApiKey\) return;/);
  assert.match(settings, /if \(translationModel === configStore\.config\.translationModel\) return;/);
});

test("translation settings are placed below clear history settings copy", () => {
  assert.match(settings, />保存同步历史</);
  assert.match(settings, /保存剪贴板同步记录，关闭后不再记录新的同步历史/);
  assert.match(settings, /label="保存同步历史"/);
  assert.doesNotMatch(settings, /保存同步摘要/);
  assert.doesNotMatch(settings, /只保存摘要，不保存完整敏感剪贴板内容/);

  const historyStart = settings.indexOf(">历史记录<");
  const translationStart = settings.indexOf("<section data-translation-settings");
  assert.equal(historyStart >= 0, true);
  assert.equal(translationStart >= 0, true);
  assert.equal(historyStart < translationStart, true);
});
