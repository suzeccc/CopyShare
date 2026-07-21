import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const packageJson = JSON.parse(readFileSync("package.json", "utf8"));
const cargo = readFileSync("src-tauri/Cargo.toml", "utf8");
const libRs = readFileSync("src-tauri/src/lib.rs", "utf8");
const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
const models = readFileSync("src-tauri/src/models.rs", "utf8");
const config = readFileSync("src-tauri/src/config.rs", "utf8");
const capability = JSON.parse(readFileSync("src-tauri/capabilities/default.json", "utf8"));
const configType = readFileSync("src/types/config.ts", "utf8");
const configStore = readFileSync("src/stores/config.ts", "utf8");
const shortcutStore = readFileSync("src/stores/shortcuts.ts", "utf8");
const app = readFileSync("src/App.vue", "utf8");
const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const settings = readFileSync("src/pages/Settings.vue", "utf8");
const shortcutDialog = readFileSync("src/components/settings/ShortcutSettingsDialog.vue", "utf8");
const floatingHistory = readFileSync("src/pages/FloatingClipboardHistory.vue", "utf8");
const tauri = readFileSync("src/lib/tauri.ts", "utf8");

assert.match(packageJson.dependencies["@tauri-apps/plugin-global-shortcut"], /^\^2\./);
assert.match(cargo, /tauri-plugin-global-shortcut\s*=\s*"2"/);
assert.match(libRs, /tauri_plugin_global_shortcut::Builder::new\(\)\.build\(\)/);
assert.ok(capability.permissions.includes("global-shortcut:allow-register"));
assert.ok(capability.permissions.includes("global-shortcut:allow-unregister"));

for (const source of [models, configType, configStore]) {
  for (const field of ["quickPanel", "ocr", "translate", "snippets", "toggleSync"]) {
    assert.match(source, new RegExp(`${field}ShortcutEnabled|${field.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`)}_shortcut_enabled`));
    assert.match(source, new RegExp(`${field}Shortcut|${field.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`)}_shortcut`));
  }
}
assert.match(config, /CURRENT_CONFIG_VERSION:\s*u16\s*=\s*8/);
assert.match(configStore, /configVersion:\s*8/);

assert.match(shortcutStore, /GlobalShortcutController/);
assert.match(shortcutStore, /global-shortcut-triggered/);
assert.match(app, /shortcutStore\.apply\(configStore\.config\)/);
assert.match(app, /shortcutStore\.dispose\(\)/);

assert.match(settings, /data-global-shortcut-settings/);
assert.match(settings, /data-shortcut-settings-entry/);
assert.match(settings, /ShortcutSettingsDialog/);
assert.match(shortcutDialog, /role="dialog"/);
assert.match(shortcutDialog, /aria-modal="true"/);
assert.match(shortcutDialog, /SHORTCUT_DEFINITIONS/);
assert.match(shortcutDialog, /startShortcutRecording/);
assert.match(shortcutDialog, /restoreAllDefaults/);
assert.match(shortcutDialog, /data-shortcut-action/);

assert.match(appShell, /"global-shortcut-triggered"/);
assert.match(appShell, /recognizeClipboardImage/);
assert.match(appShell, /readClipboardText/);
assert.match(appShell, /translateText/);
assert.match(appShell, /activeView\s*=\s*"snippets"/);
assert.match(appShell, /statusStore\.status\.running/);
assert.match(commands, /pub fn read_clipboard_text/);
assert.match(libRs, /commands::read_clipboard_text/);
assert.match(tauri, /export function readClipboardText/);

assert.match(tauri, /export async function toggleFloatingClipboardHistoryWindow/);
assert.match(floatingHistory, /activeClipboardIndex/);
assert.match(floatingHistory, /handleQuickPanelKeydown/);
assert.match(floatingHistory, /event\.key === "ArrowDown"/);
assert.match(floatingHistory, /event\.key === "ArrowUp"/);
assert.match(floatingHistory, /event\.key === "Enter"/);
assert.match(floatingHistory, /event\.key === "Escape"/);
