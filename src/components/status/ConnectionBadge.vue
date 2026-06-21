<script setup lang="ts">
import type { SyncState } from "@/types/status";

defineProps<{
  state: SyncState;
  label: string;
}>();
</script>

<template>
  <span
    class="inline-flex items-center gap-2 rounded-md border px-2.5 py-1 text-xs font-medium"
    :class="{
      'border-emerald-400/50 bg-emerald-400/10 text-emerald-200': state === 'running',
      'border-amber-400/50 bg-amber-400/10 text-amber-100': state === 'stopped',
      'border-red-400/50 bg-red-400/10 text-red-100': state === 'error',
    }"
  >
    <span
      class="h-2 w-2 rounded-full"
      :class="{
        'bg-emerald-300': state === 'running',
        'bg-amber-300': state === 'stopped',
        'bg-red-300': state === 'error',
      }"
    />
    {{ label }}
  </span>
</template>
