<script setup lang="ts">
const props = defineProps<{
  modelValue: boolean;
  disabled?: boolean;
  label: string;
  hint?: string;
  controlOnly?: boolean;
}>();

function updateValue(event: Event) {
  const input = event.target as HTMLInputElement;
  emit("update:modelValue", input.checked);
  input.checked = props.modelValue;
}

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
}>();
</script>

<template>
  <label
    v-if="controlOnly"
    class="relative inline-flex min-h-8 min-w-11 shrink-0 items-center justify-center"
    :class="disabled ? 'cursor-not-allowed' : 'cursor-pointer'"
    :aria-label="label"
    @click.stop
    @pointerdown.stop
  >
    <input
      class="peer sr-only"
      type="checkbox"
      role="switch"
      :aria-label="label"
      :checked="modelValue"
      :disabled="disabled"
      @change="updateValue"
    />
    <span
      class="switch-track"
      aria-hidden="true"
    >
      <span class="switch-thumb" />
    </span>
  </label>

  <label
    v-else
    class="relative flex items-center justify-between gap-4 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] px-4 py-3"
    :class="disabled ? 'cursor-not-allowed' : 'cursor-pointer'"
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
      role="switch"
      :aria-label="label"
      :checked="modelValue"
      :disabled="disabled"
      @change="updateValue"
    />
    <span
      class="switch-track"
      aria-hidden="true"
    >
      <span class="switch-thumb" />
    </span>
  </label>
</template>

<style scoped>
.switch-track {
  --switch-active: #5bf293;
  position: relative;
  width: 42px;
  height: 24px;
  flex-shrink: 0;
  border: 1px solid var(--main-line);
  border-radius: 999px;
  background: var(--field-bg);
  transition: background-color 180ms ease, border-color 180ms ease, opacity 180ms ease;
}

.switch-thumb {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--subtle-text);
  box-shadow: 0 1px 2px rgb(0 0 0 / 16%);
  transition: transform 180ms ease, background-color 180ms ease;
}

.peer:checked + .switch-track {
  border-color: color-mix(in srgb, var(--switch-active) 75%, transparent);
  background: color-mix(in srgb, var(--switch-active) 35%, var(--field-bg));
}

.peer:checked + .switch-track .switch-thumb {
  transform: translateX(18px);
  background: var(--switch-active);
}

.switch-thumb::after {
  content: "";
  position: absolute;
  top: 4px;
  left: 6px;
  width: 4px;
  height: 7px;
  border: solid var(--field-bg);
  border-width: 0 1.5px 1.5px 0;
  transform: rotate(45deg);
  opacity: 0;
  transition: opacity 180ms ease;
}

.peer:checked + .switch-track .switch-thumb::after {
  opacity: 1;
}

.peer:focus-visible + .switch-track {
  outline: 2px solid var(--clipboard-card-text);
  outline-offset: 3px;
}

.peer:not(:disabled) + .switch-track:hover {
  border-color: var(--muted-text);
}

.peer:checked:not(:disabled) + .switch-track:hover {
  border-color: var(--switch-active);
}

.peer:disabled + .switch-track {
  opacity: 0.45;
}

@media (prefers-reduced-motion: reduce) {
  .switch-track,
  .switch-thumb,
  .switch-thumb::after {
    transition: none;
  }
}
</style>
