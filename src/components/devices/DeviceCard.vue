<script setup lang="ts">
import { ShieldCheck, ShieldQuestion, Unplug } from "lucide-vue-next";

import Button from "@/components/ui/Button.vue";
import { deviceAddress, formatTime } from "@/lib/format";
import type { DeviceInfo } from "@/types/device";

defineProps<{
  device: DeviceInfo;
}>();

defineEmits<{
  disconnect: [deviceId: string];
  trust: [deviceId: string];
}>();
</script>

<template>
  <article class="rounded-lg border border-slate-700/80 bg-slate-950/50 p-4">
    <div class="flex items-start justify-between gap-4">
      <div class="min-w-0">
        <h3 class="truncate text-sm font-semibold text-white">{{ device.name }}</h3>
        <p class="mt-1 font-mono text-xs text-slate-400">{{ deviceAddress(device.ip, device.port) }}</p>
      </div>
      <span
        class="rounded-md border px-2 py-1 text-xs"
        :class="device.connected ? 'border-emerald-400/40 text-emerald-200' : 'border-slate-600 text-slate-400'"
      >
        {{ device.connected ? "已连接" : "未连接" }}
      </span>
    </div>
    <div class="mt-4 grid gap-2 text-xs text-slate-400">
      <p>信任状态：{{ device.trusted ? "已信任" : "等待确认" }}</p>
      <p>最后在线：{{ formatTime(device.lastSeenAt) }}</p>
    </div>
    <div class="mt-4 flex flex-wrap gap-2">
      <Button v-if="!device.trusted" size="sm" variant="secondary" @click="$emit('trust', device.id)">
        <ShieldQuestion class="h-4 w-4" />
        信任设备
      </Button>
      <Button v-else size="sm" variant="ghost">
        <ShieldCheck class="h-4 w-4" />
        已信任
      </Button>
      <Button size="sm" variant="ghost" :disabled="!device.connected" @click="$emit('disconnect', device.id)">
        <Unplug class="h-4 w-4" />
        断开
      </Button>
    </div>
  </article>
</template>
