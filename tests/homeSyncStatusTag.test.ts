import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const home = readFileSync("src/pages/Home.vue", "utf8");

assert.match(home, /data-home-recent-sync-status/);
assert.match(home, /data-clipboard-history-sync-status/);
assert.match(home, /item\.syncStatus === 'synced'/);
assert.match(home, /已同步/);
assert.match(home, /未同步/);
assert.match(home, /data-home-recent-row[\s\S]*grid-cols-\[minmax\(0,1fr\)_auto\]/);
assert.match(home, /data-home-recent-actions[\s\S]*data-home-recent-sync-status[\s\S]*CopyTextButton[\s\S]*data-home-recent-device/);
assert.match(home, /data-clipboard-history-row[\s\S]*grid-cols-\[minmax\(0,1fr\)_auto\]/);
assert.match(home, /data-clipboard-history-actions[\s\S]*data-clipboard-history-sync-status[\s\S]*CopyTextButton[\s\S]*data-clipboard-history-device/);
