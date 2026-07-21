<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useRoute } from "vue-router";

import AppShell from "@/components/layout/AppShell.vue";
import Button from "@/components/ui/Button.vue";
import ToastStack from "@/components/ui/ToastStack.vue";
import { checkForAppUpdateOnStartup } from "@/lib/about";
import type { StartupUpdatePrompt } from "@/lib/about";
import { onAppEvent, openExternalUrl } from "@/lib/tauri";
import router from "@/router";
import { useConfigStore } from "@/stores/config";
import { useDevicesStore } from "@/stores/devices";
import { useHistoryStore } from "@/stores/history";
import { useShortcutStore } from "@/stores/shortcuts";
import { useStatusStore } from "@/stores/status";
import { useToastStore } from "@/stores/toasts";

const statusStore = useStatusStore();
const devicesStore = useDevicesStore();
const configStore = useConfigStore();
const historyStore = useHistoryStore();
const shortcutStore = useShortcutStore();
const toastStore = useToastStore();
const route = useRoute();
const STARTUP_OVERLAY_MIN_MS = 900;
const startupUpdate = ref<StartupUpdatePrompt | null>(null);
const isMediaPreviewRoute = computed(() => isUtilityWindowRoute(route.path) || isUtilityWindowStartupBypassed());
const startupVisible = ref(!isUtilityWindowStartupBypassed());
let navigateUnlisten: (() => void) | undefined;

function isUtilityWindowRoute(path: string) {
  return path === "/media-preview" || path === "/floating-clipboard";
}

function isUtilityWindowStartupBypassed() {
  return window.location.hash.startsWith("#/media-preview")
    || window.location.hash.startsWith("#/floating-clipboard");
}

function wait(ms: number) {
  return new Promise<void>((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

function handlePageNavigation(route: string) {
  const allowedRoutes = new Set(["/", "/devices", "/mobile", "/logs", "/settings", "/about"]);
  if (!allowedRoutes.has(route)) {
    return;
  }

  void router.push(route);
}

function dismissStartupUpdate() {
  startupUpdate.value = null;
}

async function openStartupUpdateRelease() {
  if (!startupUpdate.value) {
    return;
  }

  const updateUrl = startupUpdate.value.updateUrl;
  startupUpdate.value = null;

  try {
    await openExternalUrl(updateUrl);
  } catch (error) {
    toastStore.error(error instanceof Error ? error.message : "打开更新页面失败");
  }
}

async function finishStartupOverlay(startedAt: number) {
  const elapsed = performance.now() - startedAt;
  const remaining = Math.max(STARTUP_OVERLAY_MIN_MS - elapsed, 0);

  if (remaining > 0) {
    await wait(remaining);
  }

  startupVisible.value = false;
}

onMounted(async () => {
  if (isMediaPreviewRoute.value) {
    startupVisible.value = false;
    return;
  }

  const startedAt = performance.now();

  try {
    await Promise.all([
      statusStore.refresh(),
      devicesStore.refresh(),
      configStore.refresh(),
      historyStore.refresh(),
    ]);
    const shortcutResult = await shortcutStore.apply(configStore.config);
    if (!shortcutResult.ok) {
      console.error("global shortcut registration failed", shortcutResult.error);
      toastStore.error("快捷键注册失败，请在设置中更换组合键");
    }
    await Promise.all([
      statusStore.subscribe(),
      devicesStore.subscribe(),
      configStore.subscribe(),
      historyStore.subscribe(),
    ]);
    navigateUnlisten?.();
    navigateUnlisten = await onAppEvent<string>("navigate-to-page", handlePageNavigation);
    void checkForAppUpdateOnStartup((update) => {
      startupUpdate.value = update;
    });
  } finally {
    await finishStartupOverlay(startedAt);
  }
});

onBeforeUnmount(() => {
  navigateUnlisten?.();
  navigateUnlisten = undefined;
  void shortcutStore.dispose();
});
</script>

<template>
  <RouterView v-if="isMediaPreviewRoute" />
  <AppShell v-else />
  <ToastStack />
  <Transition name="trust-prompt">
    <div
      v-if="startupUpdate"
      data-update-startup-dialog
      class="fixed inset-0 z-[130] flex items-center justify-center bg-[color:var(--dialog-overlay-bg)] px-6 backdrop-blur-sm"
    >
      <section
        class="w-full max-w-[430px] rounded-xl border border-[color:var(--main-line)] bg-[color:var(--dialog-bg)] p-5 shadow-[0_24px_80px_rgba(0,0,0,0.5)]"
      >
        <p class="text-lg font-semibold text-white">发现新版本</p>
        <p class="mt-2 text-sm leading-6 text-[color:var(--muted-text)]">
          当前版本 v{{ startupUpdate.currentVersion }}，最新版本 v{{ startupUpdate.latestVersion }}。
          是否前往发布页下载？
        </p>
        <div class="mt-5 flex justify-end gap-3">
          <Button data-update-dismiss-button variant="secondary" @click="dismissStartupUpdate">
            稍后
          </Button>
          <Button data-update-open-release-button variant="primary" @click="openStartupUpdateRelease">
            立即查看
          </Button>
        </div>
      </section>
    </div>
  </Transition>
  <Transition name="startup-overlay">
    <div
      v-if="startupVisible && !isMediaPreviewRoute"
      data-startup-overlay
      class="startup-overlay"
      role="status"
      aria-live="polite"
      aria-label="CopyShare 正在准备同步"
    >
      <section class="startup-card">
        <div class="startup-logo" aria-hidden="true">
          <span class="startup-logo-link"></span>
        </div>
        <p class="startup-title">CopyShare</p>
        <p class="startup-subtitle">正在准备同步</p>
        <div class="startup-progress" aria-hidden="true"></div>
      </section>
    </div>
  </Transition>
</template>
