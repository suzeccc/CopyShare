<script setup lang="ts">
defineProps<{
  modelValue: boolean;
  disabled?: boolean;
  label: string;
  hint?: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
}>();
</script>

<template>
  <label class="flex items-center justify-between gap-4 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] px-4 py-3">
    <span class="min-w-0">
      <span class="block text-sm font-medium text-slate-100">{{ label }}</span>
      <span v-if="hint" class="mt-1 block text-xs text-slate-400">{{ hint }}</span>
    </span>
    <input
      class="peer sr-only"
      type="checkbox"
      :checked="modelValue"
      :disabled="disabled"
      @change="emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
    />
    <span
      class="relative h-6 w-11 shrink-0 rounded-full border border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-muted)] transition peer-checked:border-[color:var(--theme-accent)] peer-checked:bg-[color:var(--theme-accent)] peer-disabled:opacity-50"
      aria-hidden="true"
    >
      <span
        class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition"
        :class="modelValue ? 'translate-x-5' : 'translate-x-0'"
      />
    </span>
  </label>
</template>
