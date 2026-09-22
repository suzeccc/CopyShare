<script setup lang="ts">
import RefreshCw from "lucide-vue-next/dist/esm/icons/refresh-cw.js";
import Smartphone from "lucide-vue-next/dist/esm/icons/smartphone.js";
import { computed } from "vue";

import Button from "@/components/ui/Button.vue";
import { formatTime } from "@/lib/format";
import type { MobileDeviceHistoryItem } from "@/types/mobile";

const props = defineProps<{ device: MobileDeviceHistoryItem }>();
defineEmits<{ reconnect: [] }>();
const status = computed(() => props.device.status ?? "offline");
const statusLabel = computed(() => ({ connected: "已连接", waiting: "待连接", offline: "已离线" })[status.value]);
</script>

<template>
  <article class="rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] p-4 transition duration-200 hover:border-[color:var(--main-line)]">
    <div class="flex items-start justify-between gap-4">
      <div class="min-w-0">
        <h3 class="truncate text-sm font-semibold text-white">{{ device.name }}</h3>
        <p class="mt-1 font-mono text-xs text-slate-400">二维码连接</p>
      </div>
      <span
        class="inline-flex h-8 shrink-0 items-center gap-2 rounded-md border px-2.5 text-xs font-medium"
        :class="status === 'connected'
          ? 'border-emerald-300/45 bg-emerald-400/10 text-emerald-50'
          : status === 'waiting'
            ? 'border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]'
            : 'border-white/35 bg-white/[0.08] text-white'"
      >
        <span
          class="h-2 w-2 rounded-full"
          :class="status === 'connected' ? 'bg-emerald-400' : status === 'waiting' ? 'bg-[color:var(--accent-text)]' : 'bg-white'"
        />
        {{ statusLabel }}
      </span>
    </div>

    <div class="mt-4 grid gap-2 text-xs leading-5 text-slate-400">
      <p class="flex items-start gap-2 text-slate-300">
        <Smartphone class="h-4 w-4 shrink-0 text-[color:var(--accent-text)]" />
        <span>最近通过二维码连接过手机</span>
      </p>
      <p>最后在线：{{ formatTime(device.lastSeenAt) }}</p>
    </div>

    <div class="mt-4 flex flex-wrap gap-2">
      <Button size="sm" variant="secondary" @click="$emit('reconnect')">
        <RefreshCw class="h-4 w-4" />
        重新连接
      </Button>
    </div>
  </article>
</template>
