<script setup lang="ts">
import Card from "@/components/ui/Card.vue";

defineProps<{
  label: string;
  value: string | number;
  hint?: string;
}>();
</script>

<template>
  <Card compact>
    <p class="text-xs font-medium text-slate-400">{{ label }}</p>
    <p class="mt-3 break-words text-2xl font-semibold text-white">{{ value }}</p>
    <p v-if="hint" class="mt-2 text-xs text-slate-500">{{ hint }}</p>
  </Card>
</template>
