import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const cargo = readFileSync("src-tauri/Cargo.toml", "utf8");
const lib = readFileSync("src-tauri/src/lib.rs", "utf8");

assert.match(cargo, /tauri-plugin-single-instance\s*=\s*"2"/);

assert.match(lib, /use tauri::Manager;/);
assert.match(lib, /let mut builder = tauri::Builder::default\(\);/);
assert.match(
  lib,
  /builder = builder\.plugin\(tauri_plugin_single_instance::init\(\|app, _args, _cwd\| \{[\s\S]*app\.get_webview_window\("main"\)[\s\S]*window\.show\(\)[\s\S]*window\.unminimize\(\)[\s\S]*window\.set_focus\(\)[\s\S]*\}\)\);/,
);

assert.ok(
  lib.indexOf("tauri_plugin_single_instance::init") <
    lib.indexOf("tauri_plugin_clipboard_manager::init"),
);
