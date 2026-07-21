import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const modelsRs = readFileSync("src-tauri/src/models.rs", "utf8");
const notificationsRs = readFileSync("src-tauri/src/notifications.rs", "utf8");
const syncRs = readFileSync("src-tauri/src/sync.rs", "utf8");
const mobileRs = readFileSync("src-tauri/src/mobile.rs", "utf8");
const fileTransferRs = readFileSync("src-tauri/src/file_transfer.rs", "utf8");
const settingsVue = readFileSync("src/pages/Settings.vue", "utf8");
const configTs = readFileSync("src/types/config.ts", "utf8");
const configStoreTs = readFileSync("src/stores/config.ts", "utf8");

for (const field of [
  "desktop_notifications",
  "notify_clipboard",
  "notify_trust_required",
  "notify_file_transfer",
  "notify_device_status",
  "notify_sync_error",
  "notification_clipboard_preview",
]) {
  assert.match(modelsRs, new RegExp(field));
}

for (const field of [
  "desktopNotifications",
  "notifyClipboard",
  "notifyTrustRequired",
  "notifyFileTransfer",
  "notifyDeviceStatus",
  "notifySyncError",
  "notificationClipboardPreview",
]) {
  assert.match(configTs, new RegExp(`${field}: boolean`));
  assert.match(configStoreTs, new RegExp(`${field}:`));
}

assert.match(notificationsRs, /notify_clipboard_received/);
assert.match(notificationsRs, /notify_mobile_clipboard_received/);
assert.match(notificationsRs, /notify_trust_required/);
assert.match(notificationsRs, /notify_file_transfer_offer/);
assert.match(notificationsRs, /notify_sync_error/);
assert.match(notificationsRs, /notification_clipboard_preview/);
assert.match(notificationsRs, /NOTIFICATION_COOLDOWNS/);
assert.match(notificationsRs, /navigate-to-page/);
assert.match(notificationsRs, /tauri_plugin_notification::NotificationExt/);
assert.match(notificationsRs, /\.notification\(\)\s*\.builder\(\)/);
assert.doesNotMatch(notificationsRs, /notify_rust::Notification::new/);

assert.match(syncRs, /notify_clipboard_received/);
assert.match(syncRs, /notify_trust_required/);
assert.match(syncRs, /emit_sync_error/);
assert.match(mobileRs, /notify_mobile_clipboard_received/);
assert.match(fileTransferRs, /notify_file_transfer_offer/);

const commandsRs = readFileSync("src-tauri/src/commands.rs", "utf8");
const libRs = readFileSync("src-tauri/src/lib.rs", "utf8");
const tauriTs = readFileSync("src/lib/tauri.ts", "utf8");

assert.match(commandsRs, /send_test_notification/);
assert.match(
  commandsRs,
  /send_test_notification[\s\S]*notifications::notify_test\(&app\)\.map_err\(AppError::Tauri\)/,
);
assert.match(notificationsRs, /pub fn notify_test\(app: &AppHandle\) -> Result<\(\), String>/);
assert.match(notificationsRs, /fn notification_result/);
assert.doesNotMatch(
  commandsRs,
  /notifications::notify_test\(&app\);\s*Ok\(\(\)\)/,
);
assert.match(libRs, /commands::send_test_notification/);
assert.match(tauriTs, /sendTestNotification/);
assert.match(settingsVue, /data-desktop-notification-settings/);
assert.match(settingsVue, /sendTestNotification/);
assert.match(settingsVue, /发送测试通知/);
assert.match(settingsVue, /桌面通知/);
assert.match(settingsVue, /saveNotificationSetting/);
assert.match(settingsVue, /saveDesktopNotifications/);
assert.match(settingsVue, /saveNotificationClipboardPreview/);
assert.match(settingsVue, /通知中显示剪贴板预览/);
assert.doesNotMatch(settingsVue, /默认开启/);
assert.match(configStoreTs, /notificationClipboardPreview:\s*true/);
assert.match(settingsVue, /设备上线\/离线提醒/);
assert.match(settingsVue, /发现设备上线或离线时提醒/);
assert.match(configStoreTs, /notifyDeviceStatus:\s*true/);
