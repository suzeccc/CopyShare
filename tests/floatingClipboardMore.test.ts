import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");

assert.match(floatingPanel, /showClipboardHistoryModal/);
assert.match(floatingPanel, /data-floating-more-clipboard-button/);
assert.match(floatingPanel, /aria-label="查看更多剪贴板内容"/);
assert.match(floatingPanel, /MoreHorizontal/);
assert.doesNotMatch(floatingPanel, />更多</);
assert.match(floatingPanel, /v-if="showClipboardHistoryModal"/);
assert.match(floatingPanel, /v-for="item in clipboardHistoryItems"/);
assert.match(floatingPanel, /data-floating-clipboard-modal/);
assert.match(floatingPanel, /data-floating-clipboard-history-text/);
