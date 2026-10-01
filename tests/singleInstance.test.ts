import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const cargo = readFileSync("src-tauri/Cargo.toml", "utf8");
const lib = readFileSync("src-tauri/src/lib.rs", "utf8");
const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
const notifications = readFileSync("src-tauri/src/notifications.rs", "utf8");

assert.match(cargo, /tauri-plugin-single-instance\s*=\s*"2"/);

assert.match(lib, /let mut builder = tauri::Builder::default\(\);/);
assert.match(
  lib,
  /builder = builder\.plugin\(tauri_plugin_single_instance::init\(\|app, _args, _cwd\| \{\s*notifications::show_main_window\(app\);\s*\}\)\);/,
);
assert.match(notifications, /crate::commands::show_main_window\(app\)\.await/);
assert.match(commands, /FLOATING_BALL_LOW_MEMORY\.store\(low, Ordering::Relaxed\)/);
const show = commands.match(/pub async fn show_main_window\([\s\S]*?\n\}/)?.[0] ?? "";
const hide = commands.match(/pub async fn hide_main_window\([\s\S]*?\n\}/)?.[0] ?? "";
assert.ok(show.indexOf("set_webview_memory_target") < show.indexOf("window.show()?"));
assert.match(hide, /window\.hide\(\)\?;[\s\S]*if low_memory \{[\s\S]*set_webview_memory_target\(window, true\)/);
const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
assert.match(appShell, /if \(hideNativeWindow\) \{\s*await hideMainWindow\(false\)/);

assert.ok(
  lib.indexOf("tauri_plugin_single_instance::init") <
    lib.indexOf("tauri_plugin_clipboard_manager::init"),
);
