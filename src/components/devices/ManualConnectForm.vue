<script setup lang="ts">
import { Link } from "lucide-vue-next";

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
  }>(),
  {
    loading: false,
  },
);

function submit(ip: string, port: number) {
  const nextIp = ip.trim();
  if (!nextIp) {
    return;
  }
  emit("connect", nextIp, clampPort(port));
}
</script>

<template>
  <form class="grid gap-3 md:grid-cols-[1fr_120px_auto]" @submit.prevent="submit(ip, port)">
    <label class="min-w-0">
      <span class="mb-2 block text-xs font-medium text-slate-400">输入对方 IP</span>
      <input
        :value="ip"
        class="h-10 w-full rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 text-sm text-white placeholder:text-slate-600"
        placeholder="192.168.1.20"
        @input="emit('update:ip', ($event.target as HTMLInputElement).value)"
      />
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
