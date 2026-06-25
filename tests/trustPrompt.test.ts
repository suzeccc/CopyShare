import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");

assert.match(appShell, /useDevicesStore/);
assert.match(appShell, /namedTrustDevices/);
assert.match(appShell, /trustPromptDevice\s*=\s*computed/);
assert.match(appShell, /namedTrustDevices\(devicesStore\.pendingTrust\)/);
assert.doesNotMatch(appShell, /devicesStore\.pendingTrust\[0\]/);
assert.match(appShell, /data-trust-prompt/);
assert.match(appShell, /devicesStore\.trust\(device\.id\)/);
assert.match(appShell, /devicesStore\.reject\(device\.id\)/);
assert.match(appShell, /v-if="!isFloating && trustPromptDevice"/);
