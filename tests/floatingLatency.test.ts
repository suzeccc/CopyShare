import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { getLatencyLabel } from "../src/lib/windowMode.ts";

const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const statusType = readFileSync("src/types/status.ts", "utf8");
const statusStore = readFileSync("src/stores/status.ts", "utf8");

assert.equal(getLatencyLabel({ running: false, connectedCount: 0, latencyMs: null }), "-- ms");
assert.equal(getLatencyLabel({ running: true, connectedCount: 0, latencyMs: null }), "-- ms");
assert.equal(getLatencyLabel({ running: true, connectedCount: 1, latencyMs: null }), "检测中");
assert.equal(getLatencyLabel({ running: true, connectedCount: 1, latencyMs: 27 }), "27 ms");
assert.equal(getLatencyLabel({ running: true, connectedCount: 1, latencyMs: 0 }), "0 ms");

assert.match(statusType, /latencyMs:\s*number \| null/);
assert.match(statusStore, /latencyMs:\s*null/);
assert.match(appShell, /latencyMs:\s*statusStore\.status\.latencyMs/);
