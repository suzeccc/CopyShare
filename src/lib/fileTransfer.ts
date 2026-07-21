import type {
  FileTransferProgressEvent,
  FileTransferTask,
  SelectedTransferFile,
} from "@/types/fileTransfer";

export function fileTransferSendDisabled(
  selectedFiles: SelectedTransferFile[],
  targetDeviceId: string,
): boolean {
  return selectedFiles.length === 0 || !targetDeviceId.trim();
}

export function upsertFileTransferTask(
  tasks: FileTransferTask[],
  task: FileTransferTask,
): FileTransferTask[] {
  const next = tasks.filter((item) => item.transferId !== task.transferId);
  if (task.status === "canceled") {
    return next;
  }
  next.unshift(task);
  return next;
}

export function visibleFileTransferTasks(tasks: FileTransferTask[]): FileTransferTask[] {
  return tasks.filter((task) => task.status !== "canceled");
}

export function applyFileTransferProgress(
  tasks: FileTransferTask[],
  progress: FileTransferProgressEvent,
): FileTransferTask[] {
  return tasks.map((task) => {
    if (task.transferId !== progress.transferId) {
      return task;
    }

    return {
      ...task,
      transferredBytes: Math.min(progress.totalTransferredBytes, progress.totalSize),
      totalSize: progress.totalSize,
      status: progress.status ?? "transferring",
      files: task.files.map((file) => {
        if (file.id !== progress.fileId) {
          return file;
        }
        return {
          ...file,
          transferredBytes: Math.min(
            progress.fileTransferredBytes,
            progress.fileSize,
          ),
          size: progress.fileSize,
          status: progress.status === "completed" ? "completed" : "transferring",
        };
      }),
    };
  });
}

export function pendingOfferFromTasks(
  tasks: FileTransferTask[],
): FileTransferTask | null {
  return (
    tasks.find(
      (task) =>
        task.direction === "receive" &&
        task.status === "pending" &&
        !task.clipboardSync,
    ) ?? null
  );
}

export function transferProgressPercent(task: FileTransferTask): number {
  if (task.totalSize <= 0) {
    return 0;
  }
  return Math.min(100, Math.round((task.transferredBytes / task.totalSize) * 100));
}

export function fileProgressPercent(file: FileTransferTask["files"][number]): number {
  if (file.size <= 0) {
    return 0;
  }
  return Math.min(100, Math.round((file.transferredBytes / file.size) * 100));
}

export function selectedFilesTotalSize(files: SelectedTransferFile[]): number {
  return files.reduce((total, file) => total + file.size, 0);
}

export function currentTransferFileName(task: FileTransferTask): string {
  return (
    task.files.find((file) => file.status === "transferring")?.name ??
    task.files.find((file) => file.status === "pending")?.name ??
    task.files[0]?.name ??
    "未知文件"
  );
}

export function formatTransferSize(size: number): string {
  if (size < 1024) {
    return `${size} B`;
  }
  const units = ["KB", "MB", "GB"];
  let value = size / 1024;
  let unit = units[0];
  for (let index = 1; value >= 1024 && index < units.length; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value.toFixed(value >= 10 ? 1 : 2)} ${unit}`;
}

export function fileTransferStatusLabel(status: FileTransferTask["status"]): string {
  if (status === "waitingForPeer") return "等待发送设备上线";
  if (status === "retrying") return "正在恢复";
  if (status === "paused") return "传输已暂停";

  const labels: Partial<Record<FileTransferTask["status"], string>> = {
    pending: "等待确认",
    accepted: "已接受",
    transferring: "传输中",
    completed: "已完成",
    failed: "失败",
    canceled: "已取消",
    rejected: "已拒绝",
  };
  return labels[status] ?? status;
}
