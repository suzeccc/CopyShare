import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const configTypes = readFileSync("src/types/config.ts", "utf8");
const configStore = readFileSync("src/stores/config.ts", "utf8");
const titleBar = readFileSync("src/components/layout/WindowTitleBar.vue", "utf8");
const settings = readFileSync("src/pages/Settings.vue", "utf8");
const models = readFileSync("src-tauri/src/models.rs", "utf8");

assert.match(configTypes, /export type CloseAction = "ask" \| "minimize" \| "exit"/);
assert.match(configTypes, /closeAction: CloseAction/);
assert.match(configStore, /closeAction: "ask"/);

assert.match(titleBar, /useConfigStore/);
assert.match(titleBar, /handleCloseWindow/);
assert.match(titleBar, /saveCloseActionPreference/);
assert.match(titleBar, /hideMainWindow/);
assert.match(titleBar, /data-close-action-dialog/);
assert.match(titleBar, /data-close-action-minimize/);
assert.match(titleBar, /data-close-action-exit/);
assert.match(titleBar, /data-close-action-remember/);
assert.doesNotMatch(titleBar, /@click="closeWindow\(\)"/);

assert.match(settings, /closeActionOptions/);
assert.match(settings, /data-close-action-setting/);

assert.match(models, /pub enum CloseAction/);
assert.match(models, /Ask/);
assert.match(models, /Minimize/);
assert.match(models, /Exit/);
assert.match(models, /pub close_action: CloseAction/);
