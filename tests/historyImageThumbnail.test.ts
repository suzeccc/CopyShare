import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const tauriApi = readFileSync("src/lib/tauri.ts", "utf8");
const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");
const copyButton = readFileSync("src/components/ui/CopyTextButton.vue", "utf8");

assert.match(tauriApi, /getHistoryImageThumbnail/);
assert.match(tauriApi, /invoke<string>\("get_history_image_thumbnail"/);

assert.equal(existsSync("src/components/history/HistoryImageThumb.vue"), true);
assert.match(clipboardPage, /HistoryImageThumb/);
assert.match(clipboardPage, /v-if="item\.contentType === 'image'"/);
assert.match(floatingPanel, /HistoryImageThumb/);
assert.match(floatingPanel, /v-if="item\.contentType === 'image'"/);

assert.match(copyButton, /copyHistoryItem\(props\.historyItemId\)/);
