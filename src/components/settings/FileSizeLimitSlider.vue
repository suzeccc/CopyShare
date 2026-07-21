<script setup lang="ts">
import { Download, Upload } from "lucide-vue-next";
import { computed, onBeforeUnmount } from "vue";

import {
  FILE_SIZE_LIMIT_MAX_MIB,
  FILE_SIZE_LIMIT_MIN_MIB,
  adjustFileSizeLimitFromWheel,
  clampFileSizeLimitMib,
  formatFileSizeLimit,
} from "@/lib/fileSizeLimit";

const props = withDefaults(
  defineProps<{
    modelValue: number;
    direction: "send" | "receive";
    disabled?: boolean;
  }>(),
  {
    disabled: false,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: number];
  commit: [value: number];
}>();

const directionLabel = computed(() => (props.direction === "send" ? "发送上限" : "接收上限"));
const directionHint = computed(() => (props.direction === "send" ? "本机发出" : "本机接收"));
const directionIcon = computed(() => (props.direction === "send" ? Upload : Download));
const normalizedValue = computed(() => clampFileSizeLimitMib(props.modelValue));
const formattedValue = computed(() => formatFileSizeLimit(normalizedValue.value));
const rangeStyle = computed(() => ({
  "--file-size-progress": `${
    ((normalizedValue.value - FILE_SIZE_LIMIT_MIN_MIB) /
      (FILE_SIZE_LIMIT_MAX_MIB - FILE_SIZE_LIMIT_MIN_MIB)) *
    100
  }%`,
}));

let wheelCommitTimer: number | undefined;
let pendingWheelValue: number | null = null;

function clearWheelCommit() {
  if (wheelCommitTimer !== undefined) {
    window.clearTimeout(wheelCommitTimer);
    wheelCommitTimer = undefined;
  }
}

function inputValue(event: Event) {
  return clampFileSizeLimitMib(Number((event.target as HTMLInputElement).value));
}

function onInput(event: Event) {
  clearWheelCommit();
  pendingWheelValue = null;
  emit("update:modelValue", inputValue(event));
}

function onChange(event: Event) {
  clearWheelCommit();
  pendingWheelValue = null;
  const value = inputValue(event);
  emit("update:modelValue", value);
  emit("commit", value);
}

function onWheel(event: WheelEvent) {
  if (props.disabled || event.deltaY === 0) return;

  event.preventDefault();
  const currentValue = pendingWheelValue ?? normalizedValue.value;
  const nextValue = adjustFileSizeLimitFromWheel(currentValue, event.deltaY);
  if (nextValue === currentValue) return;

  pendingWheelValue = nextValue;
  emit("update:modelValue", nextValue);
  clearWheelCommit();
  wheelCommitTimer = window.setTimeout(() => {
    wheelCommitTimer = undefined;
    const value = pendingWheelValue ?? nextValue;
    pendingWheelValue = null;
    emit("commit", value);
  }, 250);
}

onBeforeUnmount(clearWheelCommit);
</script>

<template>
  <div
    data-file-size-limit-slider
    class="grid gap-2.5 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-soft)] px-3 py-3"
    :class="disabled ? 'opacity-60' : ''"
  >
    <div class="flex items-center justify-between gap-3">
      <span class="flex min-w-0 items-center gap-2">
        <span
          class="grid h-7 w-7 shrink-0 place-items-center rounded-md border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]"
          aria-hidden="true"
        >
          <component :is="directionIcon" :size="15" :stroke-width="2" />
        </span>
        <span class="min-w-0">
          <span class="block text-[13px] font-bold text-slate-100">{{ directionLabel }}</span>
          <span class="block text-[12px] text-[color:var(--muted-text)]">{{ directionHint }}</span>
        </span>
      </span>
      <span
        class="min-w-[72px] rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-2 py-1 text-center font-mono text-[12px] font-bold text-slate-100"
      >
        {{ formattedValue }}
      </span>
    </div>

    <input
      class="file-size-limit-range w-full"
      type="range"
      :min="FILE_SIZE_LIMIT_MIN_MIB"
      :max="FILE_SIZE_LIMIT_MAX_MIB"
      step="1"
      :value="normalizedValue"
      :disabled="disabled"
      :style="rangeStyle"
      :aria-label="`${directionLabel}，单文件大小上限`"
      :aria-valuetext="formattedValue"
      @input="onInput"
      @change="onChange"
      @wheel="onWheel"
    />
    <div class="flex justify-between font-mono text-[11px] text-[color:var(--subtle-text)]" aria-hidden="true">
      <span>100 MiB</span>
      <span>2 GiB</span>
    </div>
  </div>
</template>

<style scoped>
.file-size-limit-range {
  height: 18px;
  cursor: pointer;
  appearance: none;
  background: transparent;
}

.file-size-limit-range:disabled {
  cursor: not-allowed;
}

.file-size-limit-range::-webkit-slider-runnable-track {
  height: 4px;
  border-radius: 999px;
  background: linear-gradient(
    to right,
    #35d366 0 var(--file-size-progress),
    var(--main-bg-muted) var(--file-size-progress) 100%
  );
}

.file-size-limit-range::-webkit-slider-thumb {
  width: 16px;
  height: 16px;
  margin-top: -6px;
  appearance: none;
  border: 2px solid #35d366;
  border-radius: 999px;
  background: #ffffff;
  box-shadow: 0 0 0 3px rgb(53 211 102 / 14%);
}

.file-size-limit-range:focus-visible {
  outline: 2px solid var(--accent-line);
  outline-offset: 2px;
  border-radius: 999px;
}

.file-size-limit-range::-moz-range-track {
  height: 4px;
  border-radius: 999px;
  background: var(--main-bg-muted);
}

.file-size-limit-range::-moz-range-progress {
  height: 4px;
  border-radius: 999px;
  background: #35d366;
}

.file-size-limit-range::-moz-range-thumb {
  width: 13px;
  height: 13px;
  border: 2px solid #35d366;
  border-radius: 999px;
  background: #ffffff;
  box-shadow: 0 0 0 3px rgb(53 211 102 / 14%);
}
</style>
