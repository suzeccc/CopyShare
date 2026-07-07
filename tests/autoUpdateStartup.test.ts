import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const app = readFileSync("src/App.vue", "utf8");
const aboutMeta = readFileSync("src/lib/about.ts", "utf8");

assert.match(aboutMeta, /checkForAppUpdateOnStartup/);
assert.match(aboutMeta, /getStartupUpdatePrompt/);
assert.match(aboutMeta, /getLatestRelease/);
assert.match(aboutMeta, /catch\s*\{\s*\}/);

assert.match(app, /import \{ checkForAppUpdateOnStartup \} from "@\/lib\/about"/);
assert.match(
  app,
  /void checkForAppUpdateOnStartup\(\(update\) => \{\s*startupUpdate\.value = update;\s*\}\)/,
);
assert.doesNotMatch(app, /toastStore\.info/);
assert.match(app, /data-update-startup-dialog/);
assert.match(app, /data-update-open-release-button/);
assert.match(app, /data-update-dismiss-button/);
assert.match(app, /openStartupUpdateRelease/);
assert.match(app, /发现新版本/);
assert.match(app, /立即查看/);
assert.match(app, /稍后/);
