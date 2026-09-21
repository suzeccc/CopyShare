import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { extname, join } from "node:path";

const chinese = JSON.parse(readFileSync("locales/zh-CN.json", "utf8")) as Record<string, string>;
const english = JSON.parse(readFileSync("locales/en-US.json", "utf8")) as Record<string, string>;
const traditional = JSON.parse(readFileSync("locales/zh-TW.json", "utf8")) as Record<string, string>;
const japanese = JSON.parse(readFileSync("locales/ja-JP.json", "utf8")) as Record<string, string>;
const configTypes = readFileSync("src/types/config.ts", "utf8");
const configStore = readFileSync("src/stores/config.ts", "utf8");
const main = readFileSync("src/main.ts", "utf8");
const settings = readFileSync("src/pages/Settings.vue", "utf8");
const rustModels = readFileSync("src-tauri/src/models.rs", "utf8");
const rustConfig = readFileSync("src-tauri/src/config.rs", "utf8");
const rustTray = readFileSync("src-tauri/src/tray.rs", "utf8");
const rustNotifications = readFileSync("src-tauri/src/notifications.rs", "utf8");

function sourceFiles(root: string): string[] {
  return readdirSync(root).flatMap((entry) => {
    const filePath = join(root, entry);
    return statSync(filePath).isDirectory() ? sourceFiles(filePath) : [filePath];
  });
}

const catalogSources = [
  ...sourceFiles("src").filter((file) => [".ts", ".vue"].includes(extname(file))),
  ...sourceFiles("src-tauri/src").filter((file) => extname(file) === ".rs"),
];
const missingPhrases = new Set<string>();
for (const file of catalogSources) {
  for (const phrase of readFileSync(file, "utf8").match(/[\u3400-\u9fff]+/gu) ?? []) {
    if (!(phrase in chinese)) missingPhrases.add(phrase);
  }
}

assert.deepEqual(Object.keys(english).sort(), Object.keys(chinese).sort());
assert.deepEqual(Object.keys(traditional).sort(), Object.keys(chinese).sort());
assert.deepEqual(Object.keys(japanese).sort(), Object.keys(chinese).sort());
assert.ok(Object.keys(english).length > 700);
assert.deepEqual([...missingPhrases], []);
assert.equal(english["设置"], "Settings");
assert.equal(english["显示窗口"], "Show window");
assert.equal(english["暂停同步"], "Pause sync");
assert.equal(english["退出"], "Quit");
assert.equal(english["简体中文"], "Simplified Chinese");

assert.match(configTypes, /export type UiLanguage = "system" \| "zh-CN" \| "zh-TW" \| "en-US" \| "ja-JP"/);
assert.match(configTypes, /uiLanguage: UiLanguage/);
assert.match(configStore, /configVersion: 12/);
assert.match(configStore, /uiLanguage: "system"/);
assert.match(main, /initializeI18n\(initialConfig\?\.uiLanguage \?\? "system"\)/);
assert.match(main, /onAppEvent\("config-updated"/);
assert.match(settings, /data-ui-language-setting/);
assert.match(settings, /<select[\s\S]*@change="saveUiLanguage\(/);
assert.match(settings, /\{ value: "zh-TW", label: "繁體中文" \}/);
assert.match(settings, /\{ value: "ja-JP", label: "日本語" \}/);

assert.match(rustModels, /pub enum UiLanguage/);
assert.match(rustModels, /#\[serde\(rename = "zh-CN"\)\]/);
assert.match(rustModels, /#\[serde\(rename = "zh-TW"\)\]/);
assert.match(rustModels, /#\[serde\(rename = "ja-JP"\)\]/);
assert.match(rustConfig, /CURRENT_CONFIG_VERSION: u16 = 12/);
assert.match(rustTray, /update_tray_locale/);
assert.match(rustTray, /i18n::translate/);
assert.match(rustNotifications, /\.title\(i18n::translate\(config, title\)\)/);
assert.match(rustNotifications, /i18n::translate_with_protected\(config, body, protected_body_parts\)/);
assert.match(rustNotifications, /\.body\(body\)/);
