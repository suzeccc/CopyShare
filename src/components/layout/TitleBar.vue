<script setup lang="ts">
import ConnectionBadge from "@/components/status/ConnectionBadge.vue";
import Button from "@/components/ui/Button.vue";
import RefreshButton from "@/components/ui/RefreshButton.vue";
import { useStatusStore } from "@/stores/status";

const statusStore = useStatusStore();
withDefaults(
  defineProps<{
    switchingWindowMode?: boolean;
  }>(),
  {
    switchingWindowMode: false,
  },
);

const emit = defineEmits<{
  (event: "switch-floating"): void;
}>();
</script>

<template>
  <header class="flex h-16 items-center justify-between border-b border-[color:var(--main-line)] bg-[color:var(--main-bg)] px-6">
    <div class="min-w-0">
      <p class="text-xs text-slate-500">桌面同步控制台</p>
      <p class="truncate text-sm font-medium text-slate-200">
        {{ statusStore.status.deviceName }} · {{ statusStore.status.localIp || "等待网络地址" }}
      </p>
    </div>
    <div class="flex items-center gap-3">
      <ConnectionBadge :state="statusStore.status.state" :label="statusStore.statusLabel" />
      <RefreshButton :refresh="() => statusStore.refresh()" />
      <Button
        variant="ghost"
        size="sm"
        :disabled="switchingWindowMode"
        @click="emit('switch-floating')"
      >
        {{ switchingWindowMode ? "切换中" : "切换浮窗" }}
      </Button>
    </div>
  </header>
</template>
