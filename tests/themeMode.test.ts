import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const settings = readFileSync("src/pages/Settings.vue", "utf8");
const configStore = readFileSync("src/stores/config.ts", "utf8");
const style = readFileSync("src/style.css", "utf8");
const types = readFileSync("src/types/config.ts", "utf8");

assert.match(types, /type AppTheme\s*=\s*"copyBlue"\s*\|\s*"win11Dark"/);
assert.match(types, /theme:\s*AppTheme/);
assert.match(appShell, /\(\)\s*=>\s*configStore\.config\.theme/);
assert.match(appShell, /dataset\.appTheme\s*=\s*theme/);
assert.match(settings, /\(\)\s*=>\s*draft\.theme/);
assert.match(settings, /dataset\.appTheme\s*=\s*theme/);
assert.match(settings, /onBeforeUnmount/);
assert.match(settings, /configStore\.config\.theme/);
assert.match(settings, /Win11 深色/);
assert.match(settings, /themeOptions[\s\S]*value:\s*"win11Dark"[\s\S]*value:\s*"copyBlue"/);
assert.match(configStore, /theme:\s*"win11Dark"/);
assert.match(style, /html\[data-app-theme="win11Dark"\]/);
assert.match(style, /--main-bg:\s*#202020;/);
