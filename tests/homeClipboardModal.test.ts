import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const home = readFileSync("src/pages/Home.vue", "utf8");

assert.match(home, /ref\(false\)/);
assert.match(home, /showClipboardHistoryModal/);
assert.match(home, /allClipboardItems\s*=\s*computed/);
assert.match(home, /getRecentClipboardItems\(historyStore\.items,\s*historyStore\.items\.length\)/);

assert.match(home, /data-more-clipboard-button/);
assert.match(home, /@click="showClipboardHistoryModal = true"/);
assert.match(home, /data-clipboard-history-modal/);
assert.match(home, /v-if="showClipboardHistoryModal"/);
assert.match(home, /@click\.self="showClipboardHistoryModal = false"/);
assert.match(home, /@click="showClipboardHistoryModal = false"/);
assert.match(home, /v-for="item in allClipboardItems"/);
assert.match(home, /data-clipboard-history-row/);
assert.match(home, /grid-cols-\[minmax\(0,1fr\)_auto\]/);
assert.match(home, /data-clipboard-history-text/);
assert.match(home, /\bbreak-all\b/);
assert.match(home, /<CopyTextButton/);
assert.match(home, /:text="item\.text"/);
assert.match(home, /:content-type="item\.contentType"/);
assert.match(home, /:history-item-id="item\.id"/);
