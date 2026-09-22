<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { Menu } from "@tauri-apps/api/menu";
import ShieldCheck from "lucide-vue-next/dist/esm/icons/shield-check.js";
import ShieldQuestion from "lucide-vue-next/dist/esm/icons/shield-question-mark.js";
import ShieldX from "lucide-vue-next/dist/esm/icons/shield-x.js";
import WifiOff from "lucide-vue-next/dist/esm/icons/wifi-off.js";
import X from "lucide-vue-next/dist/esm/icons/x.js";

import Button from "@/components/ui/Button.vue";
import { translateSource } from "@/i18n";
import FloatingPanel from "@/components/layout/FloatingPanel.vue";
import FloatingBall from "@/components/layout/FloatingBall.vue";
import Sidebar from "@/components/layout/Sidebar.vue";
import TitleBar from "@/components/layout/TitleBar.vue";
import WindowTitleBar from "@/components/layout/WindowTitleBar.vue";
import { deviceAddress } from "@/lib/format";
import {
  FLOATING_CLIPBOARD_HISTORY_LIMIT,
  FLOATING_CLIPBOARD_PREVIEW_LIMIT,
  getFloatingClipboardItems,
  type ClipboardPreviewItem,
} from "@/lib/historyPreview";
import { namedTrustDevices } from "@/lib/trustPrompt";
import type { ShortcutAction } from "@/lib/globalShortcut";
import {
  enterFloatingWindow,
  enterBallWindow,
  dockBallWindow,
  exitApp,
  getClipboardHistory,
  hideMainWindow,
  isMainWindowVisible,
  onAppEvent,
  onMainWindowCloseRequested,
  onMainWindowFocusChanged,
  readClipboardText,
  recognizeClipboardImage,
  restoreMainWindow,
  showMainWindow,
  toggleFloatingClipboardHistoryWindow,
  translateText,
  updateFloatingClipboardHistoryWindow,
} from "@/lib/tauri";
import { getLatencyLabel, type AppWindowMode } from "@/lib/windowMode";
import { WINDOW_MODE_ENTER_MS, WINDOW_MODE_EXIT_MS, type WindowTransitionPointer } from "@/lib/windowTransition";
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

const emit = defineEmits<{ (event: "ready"): void }>();
const props = defineProps<{ startupAnimationComplete: Promise<void> }>();
const statusStore = useStatusStore();
const configStore = useConfigStore();
const historyStore = useHistoryStore();
const devicesStore = useDevicesStore();
const toastStore = useToastStore();
const libraryStore = useLibraryStore();
const ocrStore = useOcrStore();
const translationStore = useTranslationStore();
const route = useRoute();
const windowMode = ref<AppWindowMode>(configStore.config.startupWindowMode === "ball" ? "floating" : configStore.config.startupWindowMode ?? "floating");
const isSwitchingWindowMode = ref(false);
const isResizingWindow = ref(false);
const panelTransitionPhase = ref<"exit" | "enter" | null>(null);
const isCollapsingToBall = ref(false);
const windowModeFailed = ref(false);
const ballReturnMode = ref<"main" | "floating">("floating");
const systemClipboardItems = ref<ClipboardPreviewItem[]>([]);
const mainScrollRef = ref<HTMLElement | null>(null);
const showCloseActionDialog = ref(false);
const rememberCloseAction = ref(false);
const closeActionSaving = ref(false);
let clipboardHistoryTimer: number | undefined;
let clipboardHistoryPollingGeneration = 0;
let clipboardHistoryPollingDisposed = false;
let clipboardHistoryRefreshCount = 0;
let windowFocusUnlisten: (() => void) | undefined;
let closeRequestUnlisten: (() => void) | undefined;
let globalShortcutUnlisten: (() => void) | undefined;
let windowModeChange: Promise<void> = Promise.resolve();
let ballMenu: Menu | null = null;

const clipboardHistoryItems = computed(() =>
  getFloatingClipboardItems(
    systemClipboardItems.value,
    historyStore.items,
    FLOATING_CLIPBOARD_HISTORY_LIMIT,
  ),
);
const clipboardItems = computed(() =>
  clipboardHistoryItems.value.slice(0, FLOATING_CLIPBOARD_PREVIEW_LIMIT),
);
const latencyLabel = computed(() =>
  getLatencyLabel({
    running: statusStore.status.running,
    connectedCount: statusStore.status.connectedCount,
    latencyMs: statusStore.status.latencyMs,
  }),
);
const isFloating = computed(() => windowMode.value === "floating");
const isBall = computed(() => windowMode.value === "ball");
const trustPromptDevices = computed(() => namedTrustDevices(devicesStore.pendingTrust));
const trustPromptDevice = computed(() => trustPromptDevices.value[0] ?? null);
const trustPromptExtraCount = computed(() =>
  Math.max(trustPromptDevices.value.length - 1, 0),
);
const keepFloatingAfterOnboarding = ref(false);

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
    if (windowMode.value !== "main") {
      if (keepFloatingAfterOnboarding.value) {
        keepFloatingAfterOnboarding.value = false;
      } else {
        await switchWindowMode("main", restoreMainWindow);
      }
    }
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

watch(
  () => configStore.config.onboardingCompleted,
  (completed, previous) => {
    if (!completed || previous || windowMode.value !== "main") return;
    keepFloatingAfterOnboarding.value = true;
    void switchWindowMode("floating", () => enterFloatingWindow("top-right"));
  },
);

watch(() => configStore.config.uiLanguage, () => {
  if (ballMenu) void ballMenu.close();
  ballMenu = null;
});

watch(trustPromptDevice, (device) => {
  if (device && isBall.value) void switchWindowMode("main", restoreMainWindow);
});

onBeforeUnmount(() => {
  clipboardHistoryPollingDisposed = true;
  delete document.documentElement.dataset.windowMode;
  delete document.body.dataset.windowMode;
  delete document.documentElement.dataset.appTheme;
  delete document.body.dataset.appTheme;
  stopClipboardHistoryPolling();
  windowFocusUnlisten?.();
  closeRequestUnlisten?.();
  globalShortcutUnlisten?.();
  if (ballMenu) void ballMenu.close();
});

onMounted(async () => {
  windowModeChange = initializeStartupWindow();
  await windowModeChange;
  emit("ready");
  try {
    windowFocusUnlisten = await onMainWindowFocusChanged(() => {
      void startClipboardHistoryPolling();
    });
    if (clipboardHistoryPollingDisposed) windowFocusUnlisten();
  } catch (error) {
    console.error("failed to register main-window focus listener", error);
  }
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
  await startClipboardHistoryPolling();
});

async function initializeStartupWindow() {
  await props.startupAnimationComplete;
  if (!configStore.config.onboardingCompleted) windowMode.value = "main";
  if (!("__TAURI_INTERNALS__" in window)) return;

  try {
    if (windowMode.value === "ball") {
      await enterBallWindow("top-right");
    } else if (windowMode.value === "floating") {
      await enterFloatingWindow("top-right");
    } else {
      await restoreMainWindow();
    }
  } catch (error) {
    console.error("failed to apply startup window mode", error);
    try {
      await restoreMainWindow();
      windowMode.value = "main";
    } catch (restoreError) {
      console.error("failed to restore startup window", restoreError);
      windowModeFailed.value = true;
    }
  } finally {
    await showMainWindow().catch(console.error);
  }
}

async function refreshSystemClipboardHistory(visibleOnly = false) {
  if (visibleOnly && clipboardHistoryRefreshCount > 0) return;
  clipboardHistoryRefreshCount += 1;
  try {
    if (visibleOnly && !(await isMainWindowVisible())) {
      stopClipboardHistoryPolling();
      return;
    }
    systemClipboardItems.value = (await getClipboardHistory()).map((item) => ({
      ...item,
      contentHash: "",
      contentType: "text",
      syncStatus: "unsynced",
    }));
  } catch {
    systemClipboardItems.value = [];
  } finally {
    clipboardHistoryRefreshCount -= 1;
  }
}

function stopClipboardHistoryPolling() {
  clipboardHistoryPollingGeneration += 1;
  window.clearInterval(clipboardHistoryTimer);
  clipboardHistoryTimer = undefined;
}

async function startClipboardHistoryPolling() {
  stopClipboardHistoryPolling();
  const generation = clipboardHistoryPollingGeneration;
  if (clipboardHistoryPollingDisposed || !isFloating.value) return;
  try {
    if (!(await isMainWindowVisible())) return;
  } catch {
    return;
  }
  if (generation !== clipboardHistoryPollingGeneration || !isFloating.value) return;
  void refreshSystemClipboardHistory(true);
  clipboardHistoryTimer = window.setInterval(() => {
    void refreshSystemClipboardHistory(true);
  }, 1200);
}

watch(
  isFloating,
  () => {
    void startClipboardHistoryPolling();
  },
  { immediate: true },
);

function switchWindowMode(
  nextMode: AppWindowMode,
  resizeWindow: (pointer?: WindowTransitionPointer) => Promise<void>,
  pointer?: WindowTransitionPointer,
) {
  windowModeChange = windowModeChange.then(async () => {
    if (windowMode.value === nextMode && !windowModeFailed.value) {
      return;
    }
    const previousMode = windowMode.value;
    const animatePanels = previousMode !== nextMode
      && previousMode !== "ball"
      && nextMode !== "ball";
    const hideNativeWindow = previousMode === "ball" && nextMode !== "ball";
    let nativeWindowHidden = false;
    isSwitchingWindowMode.value = true;
    if (animatePanels) {
      panelTransitionPhase.value = "exit";
      await nextTick();
      await new Promise((resolve) => window.setTimeout(resolve, WINDOW_MODE_EXIT_MS));
    }
    if (nextMode === "ball") {
      if (!window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
        isCollapsingToBall.value = true;
        await new Promise((resolve) => window.setTimeout(resolve, 100));
      }
      isCollapsingToBall.value = false;
    }
    panelTransitionPhase.value = null;
    isResizingWindow.value = true;
    await nextTick();
    try {
      if (hideNativeWindow) {
        await hideMainWindow();
        nativeWindowHidden = true;
      }
      await resizeWindow(pointer);
      windowMode.value = nextMode;
      windowModeFailed.value = false;
      await nextTick();
    } catch (error) {
      console.error("failed to switch window mode", error);
      try {
        await (previousMode === "ball" ? enterBallWindow() : previousMode === "floating" ? enterFloatingWindow() : restoreMainWindow());
        windowModeFailed.value = false;
      } catch (restoreError) {
        console.error("failed to restore window mode", restoreError);
        windowModeFailed.value = true;
      }
      toastStore.error("窗口切换失败，请重试");
    } finally {
      panelTransitionPhase.value = animatePanels && !windowModeFailed.value ? "enter" : null;
      isResizingWindow.value = false;
      await nextTick();
      if (nativeWindowHidden) {
        await showMainWindow();
      }
      if (panelTransitionPhase.value === "enter") {
        await new Promise((resolve) => window.setTimeout(resolve, WINDOW_MODE_ENTER_MS));
      }
      panelTransitionPhase.value = null;
      isSwitchingWindowMode.value = false;
    }
  });
  return windowModeChange;
}

async function toggleQuickPanelFromShortcut() {
  await refreshSystemClipboardHistory();
  await nextTick();
  await toggleFloatingClipboardHistoryWindow({
    items: clipboardHistoryItems.value,
  });
}

async function showShortcutPage(path: "/ocr" | "/translate" | "/library") {
  await switchWindowMode("main", restoreMainWindow);
  if (windowMode.value !== "main") return false;
  await router.push(path);
  await showMainWindow();
  return true;
}

async function recognizeClipboardFromShortcut() {
  if (!(await showShortcutPage("/ocr"))) return;
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
  if (!(await showShortcutPage("/translate"))) return;
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

async function switchToBallMode() {
  if (windowMode.value === "ball") return;
  ballReturnMode.value = windowMode.value === "main" ? "main" : "floating";
  await switchWindowMode("ball", () => enterBallWindow());
}

async function restoreFromBall() {
  const mode = ballReturnMode.value;
  await switchWindowMode(mode, mode === "main" ? restoreMainWindow : () => enterFloatingWindow());
}

async function openBallMenu() {
  try {
    ballMenu ??= await Menu.new({ items: [
      { text: translateSource("打开主面板"), action: () => { void switchWindowMode("main", restoreMainWindow); } },
      { text: translateSource("隐藏到托盘"), action: () => { void hideMainWindow(); } },
    ] });
    await ballMenu.popup();
  } catch (error) {
    toastStore.error(`打开浮窗球菜单失败：${String(error)}`);
  }
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
    if (isBall.value) await switchWindowMode("main", restoreMainWindow);
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
    class="app-window-shell relative flex h-screen flex-col overflow-hidden text-slate-100 transition-[background-color,border-color,padding] duration-200 ease-out"
    :data-panel-transition="panelTransitionPhase"
    :class="[
      isBall ? 'rounded-full bg-transparent p-0' : isFloating ? 'rounded-[18px] bg-transparent p-2' : 'rounded-[18px] border border-[color:var(--main-line)] bg-[color:var(--main-bg)]',
      isSwitchingWindowMode ? 'pointer-events-none' : '',
      isResizingWindow ? 'is-mode-resizing' : '',
      isCollapsingToBall ? 'is-collapsing-to-ball' : '',
    ]"
  >
    <FloatingBall
      v-if="isBall"
      v-show="!isResizingWindow && !windowModeFailed"
      :running="statusStore.status.running"
      :connected-count="statusStore.status.connectedCount"
      @open="restoreFromBall"
      @menu="openBallMenu"
      @dock="dockBallWindow"
    />
    <FloatingPanel
      v-else-if="isFloating"
      v-show="!isResizingWindow && !windowModeFailed"
      :status-label="statusStore.statusLabel"
      :running="statusStore.status.running"
      :connected-count="statusStore.status.connectedCount"
      :latency-label="latencyLabel"
      :clipboard-items="clipboardItems"
      :clipboard-history-items="clipboardHistoryItems"
      @restore="switchToMainMode"
      @hide="switchToBallMode"
      @close="handleCloseWindow"
    />

    <div v-else class="main-window-content flex min-h-0 flex-1 flex-col overflow-hidden" v-show="!isResizingWindow && !windowModeFailed">
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
            <RouterView v-slot="{ Component, route: routedRoute }">
              <Transition name="page" mode="out-in">
                <component :is="Component" :key="routedRoute.fullPath" />
              </Transition>
            </RouterView>
          </div>
        </main>
      </div>
    </div>

    <section v-if="windowModeFailed" role="alert" class="absolute inset-2 flex flex-col items-center justify-center gap-4 rounded-lg border border-[color:var(--main-line)] bg-[color:var(--main-bg)] p-5 text-center">
      <p class="text-sm text-slate-200">窗口尺寸调整失败，请重试</p>
      <div class="flex gap-2">
        <Button :disabled="isSwitchingWindowMode" @click="switchWindowMode(windowMode, isBall ? () => enterBallWindow() : isFloating ? enterFloatingWindowAtPointer : restoreMainWindow)">重新调整窗口</Button>
        <Button variant="ghost" @click="handleCloseWindow">关闭</Button>
      </div>
    </section>

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
                可以最小化到托盘继续同步，也可以直接退出应用
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
        v-if="!isBall && devicesStore.disconnectNotice"
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
        v-if="windowMode === 'main' && trustPromptDevice"
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
                信任后才会同步本机剪贴板。另一台电脑也需要信任本机，才能双向同步
              </p>
            </div>
          </div>

          <div
            class="mt-4 rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-soft)] px-3 py-2.5"
          >
            <p class="truncate text-sm font-semibold text-white">
              <span data-i18n-ignore>{{ trustPromptDevice.name }}</span>
            </p>
            <p class="mt-1 font-mono text-xs text-slate-400">
              {{ deviceAddress(trustPromptDevice.ip, trustPromptDevice.port) }}
            </p>
          </div>

          <p v-if="trustPromptExtraCount" class="mt-3 text-xs text-slate-400">
            还有 {{ trustPromptExtraCount }} 台设备等待确认
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
