<script setup lang="ts">
import { Link, List } from "lucide-vue-next";
import { ref } from "vue";

import Button from "@/components/ui/Button.vue";
import { clampPort } from "@/lib/format";

const emit = defineEmits<{
  "update:ip": [value: string];
  "update:port": [value: number];
  connect: [ip: string, port: number];
}>();

withDefaults(
  defineProps<{
    loading?: boolean;
    ip: string;
    port: number;
    recentIps?: string[];
  }>(),
  {
    loading: false,
    recentIps: () => [],
  },
);

const showRecentIps = ref(false);

function submit(ip: string, port: number) {
  const nextIp = ip.trim();
  if (!nextIp) {
    return;
  }
  emit("connect", nextIp, clampPort(port));
}

function selectRecentIp(recentIp: string) {
  emit("update:ip", recentIp);
  showRecentIps.value = false;
}
</script>

<template>
  <form class="grid gap-3 md:grid-cols-[1fr_120px_auto]" @submit.prevent="submit(ip, port)">
    <label class="relative min-w-0">
      <span class="mb-2 block text-xs font-medium text-slate-400">输入对方 IP</span>
      <input
        :value="ip"
        class="h-10 w-full rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 pr-11 text-sm text-white placeholder:text-slate-600"
        placeholder="192.168.1.20"
        @input="emit('update:ip', ($event.target as HTMLInputElement).value)"
      />
      <button
        data-recent-ip-button
        class="absolute bottom-1 right-1 grid h-8 w-8 place-items-center rounded-md border border-transparent text-slate-400 transition hover:border-[color:var(--main-line-soft)] hover:bg-[color:var(--main-bg-muted)] hover:text-white disabled:pointer-events-none disabled:opacity-35"
        type="button"
        aria-label="最近连接 IP"
        title="最近连接 IP"
        :disabled="!recentIps.length"
        @click="showRecentIps = !showRecentIps"
      >
        <List class="h-4 w-4" />
      </button>
      <div
        v-if="showRecentIps && recentIps.length"
        data-recent-ip-list
        class="absolute right-0 top-[calc(100%+6px)] z-20 w-56 overflow-hidden rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-soft)] p-1 shadow-[0_16px_42px_rgba(0,0,0,0.45)]"
      >
        <button
          v-for="recentIp in recentIps"
          :key="recentIp"
          class="block w-full truncate rounded-md px-3 py-2 text-left font-mono text-xs text-slate-200 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white"
          type="button"
          @click="selectRecentIp(recentIp)"
        >
          {{ recentIp }}
        </button>
      </div>
    </label>
    <label>
      <span class="mb-2 block text-xs font-medium text-slate-400">端口</span>
      <input
        :value="port"
        class="h-10 w-full rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 text-sm text-white"
        type="number"
        min="1"
        max="65535"
        @input="emit('update:port', Number(($event.target as HTMLInputElement).value))"
      />
    </label>
    <div class="flex items-end">
      <Button class="w-full" variant="primary" type="submit" :disabled="loading || !ip.trim()">
        <Link class="h-4 w-4" />
        连接
      </Button>
    </div>
  </form>
</template>
