<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { ShieldCheck, ShieldQuestion, ShieldX, WifiOff, X } from "lucide-vue-next";

import Button from "@/components/ui/Button.vue";
import FloatingPanel from "@/components/layout/FloatingPanel.vue";
import Sidebar from "@/components/layout/Sidebar.vue";
import TitleBar from "@/components/layout/TitleBar.vue";
import WindowTitleBar from "@/components/layout/WindowTitleBar.vue";
import { deviceAddress } from "@/lib/format";
import {
  FLOATING_CLIPBOARD_HISTORY_LIMIT,
  getFloatingClipboardItems,
  type ClipboardPreviewItem,
} from "@/lib/historyPreview";
import { namedTrustDevices } from "@/lib/trustPrompt";
import type { ShortcutAction } from "@/lib/globalShortcut";
import {
  enterFloatingWindow,
  exitApp,
  getClipboardHistory,
  hideMainWindow,
  onAppEvent,
  onMainWindowCloseRequested,
  readClipboardText,
  recognizeClipboardImage,
  restoreMainWindow,
  showMainWindow,
  toggleFloatingClipboardHistoryWindow,
  translateText,
  updateFloatingClipboardHistoryWindow,
} from "@/lib/tauri";
import { getLatencyLabel, type AppWindowMode } from "@/lib/windowMode";
import type { WindowTransitionPointer } from "@/lib/windowTransition";
import router from "@/router";
import { useHistoryStore } from "@/stores/history";
import { useConfigStore } from "@/stores/config";
import { useDevicesStore } from "@/stores/devices";
import { useLibraryStore } from "@/stores/library";
import { useOcrStore } from "@/stores/ocr";
import { useStatusStore } from "@/stores/status";
import { useToastStore } from "@/stores/toasts";
import { useTranslationStore } from "@/stores/translation";
import type { CloseAction } from "@/types/config";

type SavedCloseAction = Exclude<CloseAction, "ask">;

const statusStore = useStatusStore();
const configStore = useConfigStore();
const historyStore = useHistoryStore();
const devicesStore = useDevicesStore();
const toastStore = useToastStore();
const libraryStore = useLibraryStore();
const ocrStore = useOcrStore();
const translationStore = useTranslationStore();
const route = useRoute();
const windowMode = ref<AppWindowMode>("main");
const isSwitchingWindowMode = ref(false);
const systemClipboardItems = ref<ClipboardPreviewItem[]>([]);
const mainScrollRef = ref<HTMLElement | null>(null);
const showCloseActionDialog = ref(false);
const rememberCloseAction = ref(false);
const closeActionSaving = ref(false);
let clipboardHistoryTimer: number | undefined;
let closeRequestUnlisten: (() => void) | undefined;
let globalShortcutUnlisten: (() => void) | undefined;

const clipboardItems = computed(() =>
  getFloatingClipboardItems(systemClipboardItems.value, historyStore.items),
);
const clipboardHistoryItems = computed(() =>
  getFloatingClipboardItems(
    systemClipboardItems.value,
    historyStore.items,
    FLOATING_CLIPBOARD_HISTORY_LIMIT,
  ),
);
const latencyLabel = computed(() =>
  getLatencyLabel({
    running: statusStore.status.running,
    connectedCount: statusStore.status.connectedCount,
    latencyMs: statusStore.status.latencyMs,
  }),
);
const isFloating = computed(() => windowMode.value === "floating");
const trustPromptDevices = computed(() => namedTrustDevices(devicesStore.pendingTrust));
const trustPromptDevice = computed(() => trustPromptDevices.value[0] ?? null);
const trustPromptExtraCount = computed(() =>
  Math.max(trustPromptDevices.value.length - 1, 0),
);

watch(
  windowMode,
  (mode) => {
    document.documentElement.dataset.windowMode = mode;
    document.body.dataset.windowMode = mode;
  },
  { immediate: true },
);

watch(
  () => configStore.config.theme,
  (theme) => {
    document.documentElement.dataset.appTheme = theme;
    document.body.dataset.appTheme = theme;
  },
  { immediate: true },
);

watch(
  () => route.fullPath,
  async () => {
    await nextTick();
    if (!mainScrollRef.value) {
      return;
    }

    mainScrollRef.value.scrollTop = 0;
    mainScrollRef.value.scrollLeft = 0;
  },
  { flush: "post" },
);

watch(
  clipboardHistoryItems,
  (items) => {
    if (!isFloating.value) {
      return;
    }

    void updateFloatingClipboardHistoryWindow({
      items,
    });
  },
  { flush: "post" },
);

onBeforeUnmount(() => {
  delete document.documentElement.dataset.windowMode;
  delete document.body.dataset.windowMode;
  delete document.documentElement.dataset.appTheme;
  delete document.body.dataset.appTheme;
  window.clearInterval(clipboardHistoryTimer);
  closeRequestUnlisten?.();
  globalShortcutUnlisten?.();
});

onMounted(async () => {
  try {
    closeRequestUnlisten = await onMainWindowCloseRequested(async (event) => {
      event.preventDefault();
      await handleCloseWindow();
    });
  } catch (error) {
    console.error("failed to register main-window close policy", error);
  }

  try {
    globalShortcutUnlisten = await onAppEvent<ShortcutAction>("global-shortcut-triggered", (action) => {
      void handleGlobalShortcut(action);
    });
  } catch (error) {
    console.error("failed to register global shortcut listener", error);
  }
});

async function refreshSystemClipboardHistory() {
  try {
    systemClipboardItems.value = (await getClipboardHistory()).map((item) => ({
      ...item,
      contentHash: "",
      contentType: "text",
      syncStatus: "unsynced",
    }));
  } catch {
    systemClipboardItems.value = [];
  }
}

watch(
  isFloating,
  (floating) => {
    window.clearInterval(clipboardHistoryTimer);
    clipboardHistoryTimer = undefined;

    if (!floating) {
      return;
    }

    void refreshSystemClipboardHistory();
    clipboardHistoryTimer = window.setInterval(() => {
      void refreshSystemClipboardHistory();
    }, 1200);
  },
  { immediate: true },
);

async function switchWindowMode(
  nextMode: AppWindowMode,
  resizeWindow: (pointer?: WindowTransitionPointer) => Promise<void>,
  pointer?: WindowTransitionPointer,
) {
  if (isSwitchingWindowMode.value) {
    return;
  }

  if (windowMode.value === nextMode) {
    return;
  }

  isSwitchingWindowMode.value = true;

  try {
    await resizeWindow(pointer);
    windowMode.value = nextMode;
  } catch (error) {
    console.error(error);
  } finally {
    isSwitchingWindowMode.value = false;
  }
}

async function toggleQuickPanelFromShortcut() {
  await refreshSystemClipboardHistory();
  await nextTick();
  await toggleFloatingClipboardHistoryWindow({
    items: clipboardHistoryItems.value,
  });
}

async function showShortcutPage(path: "/ocr" | "/translate" | "/library") {
  if (isFloating.value) {
    await restoreMainWindow();
    windowMode.value = "main";
  }
  await router.push(path);
  await showMainWindow();
}

async function recognizeClipboardFromShortcut() {
  await showShortcutPage("/ocr");
  if (ocrStore.status === "loading") return;

  ocrStore.beginRecognition();
  try {
    const response = await recognizeClipboardImage();
    ocrStore.applyResponse(response);
    if (response.error) toastStore.error(response.error);
  } catch (error) {
    const message = String(error);
    ocrStore.failRecognition(message);
    toastStore.error(message);
  }
}

async function translateClipboardFromShortcut() {
  await showShortcutPage("/translate");
  if (translationStore.loading) return;

  translationStore.loading = true;
  translationStore.error = null;
  translationStore.result = null;
  try {
    const text = (await readClipboardText()).trim();
    translationStore.inputText = text;
    translationStore.result = await translateText(text, translationStore.targetLang);
  } catch (error) {
    const message = String(error);
    translationStore.error = message;
    toastStore.error(message);
  } finally {
    translationStore.loading = false;
  }
}

async function openSnippetsFromShortcut() {
  libraryStore.activeView = "snippets";
  await showShortcutPage("/library");
}

async function toggleSyncFromShortcut() {
  if (statusStore.loading) return;
  const wasRunning = statusStore.status.running;
  if (wasRunning) {
    await statusStore.stop();
  } else {
    await statusStore.start();
  }
  if (statusStore.error) {
    toastStore.error(statusStore.error);
    return;
  }
  toastStore.success(wasRunning ? "同步已暂停" : "同步已恢复");
}

async function handleGlobalShortcut(action: ShortcutAction) {
  switch (action) {
    case "quickPanel":
      await toggleQuickPanelFromShortcut();
      break;
    case "ocr":
      await recognizeClipboardFromShortcut();
      break;
    case "translate":
      await translateClipboardFromShortcut();
      break;
    case "snippets":
      await openSnippetsFromShortcut();
      break;
    case "toggleSync":
      await toggleSyncFromShortcut();
      break;
  }
}

async function enterFloatingWindowAtPointer(pointer?: WindowTransitionPointer) {
  await enterFloatingWindow();
}

async function switchToFloatingMode(pointer: WindowTransitionPointer) {
  await switchWindowMode("floating", enterFloatingWindowAtPointer, pointer);
}

async function switchToMainMode(pointer: WindowTransitionPointer) {
  await switchWindowMode("main", restoreMainWindow, pointer);
}

async function runCloseAction(action: SavedCloseAction) {
  if (action === "minimize") {
    await hideMainWindow();
    return;
  }

  await exitApp();
}

async function saveCloseActionPreference(action: SavedCloseAction) {
  if (configStore.saving) {
    toastStore.error("设置正在保存，请稍后重试");
    return false;
  }

  closeActionSaving.value = true;
  try {
    await configStore.save({
      ...configStore.config,
      closeAction: action,
    });
    if (configStore.error) {
      toastStore.error("关闭行为保存失败");
      return false;
    }
    return true;
  } finally {
    closeActionSaving.value = false;
  }
}

async function chooseCloseAction(action: SavedCloseAction) {
  if (closeActionSaving.value) return;

  if (rememberCloseAction.value && !(await saveCloseActionPreference(action))) {
    return;
  }

  showCloseActionDialog.value = false;
  await runCloseAction(action);
}

async function handleCloseWindow() {
  const closeAction = configStore.config.closeAction ?? "ask";
  if (closeAction === "ask") {
    rememberCloseAction.value = false;
    showCloseActionDialog.value = true;
    return;
  }

  await runCloseAction(closeAction);
}

async function trustPromptDeviceNow() {
  const device = trustPromptDevice.value;
  if (!device) {
    return;
  }

  await devicesStore.trust(device.id);
}

async function rejectPromptDevice() {
  const device = trustPromptDevice.value;
  if (!device) {
    return;
  }

  await devicesStore.reject(device.id);
}
</script>

<template>
  <div
    class="app-window-shell relative flex h-screen flex-col overflow-hidden rounded-[18px] text-slate-100 transition-[background-color,border-color,padding] duration-200 ease-out"
    :class="[
      isFloating ? 'bg-transparent p-2' : 'border border-[color:var(--main-line)] bg-[color:var(--main-bg)]',
      isSwitchingWindowMode ? 'pointer-events-none' : '',
    ]"
  >
    <FloatingPanel
      v-if="isFloating"
      :status-label="statusStore.statusLabel"
      :running="statusStore.status.running"
      :connected-count="statusStore.status.connectedCount"
      :latency-label="latencyLabel"
      :clipboard-items="clipboardItems"
      :clipboard-history-items="clipboardHistoryItems"
      @restore="switchToMainMode"
      @hide="hideMainWindow"
      @close="handleCloseWindow"
    />

    <div v-else class="main-window-content flex min-h-0 flex-1 flex-col overflow-hidden">
      <WindowTitleBar @close="handleCloseWindow" />
      <div class="flex min-h-0 flex-1 overflow-hidden">
        <Sidebar />
        <main class="flex min-w-0 flex-1 flex-col">
          <TitleBar
            :switching-window-mode="isSwitchingWindowMode"
            @switch-floating="switchToFloatingMode"
          />
          <div
            ref="mainScrollRef"
            data-main-scroll-container
            class="min-h-0 flex-1 overflow-auto px-6 pb-6 pt-1.5"
          >
            <RouterView />
          </div>
        </main>
      </div>
    </div>

    <Transition name="trust-prompt">
      <div
        v-if="showCloseActionDialog"
        data-close-action-dialog
        class="absolute inset-0 z-[80] flex items-center justify-center bg-[color:var(--dialog-overlay-bg)] px-6 backdrop-blur-sm"
      >
        <section class="w-full max-w-[430px] rounded-lg border border-[color:var(--main-line)] bg-[color:var(--dialog-bg)] p-5 text-slate-100 shadow-[0_20px_70px_rgba(0,0,0,0.52)]">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-base font-semibold text-white">关闭 CopyShare？</p>
              <p class="mt-2 text-sm leading-6 text-slate-300">
                可以最小化到托盘继续同步，也可以直接退出应用。
              </p>
            </div>
            <button
              class="grid h-8 w-8 shrink-0 place-items-center rounded-md text-slate-300 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white"
              type="button"
              aria-label="关闭提示"
              title="关闭提示"
              @click="showCloseActionDialog = false"
            >
              <X class="h-4 w-4" />
            </button>
          </div>

          <label class="mt-4 flex items-center gap-2 rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] px-3 py-2.5 text-sm text-slate-300">
            <input
              v-model="rememberCloseAction"
              data-close-action-remember
              type="checkbox"
              class="h-4 w-4 rounded border-[color:var(--main-line)] bg-[color:var(--field-bg)] accent-[color:var(--theme-accent)]"
            >
            <span>记住我的选择</span>
          </label>

          <div class="mt-5 grid gap-3 sm:grid-cols-2">
            <Button
              data-close-action-minimize
              variant="primary"
              :disabled="closeActionSaving"
              @click="chooseCloseAction('minimize')"
            >
              最小化到托盘
            </Button>
            <Button
              data-close-action-exit
              variant="danger"
              :disabled="closeActionSaving"
              @click="chooseCloseAction('exit')"
            >
              直接退出
            </Button>
          </div>
        </section>
      </div>
    </Transition>

    <Transition name="trust-prompt">
      <div
        v-if="devicesStore.disconnectNotice"
        data-device-disconnect-notice
        class="absolute z-[55] flex items-start gap-3 rounded-lg border border-[color:var(--disconnect-notice-line)] bg-[color:var(--disconnect-notice-bg)] px-3 py-3 text-[color:var(--disconnect-notice-text)] shadow-[var(--disconnect-notice-shadow)] ring-1 ring-[color:var(--disconnect-notice-ring)] backdrop-blur-xl"
        :class="isFloating ? 'inset-x-2 bottom-2 text-xs' : 'right-12 top-14 w-[min(410px,calc(100%-1.5rem))] text-sm'"
      >
        <div class="grid h-9 w-9 shrink-0 place-items-center rounded-md border border-[color:var(--disconnect-notice-icon-line)] bg-[color:var(--disconnect-notice-icon-bg)] text-[color:var(--disconnect-notice-icon-text)]">
          <WifiOff class="h-4 w-4" />
        </div>
        <p class="min-w-0 flex-1 font-medium leading-6">
          {{ devicesStore.disconnectNotice }}
        </p>
        <button
          class="grid h-7 w-7 shrink-0 place-items-center rounded-md text-[color:var(--disconnect-notice-muted-text)] transition hover:bg-[color:var(--disconnect-notice-close-hover)] hover:text-[color:var(--disconnect-notice-text)]"
          type="button"
          aria-label="关闭断开提示"
          title="关闭"
          @click="devicesStore.clearDisconnectNotice()"
        >
          <X class="h-4 w-4" />
        </button>
      </div>
    </Transition>

    <Transition name="trust-prompt">
      <div
        v-if="!isFloating && trustPromptDevice"
        data-trust-prompt
        class="absolute inset-0 z-50 flex items-center justify-center bg-[color:var(--dialog-overlay-bg)] px-6 backdrop-blur-sm"
      >
        <section
          class="w-full max-w-[430px] rounded-lg border border-[color:var(--main-line)] bg-[color:var(--dialog-bg)] p-5 shadow-[0_20px_70px_rgba(0,0,0,0.48)]"
        >
          <div class="flex items-start gap-3">
            <div
              class="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]"
            >
              <ShieldQuestion class="h-5 w-5" />
            </div>
            <div class="min-w-0">
              <p class="text-base font-semibold text-white">是否信任这台设备？</p>
              <p class="mt-1 text-sm leading-6 text-slate-300">
                信任后才会同步本机剪贴板。另一台电脑也需要信任本机，才能双向同步。
              </p>
            </div>
          </div>

          <div
            class="mt-4 rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-soft)] px-3 py-2.5"
          >
            <p class="truncate text-sm font-semibold text-white">
              {{ trustPromptDevice.name }}
            </p>
            <p class="mt-1 font-mono text-xs text-slate-400">
              {{ deviceAddress(trustPromptDevice.ip, trustPromptDevice.port) }}
            </p>
          </div>

          <p v-if="trustPromptExtraCount" class="mt-3 text-xs text-slate-400">
            还有 {{ trustPromptExtraCount }} 台设备等待确认。
          </p>

          <div class="mt-5 flex justify-end gap-3">
            <Button size="md" variant="danger" @click="rejectPromptDevice">
              <ShieldX class="h-4 w-4" />
              不信任
            </Button>
            <Button size="md" variant="primary" @click="trustPromptDeviceNow">
              <ShieldCheck class="h-4 w-4" />
              信任设备
            </Button>
          </div>
        </section>
      </div>
    </Transition>

  </div>
</template>
