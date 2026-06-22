<script setup lang="ts">
import { ref } from "vue";
import { Link } from "lucide-vue-next";

import Button from "@/components/ui/Button.vue";
import { clampPort } from "@/lib/format";

const props = withDefaults(
  defineProps<{
    loading?: boolean;
    defaultPort?: number;
  }>(),
  {
    loading: false,
    defaultPort: 8765,
  },
);

const emit = defineEmits<{
  connect: [ip: string, port: number];
}>();

const ip = ref("");
const port = ref(props.defaultPort);

function submit() {
  const nextIp = ip.value.trim();
  if (!nextIp) {
    return;
  }
  emit("connect", nextIp, clampPort(port.value));
}
</script>

<template>
  <form class="grid gap-3 md:grid-cols-[1fr_120px_auto]" @submit.prevent="submit">
    <label class="min-w-0">
      <span class="mb-2 block text-xs font-medium text-slate-400">输入对方 IP</span>
      <input
        v-model="ip"
        class="h-10 w-full rounded-md border border-[color:var(--main-line-soft)] bg-[rgba(19,34,63,0.72)] px-3 text-sm text-white placeholder:text-slate-600"
        placeholder="192.168.1.20"
      />
    </label>
    <label>
      <span class="mb-2 block text-xs font-medium text-slate-400">端口</span>
      <input
        v-model.number="port"
        class="h-10 w-full rounded-md border border-[color:var(--main-line-soft)] bg-[rgba(19,34,63,0.72)] px-3 text-sm text-white"
        type="number"
        min="1"
        max="65535"
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
