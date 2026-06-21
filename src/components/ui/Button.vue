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
  "inline-flex items-center justify-center gap-2 rounded-md border font-medium transition",
  "disabled:cursor-not-allowed disabled:opacity-50",
  props.size === "sm" ? "h-8 px-3 text-xs" : "h-10 px-4 text-sm",
  props.variant === "primary" &&
    "border-blue-500 bg-blue-600 text-white hover:bg-blue-500",
  props.variant === "secondary" &&
    "border-slate-600 bg-slate-800/80 text-slate-100 hover:border-slate-500 hover:bg-slate-700",
  props.variant === "ghost" &&
    "border-transparent bg-transparent text-slate-300 hover:bg-slate-800 hover:text-white",
  props.variant === "danger" &&
    "border-red-500/50 bg-red-500/12 text-red-100 hover:bg-red-500/20",
]);
</script>

<template>
  <button :type="type" :disabled="disabled" :class="classes">
    <slot />
  </button>
</template>
