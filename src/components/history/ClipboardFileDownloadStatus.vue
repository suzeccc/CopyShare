<script setup lang="ts">
import CircleAlert from "lucide-vue-next/dist/esm/icons/circle-alert.js";
import CircleCheck from "lucide-vue-next/dist/esm/icons/circle-check.js";
import Download from "lucide-vue-next/dist/esm/icons/download.js";
import LoaderCircle from "lucide-vue-next/dist/esm/icons/loader-circle.js";
import { computed } from "vue";

import { getClipboardFileDownloadFeedback } from "@/lib/clipboardFileDownload";
import type { ClipboardPreviewItem } from "@/lib/historyPreview";
import { useHistoryStore } from "@/stores/history";

const props = withDefaults(
  defineProps<{
    item: ClipboardPreviewItem;
    compact?: boolean;
    inlineBadge?: boolean;
  }>(),
  {
    compact: false,
    inlineBadge: false,
  },
);

const historyStore = useHistoryStore();
const feedback = computed(() =>
  getClipboardFileDownloadFeedback(
    props.item,
    historyStore.fileDownloadActivity(props.item.fileTransferId),
  ),
);
const feedbackClass = computed(() => {
  switch (feedback.value?.state) {
    case "downloading":
    case "completed":
      return "border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]";
    case "failed":
      return "border-red-400/25 bg-red-400/10 text-red-200";
    default:
      return "border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] text-[color:var(--clipboard-card-footer-text)]";
  }
});
const progressStyle = computed(() => ({
  width: `${feedback.value?.percent ?? 0}%`,
}));
</script>

<template>
  <template v-if="feedback">
    <span
      data-clipboard-file-download-status
      class="inline-flex shrink-0 items-center gap-1 rounded-full border font-semibold"
      :class="[
        feedbackClass,
        compact
          ? 'px-1.5 py-0.5 text-[10px] leading-4'
          : inlineBadge
            ? 'px-2 py-0.5 text-[11px] leading-5'
            : 'absolute bottom-2.5 right-3 px-2 py-0.5 text-[11px] leading-5',
      ]"
      :title="feedback.label"
    >
      <LoaderCircle v-if="feedback.state === 'downloading'" class="h-3 w-3 animate-spin" />
      <CircleCheck v-else-if="feedback.state === 'completed'" class="h-3 w-3" />
      <CircleAlert v-else-if="feedback.state === 'failed'" class="h-3 w-3" />
      <Download v-else class="h-3 w-3" />
      <span>{{ feedback.label }}</span>
    </span>

    <span
      v-if="feedback.active && !compact"
      data-clipboard-file-download-progress
      class="absolute inset-x-3 bottom-0 h-0.5 overflow-hidden rounded-full bg-[color:var(--main-line-soft)]"
      aria-hidden="true"
    >
      <span
        class="block h-full rounded-full bg-[color:var(--accent-text)] transition-[width] duration-200 ease-out"
        :style="progressStyle"
      />
    </span>
  </template>
</template>
