import { computed, ref, shallowRef } from "vue";
import { defineStore } from "pinia";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { invoke } from "@tauri-apps/api/core";

import { APP_VERSION } from "../lib/about.ts";

export const useUpdaterStore = defineStore("updater", () => {
  const update = shallowRef<Update | null>(null);
  const phase = ref<"idle" | "checking" | "available" | "downloading" | "ready" | "installing" | "installed">("idle");
  const message = ref<string | null>(null);
  const error = ref<string | null>(null);
  const downloadedBytes = ref(0);
  const totalBytes = ref<number | null>(null);
  const busy = computed(() => ["checking", "downloading", "installing"].includes(phase.value));
  const version = computed(() => update.value?.version ?? "");
  const notes = computed(() => update.value?.body ?? "");
  const progress = computed(() => totalBytes.value && totalBytes.value > 0
    ? Math.min(100, Math.round(downloadedBytes.value / totalBytes.value * 100))
    : null);

  async function checkForUpdate(silent = false) {
    if (busy.value || phase.value === "ready" || phase.value === "installed") return;
    if (!("__TAURI_INTERNALS__" in window)) return;
    phase.value = "checking";
    error.value = null;
    message.value = silent ? null : "正在检查最新版本...";
    try {
      const candidate = await check({ timeout: 20000 });
      await update.value?.close().catch(() => undefined);
      update.value = candidate;
      phase.value = candidate ? "available" : "idle";
      message.value = candidate ? `发现新版本 v${candidate.version}` : silent ? null : `已是最新版本 v${APP_VERSION}`;
    } catch (cause) {
      phase.value = update.value ? "available" : "idle";
      message.value = null;
      if (!silent) error.value = `检查更新失败：${String(cause)}`;
    }
  }

  async function downloadUpdate() {
    if (busy.value || phase.value !== "available" || !update.value) return;
    phase.value = "downloading";
    error.value = null;
    downloadedBytes.value = 0;
    totalBytes.value = null;
    message.value = "正在下载更新...";
    try {
      await update.value.download((event) => {
        if (event.event === "Started") totalBytes.value = event.data.contentLength ?? null;
        if (event.event === "Progress") downloadedBytes.value += event.data.chunkLength;
        if (event.event === "Finished") message.value = "正在验证更新包...";
      }, { timeout: 600000 });
      phase.value = "ready";
      message.value = "更新已下载并验证，安装将关闭并重新启动软件";
    } catch (cause) {
      phase.value = "available";
      message.value = null;
      error.value = `下载或验证失败，请重试：${String(cause)}`;
    }
  }

  async function restartApp() {
    try {
      await invoke("restart_app");
      error.value = null;
    } catch (cause) {
      error.value = `安装已完成，请手动重新启动软件：${String(cause)}`;
    }
  }

  async function installUpdate() {
    if (busy.value || phase.value !== "ready" || !update.value) return;
    phase.value = "installing";
    error.value = null;
    try {
      await invoke("prepare_app_update");
    } catch (cause) {
      phase.value = "ready";
      error.value = String(cause);
      return;
    }
    message.value = "正在安装更新...";
    try {
      await update.value.install({ restartAfterInstall: true });
    } catch (cause) {
      phase.value = "ready";
      error.value = `安装更新失败，请重试：${String(cause)}`;
      return;
    }
    phase.value = "installed";
    message.value = "安装已完成，正在重新启动软件";
    await restartApp();
  }

  return { phase, message, error, busy, version, notes, downloadedBytes, totalBytes, progress,
    checkForUpdate, downloadUpdate, installUpdate, restartApp };
});
