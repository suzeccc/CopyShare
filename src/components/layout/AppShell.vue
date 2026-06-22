<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";

import FloatingPanel from "@/components/layout/FloatingPanel.vue";
import Sidebar from "@/components/layout/Sidebar.vue";
import TitleBar from "@/components/layout/TitleBar.vue";
import WindowTitleBar from "@/components/layout/WindowTitleBar.vue";
import { getFloatingClipboardItems, type ClipboardPreviewItem } from "@/lib/historyPreview";
import {
  closeWindow,
  enterFloatingWindow,
  getClipboardHistory,
  restoreMainWindow,
} from "@/lib/tauri";
import { getLatencyLabel, type AppWindowMode } from "@/lib/windowMode";
import {
  getWindowModeTransition,
  WINDOW_MODE_ENTER_MS,
  WINDOW_MODE_EXIT_MS,
  type WindowTransitionPhase,
} from "@/lib/windowTransition";
import { useHistoryStore } from "@/stores/history";
import { useStatusStore } from "@/stores/status";

const statusStore = useStatusStore();
const historyStore = useHistoryStore();
const windowMode = ref<AppWindowMode>("main");
const transitionPhase = ref<WindowTransitionPhase>("idle");
const isSwitchingWindowMode = ref(false);
const systemClipboardItems = ref<ClipboardPreviewItem[]>([]);
let clipboardHistoryTimer: number | undefined;

const clipboardItems = computed(() =>
  getFloatingClipboardItems(systemClipboardItems.value, historyStore.items),
);
const latencyLabel = computed(() =>
  getLatencyLabel({
    running: statusStore.status.running,
    connectedCount: statusStore.status.connectedCount,
  }),
);
const isFloating = computed(() => windowMode.value === "floating");

watch(
  windowMode,
  (mode) => {
    document.documentElement.dataset.windowMode = mode;
    document.body.dataset.windowMode = mode;
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  delete document.documentElement.dataset.windowMode;
  delete document.body.dataset.windowMode;
  window.clearInterval(clipboardHistoryTimer);
});

function wait(ms: number) {
  return new Promise((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

function waitForPaint() {
  return new Promise((resolve) => {
    window.requestAnimationFrame(() => resolve(undefined));
  });
}

async function refreshSystemClipboardHistory() {
  try {
    systemClipboardItems.value = await getClipboardHistory();
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

async function switchWindowMode(nextMode: AppWindowMode, resizeWindow: () => Promise<void>) {
  if (isSwitchingWindowMode.value) {
    return;
  }

  const transition = getWindowModeTransition(windowMode.value, nextMode);
  if (!transition) {
    return;
  }

  isSwitchingWindowMode.value = true;
  transitionPhase.value = transition.exitPhase;

  try {
    await wait(WINDOW_MODE_EXIT_MS);
    await resizeWindow();
    windowMode.value = nextMode;
    transitionPhase.value = transition.enterPhase;
    await waitForPaint();
    await wait(WINDOW_MODE_ENTER_MS);
  } catch (error) {
    console.error(error);
  } finally {
    transitionPhase.value = "idle";
    isSwitchingWindowMode.value = false;
  }
}

async function switchToFloatingMode() {
  await switchWindowMode("floating", enterFloatingWindow);
}

async function switchToMainMode() {
  await switchWindowMode("main", restoreMainWindow);
}
</script>

<template>
  <div
    class="app-window-shell flex h-screen flex-col overflow-hidden rounded-[18px] text-slate-100 transition-[background-color,border-color,padding] duration-200 ease-out"
    :class="[
      isFloating ? 'bg-transparent p-2' : 'border border-[color:var(--main-line)] bg-[color:var(--main-bg)]',
      `window-phase-${transitionPhase}`,
      isSwitchingWindowMode ? 'pointer-events-none' : '',
    ]"
  >
    <Transition name="window-panel" mode="out-in">
      <FloatingPanel
        v-if="isFloating"
        :status-label="statusStore.statusLabel"
        :running="statusStore.status.running"
        :connected-count="statusStore.status.connectedCount"
        :latency-label="latencyLabel"
        :clipboard-items="clipboardItems"
        @restore="switchToMainMode"
        @close="closeWindow"
      />

      <div v-else class="main-window-content flex min-h-0 flex-1 flex-col overflow-hidden">
        <WindowTitleBar />
        <div class="flex min-h-0 flex-1 overflow-hidden">
          <Sidebar />
          <main class="flex min-w-0 flex-1 flex-col">
            <TitleBar
              :switching-window-mode="isSwitchingWindowMode"
              @switch-floating="switchToFloatingMode"
            />
            <div class="min-h-0 flex-1 overflow-auto px-6 pb-6 pt-1.5">
              <RouterView />
            </div>
          </main>
        </div>
      </div>
    </Transition>
  </div>
</template>
