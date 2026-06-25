import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");
const style = readFileSync("src/style.css", "utf8");

assert.match(appShell, /const clipboardHistoryItems = computed/);
assert.match(appShell, /:clipboard-history-items="clipboardHistoryItems"/);

assert.match(floatingPanel, /clipboardHistoryItems: ClipboardPreviewItem\[\]/);
assert.match(floatingPanel, /showClipboardHistoryModal/);
assert.match(floatingPanel, /data-floating-more-clipboard-button/);
assert.match(floatingPanel, /data-floating-clipboard-modal/);
assert.match(floatingPanel, /data-floating-clipboard-history-row/);
assert.match(floatingPanel, /data-floating-clipboard-text/);
assert.match(floatingPanel, /data-floating-clipboard-history-text/);
assert.match(floatingPanel, /var\(--floating-surface-bg\)/);
assert.match(floatingPanel, /var\(--floating-control-bg\)/);
assert.match(floatingPanel, /var\(--floating-control-line\)/);
assert.match(floatingPanel, /MoreHorizontal/);

assert.match(style, /\[data-floating-clipboard-text\]/);
assert.match(style, /\[data-floating-clipboard-history-text\]/);
assert.match(style, /\[data-floating-clipboard-text\][\s\S]*user-select:\s*text/);
assert.match(style, /\[data-floating-clipboard-text\][\s\S]*-webkit-user-select:\s*text/);