import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const style = readFileSync("src/style.css", "utf8");

assert.match(style, /--disconnect-notice-bg:/);
assert.match(style, /--disconnect-notice-line:/);
assert.match(style, /--disconnect-notice-ring:/);
assert.match(style, /--disconnect-notice-icon-bg:/);
assert.match(style, /--disconnect-notice-icon-text:/);
assert.match(style, /html\[data-app-theme="win11Dark"\][\s\S]*--disconnect-notice-bg:/);
assert.match(style, /html\[data-app-theme="win11Dark"\][\s\S]*--disconnect-notice-ring:/);
assert.match(style, /html\[data-app-theme="win11Dark"\][\s\S]*--disconnect-notice-bg: rgba\(43, 43, 43, 0\.98\);/);
assert.match(style, /html\[data-app-theme="win11Dark"\][\s\S]*--disconnect-notice-line: rgba\(96, 205, 255,/);
assert.match(style, /html\[data-app-theme="win11Dark"\][\s\S]*--disconnect-notice-icon-text: #dff6ff;/);
assert.doesNotMatch(style, /html\[data-app-theme="win11Dark"\][\s\S]*--disconnect-notice-line: rgba\(251, 146, 60,/);

assert.match(appShell, /var\(--disconnect-notice-bg\)/);
assert.match(appShell, /var\(--disconnect-notice-line\)/);
assert.match(appShell, /var\(--disconnect-notice-ring\)/);
assert.match(appShell, /var\(--disconnect-notice-icon-bg\)/);
assert.match(appShell, /var\(--disconnect-notice-icon-text\)/);
assert.match(appShell, /font-medium/);
assert.match(appShell, /right-12 top-14/);
assert.doesNotMatch(appShell, /right-3 top-11/);
assert.doesNotMatch(appShell, /border-sky|bg-sky|text-sky|rgba\(13,35,58/);
