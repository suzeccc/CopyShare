import { defineStore } from "pinia";

import { clearHistory, getHistory, onAppEvent, setHistoryItemPinned } from "@/lib/tauri";
import {
  applyClipboardFileDownloadProgress,
  clipboardFileDownloadActivityFromFile,
  clipboardFileDownloadActivityFromTask,
  clipboardFileDownloadKey,
  limitClipboardFileDownloads,
  type ClipboardFileDownloadActivity,
} from "@/lib/clipboardFileDownload";
import { useToastStore } from "@/stores/toasts";
import type { FileTransferProgressEvent, FileTransferTask } from "@/types/fileTransfer";
import type { HistoryItem } from "@/types/history";

function sortHistoryItems(items: HistoryItem[]) {
  return [...items].sort((left, right) => {
    if (left.isPinned !== right.isPinned) return left.isPinned ? -1 : 1;
    if (left.isPinned && right.isPinned) {
      return (Date.parse(right.pinnedAt ?? "") || 0) - (Date.parse(left.pinnedAt ?? "") || 0);
    }
    return Date.parse(right.createdAt) - Date.parse(left.createdAt);
  });
}

export const useHistoryStore = defineStore("history", {
  state: () => ({
    items: [] as HistoryItem[],
    loading: false,
    error: null as string | null,
    fileDownloads: {} as Record<string, ClipboardFileDownloadActivity>,
    pinningItemIds: new Set<string>(),
    unlisteners: [] as (() => void)[],
  }),
  actions: {
    async refresh() {
      this.error = null;
      try {
        this.items = sortHistoryItems(await getHistory());
      } catch (error) {
        this.error = String(error);
      }
    },
    async clear() {
      this.loading = true;
      this.error = null;
      try {
        await clearHistory();
        this.items = [];
        this.fileDownloads = {};
      } catch (error) {
        this.error = String(error);
      } finally {
        this.loading = false;
      }
    },
    isPinning(id: string) {
      return this.pinningItemIds.has(id);
    },
    async setPinned(id: string, pinned: boolean) {
      if (this.pinningItemIds.has(id)) return;
      this.pinningItemIds = new Set(this.pinningItemIds).add(id);
      try {
        this.items = sortHistoryItems(await setHistoryItemPinned(id, pinned));
      } finally {
        const next = new Set(this.pinningItemIds);
        next.delete(id);
        this.pinningItemIds = next;
      }
    },
    fileDownloadActivity(transferId?: string, fileId?: string) {
      const key = clipboardFileDownloadKey(transferId, fileId);
      return key ? this.fileDownloads[key] : undefined;
    },
    isFileDownloadActive(transferId?: string, fileId?: string) {
      const status = this.fileDownloadActivity(transferId, fileId)?.status;
      return status === "accepted" || status === "transferring" || status === "retrying";
    },
    beginFileDownload(transferId?: string, fileId?: string) {
      const key = clipboardFileDownloadKey(transferId, fileId);
      if (!key) {
        return;
      }
      const current = this.fileDownloads[key];
      this.fileDownloads = limitClipboardFileDownloads({
        ...this.fileDownloads,
        [key]: {
          status: "accepted",
          transferredBytes: current?.transferredBytes ?? 0,
          totalSize: current?.totalSize ?? 0,
          error: null,
        },
      });
    },
    failFileDownload(transferId?: string, fileId?: string, error = "文件下载失败") {
      const key = clipboardFileDownloadKey(transferId, fileId);
      if (!key) {
        return;
      }
      const current = this.fileDownloads[key];
      this.fileDownloads = limitClipboardFileDownloads({
        ...this.fileDownloads,
        [key]: {
          status: "failed",
          transferredBytes: current?.transferredBytes ?? 0,
          totalSize: current?.totalSize ?? 0,
          error,
        },
      });
    },
    updateFileDownloadTask(task: FileTransferTask) {
      if (!task.clipboardSync) {
        return;
      }
      const next = {
        ...this.fileDownloads,
        [task.transferId]: clipboardFileDownloadActivityFromTask(task),
      };
      for (const file of task.files) {
        const key = clipboardFileDownloadKey(task.transferId, file.id);
        if (key) {
          next[key] = clipboardFileDownloadActivityFromFile(task, file);
        }
      }
      this.fileDownloads = limitClipboardFileDownloads(next);
    },
    updateFileDownloadProgress(progress: FileTransferProgressEvent) {
      const belongsToClipboard = Boolean(this.fileDownloads[progress.transferId])
        || this.items.some((item) => item.fileTransferId === progress.transferId);
      if (!belongsToClipboard) {
        return;
      }
      this.fileDownloads = limitClipboardFileDownloads({
        ...this.fileDownloads,
        [progress.transferId]: applyClipboardFileDownloadProgress(
          this.fileDownloads[progress.transferId],
          progress,
        ),
        [`${progress.transferId}:${progress.fileId}`]: {
          status: progress.status ?? "transferring",
          transferredBytes: Math.min(progress.fileTransferredBytes, progress.fileSize),
          totalSize: progress.fileSize,
          error: this.fileDownloads[`${progress.transferId}:${progress.fileId}`]?.error ?? null,
        },
      });
    },
    async subscribe() {
      if (this.unlisteners.length) {
        return;
      }
      this.unlisteners = await Promise.all([
        onAppEvent<HistoryItem>("clipboard-synced", (item) => {
          this.items = sortHistoryItems([item, ...this.items.filter((existing) => existing.id !== item.id)]).slice(0, 100);
        }),
        onAppEvent<HistoryItem[]>("history-updated", (items) => {
          this.items = sortHistoryItems(items);
        }),
        onAppEvent<FileTransferTask>("file-transfer-updated", (task) => {
          this.updateFileDownloadTask(task);
        }),
        onAppEvent<FileTransferProgressEvent>("file-transfer-progress", (progress) => {
          this.updateFileDownloadProgress(progress);
        }),
        onAppEvent<FileTransferTask>("file-transfer-completed", (task) => {
          if (!task.clipboardSync) {
            return;
          }
          this.updateFileDownloadTask(task);
          useToastStore().success("文件下载完成，已写入剪贴板");
        }),
        onAppEvent<FileTransferTask>("file-transfer-failed", (task) => {
          if (!task.clipboardSync) {
            return;
          }
          this.updateFileDownloadTask(task);
          useToastStore().error(task.error ? `文件下载失败：${task.error}` : "文件下载失败");
        }),
      ]);
    },
  },
});
