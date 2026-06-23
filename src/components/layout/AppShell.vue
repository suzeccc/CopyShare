<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { ShieldCheck, ShieldQuestion, ShieldX } from "lucide-vue-next";

import Button from "@/components/ui/Button.vue";
import FloatingPanel from "@/components/layout/FloatingPanel.vue";
import Sidebar from "@/components/layout/Sidebar.vue";
import TitleBar from "@/components/layout/TitleBar.vue";
import WindowTitleBar from "@/components/layout/WindowTitleBar.vue";
import { deviceAddress } from "@/lib/format";
import { getFloatingClipboardItems, type ClipboardPreviewItem } from "@/lib/historyPreview";
import {
  closeWindow,
  enterFloatingWindow,
  getClipboardHistory,
  hideMainWindow,
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
import { useConfigStore } from "@/stores/config";
import { useDevicesStore } from "@/stores/devices";
import { useStatusStore } from "@/stores/status";

const statusStore = useStatusStore();
const configStore = useConfigStore();
const historyStore = useHistoryStore();
const devicesStore = useDevicesStore();
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
const trustPromptDevice = computed(() => devicesStore.pendingTrust[0] ?? null);
const trustPromptExtraCount = computed(() =>
  Math.max(devicesStore.pendingTrust.length - 1, 0),
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

onBeforeUnmount(() => {
  delete document.documentElement.dataset.windowMode;
  delete document.body.dataset.windowMode;
  delete document.documentElement.dataset.appTheme;
  delete document.body.dataset.appTheme;
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
        @hide="hideMainWindow"
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
