<script setup lang="ts">
import { Check, Copy, TriangleAlert } from "lucide-vue-next";
import { computed, onBeforeUnmount, ref } from "vue";

import { copyTextToClipboard, getCopyableText, type CopyTextResult } from "@/lib/clipboard";

import Button from "./Button.vue";

const props = withDefaults(
  defineProps<{
    text: string | null | undefined;
    label?: string;
    copiedLabel?: string;
    size?: "sm" | "md";
    variant?: "secondary" | "ghost";
    iconOnly?: boolean;
  }>(),
  {
    label: "复制",
    copiedLabel: "已复制",
    size: "sm",
    variant: "ghost",
    iconOnly: false,
  },
);

const result = ref<CopyTextResult | null>(null);
let resetTimer: number | undefined;

const canCopy = computed(() => Boolean(getCopyableText(props.text)));
const hasError = computed(() => result.value === "failed" || result.value === "unsupported");
const buttonLabel = computed(() => {
  if (result.value === "copied") {
    return props.copiedLabel;
  }

  if (hasError.value) {
    return "复制失败";
  }

  return props.label;
});

async function copyText() {
  result.value = await copyTextToClipboard(props.text);

  window.clearTimeout(resetTimer);
  resetTimer = window.setTimeout(() => {
    result.value = null;
  }, 1400);
}

onBeforeUnmount(() => {
  window.clearTimeout(resetTimer);
});
</script>

<template>
  <button
    v-if="iconOnly"
    class="grid h-6 w-6 shrink-0 place-items-center rounded-md bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-[color:var(--floating-control-bg-hover)] disabled:cursor-not-allowed disabled:opacity-45"
    type="button"
    :disabled="!canCopy"
    :aria-label="buttonLabel"
    :title="buttonLabel"
    data-window-control
    @click.stop="copyText"
  >
    <Check v-if="result === 'copied'" class="h-3.5 w-3.5" />
    <TriangleAlert v-else-if="hasError" class="h-3.5 w-3.5" />
    <Copy v-else class="h-3.5 w-3.5" />
  </button>

  <Button v-else :size="size" :variant="variant" :disabled="!canCopy" @click.stop="copyText">
    <Check v-if="result === 'copied'" class="h-4 w-4" />
    <TriangleAlert v-else-if="hasError" class="h-4 w-4" />
    <Copy v-else class="h-4 w-4" />
    {{ buttonLabel }}
  </Button>
</template>
