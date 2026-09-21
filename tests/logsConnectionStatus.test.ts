import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const logsPage = readFileSync("src/pages/Logs.vue", "utf8");
const activityItem = readFileSync("src/components/activity/ActivityLogItem.vue", "utf8");
const activityStore = readFileSync("src/stores/activityLog.ts", "utf8");
const app = readFileSync("src/App.vue", "utf8");
const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");

assert.match(logsPage, />日志</);
assert.match(logsPage, /不保存或展示剪贴板正文/);
assert.match(logsPage, /data-activity-log-filters/);
assert.match(logsPage, /仅异常/);
assert.match(logsPage, /设备事件/);
assert.match(logsPage, /文件传输/);
assert.match(logsPage, /activityLogStore\.clear\(\)/);
assert.doesNotMatch(logsPage, /useHistoryStore/);
assert.doesNotMatch(logsPage, /HistoryItem/);
assert.doesNotMatch(logsPage, /CopyTextButton/);

assert.match(activityItem, /data-activity-log-item/);
assert.match(activityItem, /继续传输/);
assert.doesNotMatch(activityItem, /item\.summary/);
assert.doesNotMatch(activityItem, /item\.content/);

assert.match(activityStore, /onAppEvent<DeviceInfo>\("device-connected"/);
assert.match(activityStore, /onAppEvent<DeviceInfo>\("device-disconnected"/);
assert.match(activityStore, /onAppEvent<string>\("sync-error"/);
assert.match(app, /activityLogStore\.initialize\(historyStore\.items, statusStore\.status\)/);
assert.match(app, /activityLogStore\.subscribe\(\)/);
assert.match(sidebar, /label: "日志", path: "\/logs"/);
