<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "sm" | "md";
    type?: "button" | "submit";
    disabled?: boolean;
  }>(),
  {
    variant: "secondary",
    size: "md",
    type: "button",
    disabled: false,
  },
);

const classes = computed(() => [
  "inline-flex items-center justify-center gap-2 rounded-md border font-medium transition duration-150 active:scale-[0.98]",
  "disabled:cursor-not-allowed disabled:opacity-50",
  props.size === "sm" ? "h-8 px-3 text-xs" : "h-10 px-4 text-sm",
  props.variant === "primary" &&
    "border-[color:var(--button-primary-line)] bg-[color:var(--button-primary-bg)] text-[color:var(--button-primary-text)] hover:bg-[color:var(--button-primary-bg-hover)]",
  props.variant === "secondary" &&
    "border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-soft)] text-slate-100 hover:border-[color:var(--main-line)] hover:bg-[color:var(--main-bg-muted)]",
  props.variant === "ghost" &&
    "border-transparent bg-transparent text-slate-300 hover:bg-[color:var(--main-bg-muted)] hover:text-white",
  props.variant === "danger" &&
    "border-red-500/50 bg-red-500/12 text-red-100 hover:bg-red-500/20",
]);
</script>

<template>
  <button :type="type" :disabled="disabled" :class="classes">
    <slot />
  </button>
</template>
