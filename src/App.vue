<script setup lang="ts">
import { onMounted } from "vue";

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

onMounted(async () => {
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
});
</script>

<template>
  <AppShell />
  <ToastStack />
</template>
