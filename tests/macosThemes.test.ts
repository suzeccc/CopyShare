import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const settings = readFileSync("src/pages/Settings.vue", "utf8");
const style = readFileSync("src/style.css", "utf8");
const types = readFileSync("src/types/config.ts", "utf8");
const models = readFileSync("src-tauri/src/models.rs", "utf8");

assert.match(types, /"macosLight"/);
assert.match(types, /"macosDark"/);

assert.match(settings, /value:\s*"copyBlue",\s*label:\s*"\u6e05\u96c5\u8336\u7eff"/);
assert.match(settings, /value:\s*"macosLight",\s*label:\s*"\u77f3\u58a8\u767d\u96fe"/);
assert.match(settings, /value:\s*"macosDark",\s*label:\s*"\u5348\u591c\u73bb\u7483"/);
assert.match(settings, /value:\s*"win11Dark"[\s\S]*value:\s*"macosDark"[\s\S]*value:\s*"macosLight"[\s\S]*value:\s*"copyBlue"/);
assert.doesNotMatch(settings, /label:\s*"macOS /);
assert.doesNotMatch(settings, /label:\s*"\u8336\u8bdd\u7eff"/);
assert.doesNotMatch(settings, /label:\s*"\u77f3\u58a8\u96fe"/);

assert.match(style, /html\[data-app-theme="macosLight"\]/);
assert.match(style, /html\[data-app-theme="macosLight"\][\s\S]*--main-bg:\s*#f5f5f7;/);
assert.match(style, /html\[data-app-theme="macosLight"\][\s\S]*--theme-accent:\s*#0a84ff;/);
assert.match(style, /html\[data-app-theme="macosLight"\][\s\S]*--clipboard-card-bg:\s*rgba\(255,\s*255,\s*255,\s*0\.78\);/);
assert.match(style, /html\[data-app-theme="macosLight"\][\s\S]*--clipboard-card-text:\s*#1d1d1f;/);
assert.match(style, /html\[data-app-theme="macosLight"\]\s*\.text-white/);
assert.match(style, /html\[data-app-theme="macosLight"\]\s*\.text-slate-300/);
assert.match(style, /html\[data-app-theme="macosLight"\]\s*\.clipboard-category-chip:hover/);
assert.match(style, /html\[data-app-theme="macosDark"\]/);
assert.match(style, /html\[data-app-theme="macosDark"\][\s\S]*--main-bg:\s*#1c1c1e;/);
assert.match(style, /html\[data-app-theme="macosDark"\][\s\S]*--theme-accent:\s*#0a84ff;/);

assert.match(models, /MacosLight/);
assert.match(models, /MacosDark/);
