import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");

assert.match(sidebar, /const primaryItems = \[/);
assert.match(sidebar, /const footerItems = \[/);
assert.match(sidebar, /data-sidebar-footer-nav/);
assert.match(sidebar, /w-48/);
assert.match(sidebar, /px-2 py-5/);
assert.match(sidebar, /gap-2\.5 rounded-md px-2 text-sm/);
assert.doesNotMatch(sidebar, /w-56/);
assert.doesNotMatch(sidebar, /w-60/);
assert.doesNotMatch(sidebar, /w-64/);
assert.match(sidebar, /path: "\/settings"/);
assert.match(sidebar, /path: "\/about"/);
assert.doesNotMatch(sidebar, /局域网直连/);
assert.doesNotMatch(sidebar, /发现附近设备，确认信任后即可同步/);
