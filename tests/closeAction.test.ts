import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const configTypes = readFileSync("src/types/config.ts", "utf8");
const configStore = readFileSync("src/stores/config.ts", "utf8");
const titleBar = readFileSync("src/components/layout/WindowTitleBar.vue", "utf8");
const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const tauriApi = readFileSync("src/lib/tauri.ts", "utf8");
const settings = readFileSync("src/pages/Settings.vue", "utf8");
const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
const lib = readFileSync("src-tauri/src/lib.rs", "utf8");
const models = readFileSync("src-tauri/src/models.rs", "utf8");

assert.match(configTypes, /export type CloseAction = "ask" \| "minimize" \| "exit"/);
assert.match(configTypes, /closeAction: CloseAction/);
assert.match(configStore, /closeAction: "ask"/);

assert.match(titleBar, /defineEmits/);
assert.match(titleBar, /emit\("close"\)/);
assert.doesNotMatch(titleBar, /useConfigStore/);
assert.doesNotMatch(titleBar, /@click="closeWindow\(\)"/);

assert.match(appShell, /onMainWindowCloseRequested/);
assert.match(appShell, /event\.preventDefault\(\)/);
assert.match(appShell, /handleCloseWindow/);
assert.match(appShell, /saveCloseActionPreference/);
assert.match(appShell, /hideMainWindow/);
assert.match(appShell, /exitApp/);
assert.match(appShell, /data-close-action-dialog/);
assert.match(appShell, /data-close-action-minimize/);
assert.match(appShell, /data-close-action-exit/);
assert.match(appShell, /data-close-action-remember/);
assert.ok((appShell.match(/@close="handleCloseWindow"/g) ?? []).length >= 2);

assert.match(tauriApi, /export function onMainWindowCloseRequested/);
assert.match(tauriApi, /export function exitApp/);
assert.match(commands, /pub fn exit_app/);
assert.match(lib, /commands::exit_app/);

assert.match(settings, /closeActionOptions/);
assert.match(settings, /data-close-action-setting/);

assert.match(models, /pub enum CloseAction/);
assert.match(models, /Ask/);
assert.match(models, /Minimize/);
assert.match(models, /Exit/);
assert.match(models, /pub close_action: CloseAction/);
