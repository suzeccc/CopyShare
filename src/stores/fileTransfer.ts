import { defineStore } from "pinia";

import {
  acceptFileTransfer,
  cancelFileTransfer,
  getFileTransfers,
  onAppEvent,
  openTransferFolder,
  rejectFileTransfer,
  selectFileForTransfer,
  selectFilesForTransfer,
  sendFileToDevice,
  sendFilesToDevice,
} from "@/lib/tauri";
import {
  applyFileTransferProgress,
  fileTransferSendDisabled,
  pendingOfferFromTasks,
  upsertFileTransferTask,
  visibleFileTransferTasks,
} from "@/lib/fileTransfer";
import { useToastStore } from "@/stores/toasts";
import type {
  FileTransferProgressEvent,
  FileTransferTask,
  SelectedTransferFile,
} from "@/types/fileTransfer";

export const useFileTransferStore = defineStore("fileTransfer", {
  state: () => ({
    selectedFiles: [] as SelectedTransferFile[],
    targetDeviceId: "",
    tasks: [] as FileTransferTask[],
    loading: false,
    error: null as string | null,
  }),
  getters: {
    selectedFile: (state) => state.selectedFiles[0] ?? null,
    sendDisabled: (state) =>
      fileTransferSendDisabled(state.selectedFiles, state.targetDeviceId) ||
      state.loading,
    pendingOffer: (state) => pendingOfferFromTasks(state.tasks),
  },
  actions: {
    async refresh() {
      this.error = null;
      try {
        this.tasks = visibleFileTransferTasks(await getFileTransfers());
      } catch (error) {
        this.error = String(error);
      }
    },
    setTargetDevice(deviceId: string) {
      this.targetDeviceId = deviceId;
    },
    async selectFile() {
      this.error = null;
      const selected = await selectFileForTransfer();
      if (selected) {
        this.selectedFiles = [selected];
      }
    },
    async selectFiles() {
      this.error = null;
      const selected = await selectFilesForTransfer();
      if (selected.length > 0) {
        this.selectedFiles = selected;
      }
    },
    async sendSelectedFile() {
      if (this.sendDisabled || this.selectedFiles.length !== 1) {
        return;
      }

      this.loading = true;
      this.error = null;
      try {
        const task = await sendFileToDevice(
          this.targetDeviceId,
          this.selectedFiles[0].path,
        );
        this.upsertTask(task);
        this.selectedFiles = [];
        useToastStore().success("文件传输请求已发送");
      } catch (error) {
        this.error = String(error);
        useToastStore().error("文件传输请求失败");
      } finally {
        this.loading = false;
      }
    },
    async sendFiles() {
      if (this.sendDisabled) {
        return;
      }

      this.loading = true;
      this.error = null;
      try {
        const task = await sendFilesToDevice(
          this.targetDeviceId,
          this.selectedFiles.map((file) => file.path),
        );
        this.upsertTask(task);
        this.selectedFiles = [];
        useToastStore().success("文件传输请求已发送");
      } catch (error) {
        this.error = String(error);
        useToastStore().error("文件传输请求失败");
      } finally {
        this.loading = false;
      }
    },
    async acceptOffer(transferId: string) {
      const task = await acceptFileTransfer(transferId);
      this.upsertTask(task);
    },
    async rejectOffer(transferId: string) {
      const task = await rejectFileTransfer(transferId);
      this.upsertTask(task);
    },
    async cancel(transferId: string) {
      const task = await cancelFileTransfer(transferId);
      this.upsertTask(task);
    },
    async openFolder() {
      await openTransferFolder();
    },
    upsertTask(task: FileTransferTask) {
      this.tasks = upsertFileTransferTask(this.tasks, task);
    },
    applyProgress(progress: FileTransferProgressEvent) {
      this.tasks = applyFileTransferProgress(this.tasks, progress);
    },
    async subscribe() {
      await Promise.all([
        onAppEvent<FileTransferTask>("file-transfer-offer", (task) => {
          this.upsertTask(task);
          useToastStore().info("收到文件传输请求");
        }),
        onAppEvent<FileTransferTask>("file-transfer-updated", (task) =>
          this.upsertTask(task),
        ),
        onAppEvent<FileTransferProgressEvent>("file-transfer-progress", (progress) =>
          this.applyProgress(progress),
        ),
        onAppEvent<FileTransferTask>("file-transfer-completed", (task) => {
          this.upsertTask(task);
          useToastStore().success("文件传输完成");
        }),
        onAppEvent<FileTransferTask>("file-transfer-failed", (task) => {
          this.upsertTask(task);
          useToastStore().error(task.error || "文件传输失败");
        }),
      ]);
    },
  },
});
