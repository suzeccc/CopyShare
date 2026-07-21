import assert from "node:assert/strict";

import {
  applyFileTransferProgress,
  fileTransferStatusLabel,
  fileTransferSendDisabled,
  pendingOfferFromTasks,
  upsertFileTransferTask,
  visibleFileTransferTasks,
} from "../src/lib/fileTransfer.ts";
import type { FileTransferTask } from "../src/types/fileTransfer.ts";

function task(overrides: Partial<FileTransferTask> = {}): FileTransferTask {
  return {
    transferId: overrides.transferId ?? "transfer-1",
    direction: overrides.direction ?? "receive",
    peerDeviceId: overrides.peerDeviceId ?? "device-a",
    peerDeviceName: overrides.peerDeviceName ?? "Laptop A",
    clipboardSync: overrides.clipboardSync ?? false,
    files: overrides.files ?? [
      {
        id: "file-1",
        name: "hello.txt",
        size: 100,
        sha256: "hash",
        savedPath: null,
        transferredBytes: 0,
        status: "pending",
        error: null,
      },
    ],
    totalSize: overrides.totalSize ?? 100,
    transferredBytes: overrides.transferredBytes ?? 0,
    status: overrides.status ?? "pending",
    createdAt: overrides.createdAt ?? "2026-07-01T00:00:00Z",
    completedAt: overrides.completedAt ?? null,
    error: overrides.error ?? null,
  };
}

assert.equal(fileTransferSendDisabled([], ""), true);
assert.equal(
  fileTransferSendDisabled(
    [{ path: "C:/tmp/hello.txt", name: "hello.txt", size: 100, sha256: "hash" }],
    "",
  ),
  true,
);
assert.equal(
  fileTransferSendDisabled(
    [{ path: "C:/tmp/hello.txt", name: "hello.txt", size: 100, sha256: "hash" }],
    "device-b",
  ),
  false,
);

const pending = task({ transferId: "pending-transfer", status: "pending" });
const completed = task({ transferId: "completed-transfer", status: "completed" });
assert.equal(pendingOfferFromTasks([completed, pending])?.transferId, "pending-transfer");
assert.equal(
  pendingOfferFromTasks([
    task({ transferId: "clipboard-pending", status: "pending", clipboardSync: true }),
  ]),
  null,
);

const inserted = upsertFileTransferTask([], pending);
assert.deepEqual(inserted.map((item) => item.transferId), ["pending-transfer"]);

const replaced = upsertFileTransferTask(inserted, {
  ...pending,
  status: "transferring",
});
assert.equal(replaced.length, 1);
assert.equal(replaced[0].status, "transferring");

const removedAfterCancel = upsertFileTransferTask(replaced, {
  ...pending,
  status: "canceled",
});
assert.deepEqual(removedAfterCancel.map((item) => item.transferId), []);

assert.deepEqual(
  visibleFileTransferTasks([
    task({ transferId: "active-transfer", status: "pending" }),
    task({ transferId: "canceled-transfer", status: "canceled" }),
  ]).map((item) => item.transferId),
  ["active-transfer"],
);

const progressed = applyFileTransferProgress(replaced, {
  transferId: "pending-transfer",
  fileId: "file-1",
  fileTransferredBytes: 40,
  fileSize: 100,
  totalTransferredBytes: 40,
  totalSize: 100,
});
assert.equal(progressed[0].transferredBytes, 40);
assert.equal(progressed[0].files[0].transferredBytes, 40);
assert.equal(progressed[0].files[0].status, "transferring");
assert.equal(progressed[0].status, "transferring");
assert.equal(fileTransferStatusLabel("waitingForPeer"), "等待发送设备上线");
assert.equal(fileTransferStatusLabel("retrying"), "正在恢复");
assert.equal(fileTransferStatusLabel("paused"), "传输已暂停");
