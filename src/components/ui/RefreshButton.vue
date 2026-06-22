<script setup lang="ts">
import { RefreshCw } from "lucide-vue-next";
import { computed, onBeforeUnmount, ref } from "vue";

import Button from "@/components/ui/Button.vue";
import {
  getRefreshFeedbackView,
  type RefreshFeedbackState,
} from "@/lib/refreshFeedback";

const props = withDefaults(
  defineProps<{
    refresh: () => Promise<unknown> | unknown;
    size?: "sm" | "md";
    variant?: "secondary" | "ghost";
  }>(),
  {
    size: "sm",
    variant: "ghost",
  },
);

const feedbackState = ref<RefreshFeedbackState>("idle");
const view = computed(() => getRefreshFeedbackView(feedbackState.value));
let resetTimer: number | undefined;

onBeforeUnmount(() => {
  window.clearTimeout(resetTimer);
});

async function handleRefresh() {
  if (feedbackState.value === "refreshing") {
    return;
  }

  window.clearTimeout(resetTimer);
  feedbackState.value = "refreshing";

  try {
    await props.refresh();
    feedbackState.value = "done";
    resetTimer = window.setTimeout(() => {
      feedbackState.value = "idle";
    }, 900);
  } catch (error) {
    feedbackState.value = "idle";
    throw error;
  }
}
</script>

<template>
  <Button
    :size="size"
    :variant="variant"
    :disabled="view.disabled"
    :class="view.buttonClass"
    aria-live="polite"
    @click="handleRefresh"
  >
    <RefreshCw class="h-4 w-4 transition-transform" :class="view.iconClass" />
    {{ view.label }}
  </Button>
</template>
