import assert from "node:assert/strict";

import {
  applyClipboardFileDownloadProgress,
  clipboardFileDownloadActivityFromTask,
  getClipboardFileCardAction,
  getClipboardFileDownloadFeedback,
} from "../src/lib/clipboardFileDownload.ts";
import type { ClipboardPreviewItem } from "../src/lib/historyPreview.ts";
import type { FileTransferTask } from "../src/types/fileTransfer.ts";

const item: ClipboardPreviewItem = {
  id: "history-file",
  text: "茶话间.lnk 897 B",
  contentType: "fileList",
  direction: "remote",
  syncStatus: "synced",
  fileTransferId: "transfer-file",
  fileTransferStatus: "pending",
};

function task(status: FileTransferTask["status"]): FileTransferTask {
  return {
    transferId: "transfer-file",
    direction: "receive",
    peerDeviceId: "device-a",
    peerDeviceName: "Suzec",
    clipboardSync: true,
    files: [],
    totalSize: 1000,
    transferredBytes: status === "completed" ? 1000 : 0,
    status,
    createdAt: "2026-07-10T15:55:00Z",
    completedAt: status === "completed" ? "2026-07-10T15:56:00Z" : null,
    error: status === "failed" ? "connection closed" : null,
  };
}

assert.deepEqual(getClipboardFileDownloadFeedback(item), {
  state: "ready",
  label: "点击下载",
  percent: 0,
  active: false,
});
assert.equal(getClipboardFileCardAction(item), "download");
assert.equal(
  getClipboardFileCardAction({ ...item, direction: "local" }),
  "openSourceLocation",
);

const accepted = clipboardFileDownloadActivityFromTask(task("accepted"));
assert.deepEqual(getClipboardFileDownloadFeedback(item, accepted), {
  state: "downloading",
  label: "下载中 0%",
  percent: 0,
  active: true,
});
assert.equal(getClipboardFileCardAction(item, accepted), "downloading");

const progressed = applyClipboardFileDownloadProgress(accepted, {
  transferId: "transfer-file",
  fileId: "file-a",
  fileTransferredBytes: 450,
  fileSize: 1000,
  totalTransferredBytes: 450,
  totalSize: 1000,
  status: "transferring",
});
assert.deepEqual(getClipboardFileDownloadFeedback(item, progressed), {
  state: "downloading",
  label: "下载中 45%",
  percent: 45,
  active: true,
});

assert.deepEqual(
  getClipboardFileDownloadFeedback(
    { ...item, fileTransferStatus: "completed" },
    clipboardFileDownloadActivityFromTask(task("completed")),
  ),
  {
    state: "completed",
    label: "打开文件位置",
    percent: 100,
    active: false,
  },
);
assert.equal(
  getClipboardFileCardAction(
    { ...item, fileTransferStatus: "completed" },
    clipboardFileDownloadActivityFromTask(task("completed")),
  ),
  "openDownloadFolder",
);

assert.deepEqual(
  getClipboardFileDownloadFeedback(
    { ...item, fileTransferStatus: "failed" },
    clipboardFileDownloadActivityFromTask(task("failed")),
  ),
  {
    state: "failed",
    label: "下载失败",
    percent: 0,
    active: false,
  },
);

assert.equal(
  getClipboardFileDownloadFeedback({ ...item, contentType: "text" }),
  null,
);
assert.equal(
  getClipboardFileCardAction({ ...item, fileTransferId: undefined }),
  "copy",
);
