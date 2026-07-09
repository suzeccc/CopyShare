<script setup lang="ts">
defineProps<{
  modelValue: boolean;
  disabled?: boolean;
  label: string;
  hint?: string;
  controlOnly?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
}>();
</script>

<template>
  <label
    v-if="controlOnly"
    class="relative inline-flex shrink-0 items-center"
    :aria-label="label"
    @click.stop
    @pointerdown.stop
  >
    <input
      class="peer sr-only"
      type="checkbox"
      :checked="modelValue"
      :disabled="disabled"
      @change="emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
    />
    <span
      class="relative h-6 w-10 shrink-0 rounded-full border border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-muted)] transition peer-checked:border-[#35d366] peer-checked:bg-[#35d366] peer-disabled:opacity-50"
      aria-hidden="true"
    >
      <span
        class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition"
        :class="modelValue ? 'translate-x-4' : 'translate-x-0'"
      />
    </span>
  </label>

  <label
    v-else
    class="relative flex items-center justify-between gap-4 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] px-4 py-3"
    @click.stop
    @pointerdown.stop
  >
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
      class="relative h-6 w-11 shrink-0 rounded-full border border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-muted)] transition peer-checked:border-[#35d366] peer-checked:bg-[#35d366] peer-disabled:opacity-50"
      aria-hidden="true"
    >
      <span
        class="absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition"
        :class="modelValue ? 'translate-x-5' : 'translate-x-0'"
      />
    </span>
  </label>
</template>
