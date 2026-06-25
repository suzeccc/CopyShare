<script setup lang="ts">
import { RefreshCw } from "lucide-vue-next";
import { computed, onBeforeUnmount, ref } from "vue";

import Button from "@/components/ui/Button.vue";
import {
  getRefreshFeedbackView,
  type RefreshFeedbackState,
} from "@/lib/refreshFeedback";
import { useToastStore } from "@/stores/toasts";

const props = withDefaults(
  defineProps<{
    refresh: () => Promise<unknown> | unknown;
    failed?: () => boolean;
    successMessage?: string;
    errorMessage?: string;
    size?: "sm" | "md";
    variant?: "secondary" | "ghost";
  }>(),
  {
    size: "sm",
    variant: "ghost",
    successMessage: "刷新成功",
    errorMessage: "刷新失败",
  },
);

const feedbackState = ref<RefreshFeedbackState>("idle");
const view = computed(() => getRefreshFeedbackView(feedbackState.value));
const toastStore = useToastStore();
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
    if (props.failed?.()) {
      feedbackState.value = "idle";
      toastStore.error(props.errorMessage);
      return;
    }

    feedbackState.value = "done";
    toastStore.success(props.successMessage);
    resetTimer = window.setTimeout(() => {
      feedbackState.value = "idle";
    }, 900);
  } catch (error) {
    feedbackState.value = "idle";
    toastStore.error(props.errorMessage);
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
