import assert from "node:assert/strict";

import {
  activityFromFileTransfer,
  activityFromHistory,
  filterActivities,
  readPersistedActivityLog,
  seedActivitiesFromHistory,
} from "../src/lib/activityLog.ts";

const remoteText = activityFromHistory({
  id: "history-1",
  direction: "remote",
  sourceDevice: "Laptop",
  content: "private clipboard text",
  contentType: "text",
  syncStatus: "synced",
  success: true,
  createdAt: "2026-07-30T12:00:00Z",
});
assert.ok(remoteText);
assert.equal(remoteText.title, "已接收剪贴板");
assert.equal(remoteText.deviceName, "Laptop");
assert.equal(remoteText.sizeBytes, 22);
assert.doesNotMatch(JSON.stringify(remoteText), /private clipboard text/);

assert.equal(activityFromHistory({
  id: "offline-local",
  direction: "local",
  sourceDevice: "Desktop",
  content: "not sent",
  contentType: "text",
  syncStatus: "unsynced",
  success: true,
  createdAt: "2026-07-30T12:00:00Z",
}), null);

const fileActivity = activityFromFileTransfer({
  transferId: "transfer-1",
  direction: "receive",
  peerDeviceId: "device-a",
  peerDeviceName: "Laptop",
  clipboardSync: false,
  files: [{
    id: "file-1",
    name: "secret-name.zip",
    size: 4096,
    sha256: "hash",
    savedPath: null,
    transferredBytes: 2048,
    status: "failed",
    error: "connection reset",
  }],
  totalSize: 4096,
  transferredBytes: 2048,
  status: "failed",
  createdAt: "2026-07-30T12:00:00Z",
  completedAt: "2026-07-30T12:01:00Z",
  error: "connection reset",
});
assert.equal(fileActivity.title, "文件传输失败");
assert.equal(fileActivity.sizeBytes, 4096);
assert.doesNotMatch(JSON.stringify(fileActivity), /secret-name\.zip/);
assert.equal(filterActivities([remoteText, fileActivity], "issues").length, 1);
assert.equal(filterActivities([remoteText, fileActivity], "file")[0].id, "file:transfer-1");

assert.deepEqual(seedActivitiesFromHistory([]), []);
assert.equal(readPersistedActivityLog("not json"), null);
const sanitized = readPersistedActivityLog(JSON.stringify({
  version: 1,
  seededFromHistory: true,
  items: [{ ...remoteText, content: "must not survive persistence" }],
}));
assert.ok(sanitized);
assert.doesNotMatch(JSON.stringify(sanitized), /must not survive persistence/);
