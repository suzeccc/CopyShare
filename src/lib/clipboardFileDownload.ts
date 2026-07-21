import type { ClipboardPreviewItem } from "@/lib/historyPreview";
import type {
  FileTransferProgressEvent,
  FileTransferStatus,
  FileTransferTask,
} from "@/types/fileTransfer";

export type ClipboardFileDownloadActivity = {
  status: FileTransferStatus;
  transferredBytes: number;
  totalSize: number;
  error: string | null;
};

export type ClipboardFileDownloadFeedback = {
  state:
    | "ready"
    | "downloading"
    | "waiting"
    | "retrying"
    | "paused"
    | "completed"
    | "failed";
  label: string;
  percent: number;
  active: boolean;
};

export type ClipboardFileCardAction =
  | "none"
  | "copy"
  | "download"
  | "resume"
  | "downloading"
  | "openDownloadFolder"
  | "openSourceLocation"
  | "unavailable";

export const CLIPBOARD_FILE_DOWNLOAD_LIMIT = 100;

export function clipboardFileDownloadActivityFromTask(
  task: FileTransferTask,
): ClipboardFileDownloadActivity {
  return {
    status: task.status,
    transferredBytes: task.transferredBytes,
    totalSize: task.totalSize,
    error: task.error,
  };
}

export function applyClipboardFileDownloadProgress(
  current: ClipboardFileDownloadActivity | undefined,
  progress: FileTransferProgressEvent,
): ClipboardFileDownloadActivity {
  return {
    status: progress.status ?? "transferring",
    transferredBytes: Math.min(progress.totalTransferredBytes, progress.totalSize),
    totalSize: progress.totalSize,
    error: current?.error ?? null,
  };
}

export function limitClipboardFileDownloads(
  downloads: Record<string, ClipboardFileDownloadActivity>,
  limit = CLIPBOARD_FILE_DOWNLOAD_LIMIT,
): Record<string, ClipboardFileDownloadActivity> {
  if (limit <= 0) {
    return {};
  }

  const entries = Object.entries(downloads);
  if (entries.length <= limit) {
    return downloads;
  }
  return Object.fromEntries(entries.slice(-limit));
}

export function getClipboardFileDownloadFeedback(
  item: ClipboardPreviewItem,
  activity?: ClipboardFileDownloadActivity,
): ClipboardFileDownloadFeedback | null {
  if (item.contentType !== "fileList" || !item.fileTransferId) {
    return null;
  }

  const status = activity?.status ?? item.fileTransferStatus ?? "pending";
  const percent = transferPercent(
    activity?.transferredBytes ?? (status === "completed" ? 1 : 0),
    activity?.totalSize ?? 1,
  );

  if (status === "accepted" || status === "transferring") {
    return {
      state: "downloading",
      label: `下载中 ${percent}%`,
      percent,
      active: true,
    };
  }

  if (status === "waitingForPeer") {
    return {
      state: "waiting",
      label: `等待发送设备上线 · 已完成 ${percent}%`,
      percent,
      active: false,
    };
  }

  if (status === "retrying") {
    return {
      state: "retrying",
      label: `正在恢复 · 已完成 ${percent}%`,
      percent,
      active: true,
    };
  }

  if (status === "paused") {
    return {
      state: "paused",
      label: `继续下载 · 已完成 ${percent}%`,
      percent,
      active: false,
    };
  }

  if (status === "completed") {
    return {
      state: "completed",
      label: "打开文件位置",
      percent: 100,
      active: false,
    };
  }

  if (status === "failed" || status === "canceled" || status === "rejected") {
    return {
      state: "failed",
      label: "下载失败",
      percent,
      active: false,
    };
  }

  return {
    state: "ready",
    label: "点击下载",
    percent: 0,
    active: false,
  };
}

export function getClipboardFileCardAction(
  item: ClipboardPreviewItem,
  activity?: ClipboardFileDownloadActivity,
): ClipboardFileCardAction {
  if (item.contentType === "fileList" && item.direction === "local") {
    return "openSourceLocation";
  }

  if (item.contentType !== "fileList" || !item.fileTransferId) {
    return "copy";
  }

  const status = activity?.status ?? item.fileTransferStatus ?? "pending";
  if (status === "accepted" || status === "transferring" || status === "retrying") {
    return "downloading";
  }
  if (status === "waitingForPeer" || status === "paused") {
    return "resume";
  }
  if (status === "completed") {
    return "openDownloadFolder";
  }
  if (status === "failed" || status === "canceled" || status === "rejected") {
    return "unavailable";
  }
  return "download";
}

export function isClipboardFileCardInteractive(
  item: ClipboardPreviewItem,
  activity?: ClipboardFileDownloadActivity,
): boolean {
  return item.contentType === "fileList"
    && getClipboardFileCardAction(item, activity) !== "none";
}

function transferPercent(transferredBytes: number, totalSize: number): number {
  if (totalSize <= 0) {
    return 0;
  }
  return Math.min(100, Math.max(0, Math.round((transferredBytes / totalSize) * 100)));
}
