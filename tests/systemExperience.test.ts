import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const cargoToml = readFileSync("src-tauri/Cargo.toml", "utf8");
const libRs = readFileSync("src-tauri/src/lib.rs", "utf8");
const notificationsRs = readFileSync("src-tauri/src/notifications.rs", "utf8");
const discoveryRs = readFileSync("src-tauri/src/discovery.rs", "utf8");
const fileTransferRs = readFileSync("src-tauri/src/file_transfer.rs", "utf8");
const trayRs = readFileSync("src-tauri/src/tray.rs", "utf8");
const tauriTs = readFileSync("src/lib/tauri.ts", "utf8");
const appVue = readFileSync("src/App.vue", "utf8");

assert.match(cargoToml, /tauri-plugin-notification/);
assert.match(cargoToml, /notify-rust/);
assert.match(libRs, /tauri_plugin_notification::init/);

assert.match(notificationsRs, /notify_device_discovered/);
assert.match(notificationsRs, /notify_device_offline/);
assert.match(notificationsRs, /notify_file_transfer_completed/);
assert.match(notificationsRs, /notify_file_transfer_failed/);
assert.match(notificationsRs, /show_main_window/);
assert.match(notificationsRs, /navigate-to-page/);
assert.match(notificationsRs, /NotificationResponse::Default/);
assert.match(notificationsRs, /NotificationResponse::Action/);

assert.match(discoveryRs, /notify_device_online_once/);
assert.match(discoveryRs, /notify_device_offline/);
assert.match(fileTransferRs, /notify_file_transfer_completed/);
assert.match(fileTransferRs, /notify_file_transfer_failed/);

assert.match(trayRs, /TrayIconBuilder::with_id/);
assert.match(trayRs, /set_tooltip/);
assert.match(trayRs, /update_tray_status/);

assert.match(tauriTs, /navigate-to-page/);
assert.match(appVue, /onAppEvent<string>\("navigate-to-page"/);
assert.match(appVue, /router\.push\(route\)/);
