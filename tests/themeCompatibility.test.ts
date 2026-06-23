import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const button = readFileSync("src/components/ui/Button.vue", "utf8");
const copyTextButton = readFileSync("src/components/ui/CopyTextButton.vue", "utf8");
const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");
const home = readFileSync("src/pages/Home.vue", "utf8");
const devices = readFileSync("src/pages/Devices.vue", "utf8");
const refreshFeedback = readFileSync("src/lib/refreshFeedback.ts", "utf8");
const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");
const style = readFileSync("src/style.css", "utf8");

assert.match(style, /--accent-bg:/);
assert.match(style, /--accent-bg-hover:/);
assert.match(style, /--accent-line:/);
assert.match(style, /--accent-soft:/);
assert.match(style, /--accent-text:/);
assert.match(style, /--muted-text:/);
assert.match(style, /--subtle-text:/);
assert.match(style, /--button-primary-bg:/);
assert.match(style, /--floating-surface-bg:/);
assert.match(style, /--scrollbar-track:/);
assert.match(style, /html\[data-app-theme="win11Dark"\][\s\S]*--button-primary-bg:/);
assert.match(style, /html\[data-app-theme="win11Dark"\][\s\S]*--floating-surface-bg:/);

assert.match(button, /var\(--button-primary-bg\)/);
assert.match(button, /var\(--button-primary-bg-hover\)/);
assert.doesNotMatch(button, /border-blue-500|bg-blue-600|hover:bg-blue-500/);

assert.match(floatingPanel, /var\(--floating-control-bg\)/);
assert.match(floatingPanel, /var\(--floating-muted-text\)/);
assert.doesNotMatch(floatingPanel, /sky-|cyan-/);

assert.match(appShell, /var\(--dialog-bg\)/);
assert.match(appShell, /var\(--accent-soft\)/);
assert.doesNotMatch(appShell, /cyan-200|cyan-300|bg-slate-950/);

assert.match(copyTextButton, /var\(--floating-control-bg\)/);
assert.doesNotMatch(copyTextButton, /sky-/);

assert.doesNotMatch(home, /text-blue|border-blue|bg-blue|hover:border-sky|group-hover:bg-sky|group-hover:text-sky/);
assert.match(devices, /text-\[color:var\(--muted-text\)\]/);
assert.match(devices, /text-\[color:var\(--subtle-text\)\]/);
assert.doesNotMatch(devices, /text-slate-400|text-slate-500/);
assert.doesNotMatch(refreshFeedback, /sky-/);
assert.match(refreshFeedback, /var\(--accent-text\)/);
assert.doesNotMatch(sidebar, /text-blue/);
