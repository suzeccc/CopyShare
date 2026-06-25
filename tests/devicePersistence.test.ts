import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

assert.ok(existsSync("src-tauri/src/device_store.rs"));

const lib = readFileSync("src-tauri/src/lib.rs", "utf8");
const state = readFileSync("src-tauri/src/state.rs", "utf8");
const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
const sync = readFileSync("src-tauri/src/sync.rs", "utf8");
const deviceStore = readFileSync("src-tauri/src/device_store.rs", "utf8");

assert.match(lib, /mod device_store;/);
assert.match(state, /device_store::load_devices\(app\)\?/);
assert.match(state, /replace_devices/);
assert.match(deviceStore, /const DEVICES_FILE:\s*&str\s*=\s*"devices\.json"/);
assert.match(deviceStore, /pub fn load_devices/);
assert.match(deviceStore, /pub fn save_devices/);
assert.match(deviceStore, /load_device_items_from_text/);
assert.match(commands, /device_store::save_devices/);
assert.match(sync, /device_store::save_devices/);