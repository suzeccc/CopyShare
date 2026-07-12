<script setup lang="ts">
import { Image as ImageIcon } from "lucide-vue-next";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

import { getHistoryImageThumbnail } from "@/lib/tauri";

const props = withDefaults(
  defineProps<{
    historyId: string;
    maxSize?: number;
    variant?: "thumb" | "preview";
    alt?: string;
  }>(),
  {
    maxSize: 200,
    variant: "thumb",
    alt: "",
  },
);

const root = ref<HTMLElement | null>(null);
const src = ref("");
const failed = ref(false);
const rootClass = computed(() =>
  props.variant === "preview"
    ? "grid max-h-[72vh] max-w-[82vw] place-items-center overflow-hidden rounded-xl border border-white/10 bg-black/30"
    : "grid h-14 w-20 shrink-0 place-items-center overflow-hidden rounded-lg border border-white/10 bg-black/20",
);
const imageClass = computed(() =>
  props.variant === "preview"
    ? "max-h-[72vh] max-w-[82vw] object-contain"
    : "h-full w-full object-cover",
);
let observer: IntersectionObserver | undefined;
let cancelled = false;

async function loadThumbnail() {
  if (src.value || failed.value) {
    return;
  }
  try {
    const base64 = await getHistoryImageThumbnail(props.historyId, props.maxSize);
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
    ref="root"
    data-history-image-thumb
    :class="rootClass"
  >
    <img
      v-if="src"
      :src="src"
      :alt="props.alt"
      :class="imageClass"
      loading="lazy"
      draggable="false"
    />
    <ImageIcon v-else class="h-5 w-5 text-slate-500" />
  </div>
</template>
