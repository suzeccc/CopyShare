import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const app = readFileSync("src/App.vue", "utf8");
const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const router = readFileSync("src/router/index.ts", "utf8");
const settings = readFileSync("src/pages/Settings.vue", "utf8");
const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");

assert.doesNotMatch(sidebar, /FileUp/);
assert.doesNotMatch(sidebar, /path:\s*"\/files"/);
assert.doesNotMatch(router, /FileTransfer/);
assert.doesNotMatch(router, /path:\s*"\/files"/);
assert.doesNotMatch(app, /useFileTransferStore/);
assert.doesNotMatch(app, /fileTransferStore/);
assert.doesNotMatch(appShell, /FileTransferOfferDialog/);

assert.match(settings, /draft\.syncFiles/);
assert.match(settings, /saveSyncFiles/);
assert.doesNotMatch(settings, /syncFiles:\s*false/);
assert.doesNotMatch(settings, /data-transfer-save-dir-setting/);
assert.match(settings, /data-download-location-setting/);
assert.doesNotMatch(settings, /notifyFileTransfer/);

assert.equal(existsSync("src/pages/FileTransfer.vue"), false);
assert.equal(existsSync("src/stores/fileTransfer.ts"), false);
assert.equal(existsSync("src/components/fileTransfer/FileTransferOfferDialog.vue"), false);
