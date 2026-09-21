import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const app = readFileSync("src/App.vue", "utf8");
assert.match(app, /useUpdaterStore/);
assert.match(app, /void updater\.checkForUpdate\(true\)\.then/);
assert.match(app, /if \(updater\.phase === "available"\) startupUpdate\.value = updater\.version/);
assert.match(app, /if \(isMediaPreviewRoute\.value\) \{[\s\S]*?return;/);
assert.match(app, /data-update-startup-dialog/);
assert.match(app, /data-update-open-button/);
assert.match(app, /data-update-dismiss-button/);
assert.match(app, /await router\.push\("\/about"\)/);
assert.doesNotMatch(app, /openExternalUrl/);
assert.match(app, /可在软件内下载并安装更新/);
assert.match(app, /稍后/);
