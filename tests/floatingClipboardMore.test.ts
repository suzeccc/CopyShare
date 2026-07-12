import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");
const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");

assert.match(floatingPanel, /showClipboardHistoryModal/);
assert.match(floatingPanel, /data-floating-more-clipboard-button/);
assert.match(floatingPanel, /aria-label="查看更多剪贴板内容"/);
assert.match(floatingPanel, /MoreHorizontal/);
assert.doesNotMatch(floatingPanel, />更多</);
assert.match(floatingPanel, /v-if="showClipboardHistoryModal"/);
assert.match(floatingPanel, /v-for="item in clipboardHistoryItems"/);
assert.match(floatingPanel, /data-floating-clipboard-modal/);
assert.match(floatingPanel, /data-floating-clipboard-history-text/);
assert.match(appShell, /const clipboardItems = computed\(\(\) =>\s*getFloatingClipboardItems\(systemClipboardItems\.value, historyStore\.items\),\s*\)/);
assert.match(appShell, /const clipboardHistoryItems = computed\(\(\) =>\s*getFloatingClipboardItems\(\s*systemClipboardItems\.value,\s*historyStore\.items,\s*Math\.max\(systemClipboardItems\.value\.length \+ historyStore\.items\.length, 1\),\s*\),\s*\)/);
