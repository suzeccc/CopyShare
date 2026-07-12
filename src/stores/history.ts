import { defineStore } from "pinia";

import { clearHistory, getHistory, onAppEvent } from "@/lib/tauri";
import {
  applyClipboardFileDownloadProgress,
  clipboardFileDownloadActivityFromTask,
  type ClipboardFileDownloadActivity,
} from "@/lib/clipboardFileDownload";
import { useToastStore } from "@/stores/toasts";
import type { FileTransferProgressEvent, FileTransferTask } from "@/types/fileTransfer";
import type { HistoryItem } from "@/types/history";

export const useHistoryStore = defineStore("history", {
  state: () => ({
    items: [] as HistoryItem[],
    loading: false,
    error: null as string | null,
    fileDownloads: {} as Record<string, ClipboardFileDownloadActivity>,
  }),
  actions: {
    async refresh() {
      this.error = null;
      try {
        this.items = await getHistory();
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
    fileDownloadActivity(transferId?: string) {
      return transferId ? this.fileDownloads[transferId] : undefined;
    },
    isFileDownloadActive(transferId?: string) {
      const status = this.fileDownloadActivity(transferId)?.status;
      return status === "accepted" || status === "transferring";
    },
    beginFileDownload(transferId?: string) {
      if (!transferId) {
        return;
      }
      const current = this.fileDownloads[transferId];
      this.fileDownloads = {
        ...this.fileDownloads,
        [transferId]: {
          status: "accepted",
          transferredBytes: current?.transferredBytes ?? 0,
          totalSize: current?.totalSize ?? 0,
          error: null,
        },
      };
    },
    failFileDownload(transferId?: string, error = "文件下载失败") {
      if (!transferId) {
        return;
      }
      const current = this.fileDownloads[transferId];
      this.fileDownloads = {
        ...this.fileDownloads,
        [transferId]: {
          status: "failed",
          transferredBytes: current?.transferredBytes ?? 0,
          totalSize: current?.totalSize ?? 0,
          error,
        },
      };
    },
    updateFileDownloadTask(task: FileTransferTask) {
      if (!task.clipboardSync) {
        return;
      }
      this.fileDownloads = {
        ...this.fileDownloads,
        [task.transferId]: clipboardFileDownloadActivityFromTask(task),
      };
    },
    updateFileDownloadProgress(progress: FileTransferProgressEvent) {
      const belongsToClipboard = Boolean(this.fileDownloads[progress.transferId])
        || this.items.some((item) => item.fileTransferId === progress.transferId);
      if (!belongsToClipboard) {
        return;
      }
      this.fileDownloads = {
        ...this.fileDownloads,
        [progress.transferId]: applyClipboardFileDownloadProgress(
          this.fileDownloads[progress.transferId],
          progress,
        ),
      };
    },
    async subscribe() {
      await Promise.all([
        onAppEvent<HistoryItem>("clipboard-synced", (item) => {
          this.items = [item, ...this.items.filter((existing) => existing.id !== item.id)].slice(0, 100);
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
