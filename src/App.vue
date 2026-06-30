<script setup lang="ts">
import { onMounted, ref } from "vue";

import AppShell from "@/components/layout/AppShell.vue";
import ToastStack from "@/components/ui/ToastStack.vue";
import { useConfigStore } from "@/stores/config";
import { useDevicesStore } from "@/stores/devices";
import { useHistoryStore } from "@/stores/history";
import { useStatusStore } from "@/stores/status";

const statusStore = useStatusStore();
const devicesStore = useDevicesStore();
const configStore = useConfigStore();
const historyStore = useHistoryStore();
const STARTUP_OVERLAY_MIN_MS = 900;
const startupVisible = ref(true);

function wait(ms: number) {
  return new Promise<void>((resolve) => {
    window.setTimeout(resolve, ms);
  });
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
  const startedAt = performance.now();

  try {
    await Promise.all([
      statusStore.refresh(),
      devicesStore.refresh(),
      configStore.refresh(),
      historyStore.refresh(),
    ]);
    await Promise.all([
      statusStore.subscribe(),
      devicesStore.subscribe(),
      configStore.subscribe(),
      historyStore.subscribe(),
    ]);
  } finally {
    await finishStartupOverlay(startedAt);
  }
});
</script>

<template>
  <AppShell />
  <ToastStack />
  <Transition name="startup-overlay">
    <div
      v-if="startupVisible"
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
