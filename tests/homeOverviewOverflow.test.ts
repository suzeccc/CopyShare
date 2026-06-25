import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const card = readFileSync("src/components/ui/Card.vue", "utf8");
const home = readFileSync("src/pages/Home.vue", "utf8");

assert.match(card, /min-w-0/);
assert.match(card, /max-w-full/);

assert.match(home, /data-home-overview/);
assert.match(home, /overflow-x-hidden/);
assert.match(home, /data-home-stats-grid/);
assert.match(home, /grid-cols-1/);
assert.match(home, /data-home-quick-device-row/);
assert.match(home, /md:grid-cols-\[minmax\(0,1fr\)_minmax\(0,1fr\)\]/);
assert.match(home, /data-home-quick-actions/);
assert.match(home, /data-home-sync-content-grid/);
assert.match(home, /data-home-recent-row/);
assert.match(home, /break-all/);
assert.doesNotMatch(home, /overflow-auto/);
