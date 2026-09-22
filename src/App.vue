<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { useRoute } from "vue-router";

import AppShell from "@/components/layout/AppShell.vue";
import FirstRunWizard from "@/components/onboarding/FirstRunWizard.vue";
import Button from "@/components/ui/Button.vue";
import ToastStack from "@/components/ui/ToastStack.vue";
import { APP_VERSION } from "@/lib/about";
import { useUpdaterStore } from "@/stores/updater";
import { onAppEvent, restoreMainWindow, showMainWindow } from "@/lib/tauri";
import router from "@/router";
import { useActivityLogStore } from "@/stores/activityLog";
import { useConfigStore } from "@/stores/config";
import { useDevicesStore } from "@/stores/devices";
import { useHistoryStore } from "@/stores/history";
import { useShortcutStore } from "@/stores/shortcuts";
import { useStatusStore } from "@/stores/status";
import { useToastStore } from "@/stores/toasts";

const statusStore = useStatusStore();
const activityLogStore = useActivityLogStore();
const devicesStore = useDevicesStore();
const configStore = useConfigStore();
const historyStore = useHistoryStore();
const shortcutStore = useShortcutStore();
const toastStore = useToastStore();
const route = useRoute();
const STARTUP_OVERLAY_MIN_MS = 900;
const ALLOWED_NAVIGATION_ROUTES = new Set([
  "/",
  "/devices",
  "/mobile",
  "/logs",
  "/settings",
  "/about",
]);
const updater = useUpdaterStore();
const startupUpdate = ref<string | null>(null);
const isMediaPreviewRoute = computed(() => isUtilityWindowRoute(route.path) || isUtilityWindowStartupBypassed());
const startupVisible = ref(!isUtilityWindowStartupBypassed());
const startupAnimationVisible = ref(false);
let resolveStartupAnimation: () => void;
const startupAnimationComplete = new Promise<void>((resolve) => { resolveStartupAnimation = resolve; });
let resolveStartupWindow: () => void;
const startupWindowReady = new Promise<void>((resolve) => { resolveStartupWindow = resolve; });
const onboardingVisible = computed(() =>
  !startupVisible.value
  && !isMediaPreviewRoute.value
  && !configStore.config.onboardingCompleted,
);
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
  if (!ALLOWED_NAVIGATION_ROUTES.has(route)) {
    return;
  }

  void router.push(route);
}

function dismissStartupUpdate() {
  startupUpdate.value = null;
}

async function openStartupUpdate() {
  startupUpdate.value = null;
  await router.push("/about");
}

async function finishStartupOverlay(startedAt: number) {
  const elapsed = performance.now() - startedAt;
  const remaining = Math.max(STARTUP_OVERLAY_MIN_MS - elapsed, 0);

  if (remaining > 0) {
    await wait(remaining);
  }

  startupAnimationVisible.value = false;
  await startupWindowReady;
  startupVisible.value = false;
}

onMounted(async () => {
  if (isMediaPreviewRoute.value) {
    startupVisible.value = false;
    return;
  }

  let startedAt = performance.now();

  try {
    if ("__TAURI_INTERNALS__" in window) {
      try {
        await restoreMainWindow();
      } catch (error) {
        console.error("failed to center startup animation", error);
      }
      await showMainWindow().catch(console.error);
    }
    startupAnimationVisible.value = true;
    await nextTick();
    startedAt = performance.now();
    await Promise.all([
      statusStore.refresh(),
      configStore.refresh(),
      historyStore.refresh(),
    ]);
    activityLogStore.initialize(historyStore.items, statusStore.status);
    void devicesStore.refresh();
    void shortcutStore.apply(configStore.config).then((shortcutResult) => {
      if (!shortcutResult.ok) {
        console.error("global shortcut registration failed", shortcutResult.error);
        toastStore.error("快捷键注册失败，请在设置中更换组合键");
      }
    }).catch((error) => {
      console.error("global shortcut registration failed", error);
      toastStore.error("快捷键注册失败，请在设置中更换组合键");
    });
    void Promise.allSettled([
      statusStore.subscribe(),
      devicesStore.subscribe(),
      configStore.subscribe(),
      historyStore.subscribe(),
      activityLogStore.subscribe(),
      onAppEvent<string>("navigate-to-page", handlePageNavigation).then((unlisten) => {
        navigateUnlisten?.();
        navigateUnlisten = unlisten;
      }),
    ]).then((results) => {
      for (const result of results) {
        if (result.status === "rejected") console.error("startup subscription failed", result.reason);
      }
    });
    void updater.checkForUpdate(true).then(() => {
      if (updater.phase === "available") startupUpdate.value = updater.version;
    });
  } finally {
    await finishStartupOverlay(startedAt);
  }
});

onBeforeUnmount(() => {
  navigateUnlisten?.();
  navigateUnlisten = undefined;
  void shortcutStore.dispose();
  activityLogStore.dispose();
});
</script>

<template>
  <RouterView v-if="isMediaPreviewRoute" />
  <AppShell v-else v-show="!startupVisible && !onboardingVisible" :startup-animation-complete="startupAnimationComplete" @ready="resolveStartupWindow" />
  <FirstRunWizard v-if="onboardingVisible" />
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
          当前版本 v{{ APP_VERSION }}，最新版本 v{{ startupUpdate }}
          可在软件内下载并安装更新。
        </p>
        <div class="mt-5 flex justify-end gap-3">
          <Button data-update-dismiss-button variant="secondary" @click="dismissStartupUpdate">
            稍后
          </Button>
          <Button data-update-open-button variant="primary" @click="openStartupUpdate">
            立即查看
          </Button>
        </div>
      </section>
    </div>
  </Transition>
  <Transition name="startup-overlay" @after-leave="resolveStartupAnimation">
    <div
      v-if="startupAnimationVisible && !isMediaPreviewRoute"
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
