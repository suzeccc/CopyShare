<script setup lang="ts">
import Video from "lucide-vue-next/dist/esm/icons/video.js";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

import { getHistoryFileThumbnail } from "@/lib/tauri";

const props = withDefaults(
  defineProps<{
    historyId: string;
    fileName: string;
    maxSize?: number;
    compact?: boolean;
  }>(),
  {
    maxSize: 200,
    compact: false,
  },
);

const root = ref<HTMLElement | null>(null);
const src = ref("");
const failed = ref(false);
const isVideo = computed(() => /\.(mp4|mov|mkv|avi|webm|m4v|wmv)$/i.test(props.fileName));
const rootClass = computed(() =>
  props.compact
    ? "grid h-8 w-10 shrink-0 place-items-center overflow-hidden rounded-md border border-white/10 bg-black/20"
    : "grid h-14 w-20 shrink-0 place-items-center overflow-hidden rounded-lg border border-white/10 bg-black/20",
);
let observer: IntersectionObserver | undefined;
let cancelled = false;

async function loadThumbnail() {
  if (!isVideo.value || src.value || failed.value) {
    return;
  }
  try {
    const base64 = await getHistoryFileThumbnail(props.historyId, props.maxSize);
    if (!cancelled && base64) {
      src.value = `data:image/png;base64,${base64}`;
    }
  } catch {
    if (!cancelled) {
      failed.value = true;
    }
  }
}

onMounted(() => {
  if (!isVideo.value) {
    return;
  }
  if (!root.value || typeof IntersectionObserver === "undefined") {
    void loadThumbnail();
    return;
  }

  observer = new IntersectionObserver(
    ([entry]) => {
      if (!entry.isIntersecting) {
        return;
      }
      observer?.disconnect();
      void loadThumbnail();
    },
    { rootMargin: "160px" },
  );
  observer.observe(root.value);
});

onBeforeUnmount(() => {
  cancelled = true;
  observer?.disconnect();
});
</script>

<template>
  <div
    v-if="isVideo"
    ref="root"
    data-history-file-thumb
    :class="rootClass"
  >
    <img
      v-if="src"
      :src="src"
      :alt="fileName"
      class="h-full w-full object-cover"
      loading="lazy"
    >
    <div
      v-else
      data-history-file-thumb-placeholder
      class="grid h-full w-full place-items-center bg-gradient-to-br from-[#2f2314] via-[#242424] to-[#191919]"
      :title="failed ? '无法生成视频缩略图' : '正在生成视频缩略图'"
    >
      <Video class="h-5 w-5 text-[#ffb457]" />
    </div>
  </div>
</template>
