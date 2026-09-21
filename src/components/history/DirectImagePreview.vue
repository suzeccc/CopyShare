<script setup lang="ts">
import X from "lucide-vue-next/dist/esm/icons/x.js";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";

import HistoryImageThumb from "@/components/history/HistoryImageThumb.vue";
import MediaPreviewToolbar from "@/components/history/MediaPreviewToolbar.vue";
import { adjacentMediaPreviewItem, mediaPreviewItems } from "@/lib/mediaPreviewControls";
import type { ClipboardPreviewItem } from "@/lib/historyPreview";
import {
  getNextMediaPreviewImageOffset,
  getNextMediaPreviewImageScale,
  MEDIA_PREVIEW_IMAGE_MIN_SCALE,
  MEDIA_PREVIEW_IMAGE_MAX_SCALE,
  type MediaPreviewImagePoint,
} from "@/lib/mediaPreviewImagePanZoom";

const props = defineProps<{
  historyId: string;
  alt?: string;
  items?: ClipboardPreviewItem[];
  embedded?: boolean;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "change", historyId: string): void;
}>();

const scale = ref(1);
const rotation = ref(0);
const currentHistoryId = ref(props.historyId);
const previewRoot = ref<HTMLElement | null>(null);
const surface = ref<HTMLElement | null>(null);
const imageSize = ref({ width: 0, height: 0 });
const surfaceSize = ref({ width: 0, height: 0 });
const gallery = computed(() => mediaPreviewItems(props.items ?? [], "image"));
const previous = computed(() => adjacentMediaPreviewItem(gallery.value, currentHistoryId.value, -1));
const next = computed(() => adjacentMediaPreviewItem(gallery.value, currentHistoryId.value, 1));
const currentAlt = computed(() => gallery.value.find(item => item.id === currentHistoryId.value)?.text ?? props.alt);
const offset = ref<MediaPreviewImagePoint>({ x: 0, y: 0 });
const dragging = ref(false);
const pointerId = ref<number | null>(null);
let dragOriginPointer: MediaPreviewImagePoint | null = null;
let dragOriginOffset: MediaPreviewImagePoint | null = null;
let previousFocus: HTMLElement | null = null;
let resizeObserver: ResizeObserver | undefined;
const rotationFit = computed(() => {
  const image = imageSize.value;
  const stage = surfaceSize.value;
  if (rotation.value % 180 === 0 || !image.width || !image.height || !stage.width || !stage.height) return 1;
  const original = Math.min(stage.width / image.width, stage.height / image.height);
  return Math.min(stage.width / image.height, stage.height / image.width) / original;
});

const imageStyle = computed(() => ({
  transform: `translate3d(${offset.value.x}px, ${offset.value.y}px, 0) scale(${scale.value * rotationFit.value}) rotate(${rotation.value}deg)`,
  transition: dragging.value ? "none" : "transform 100ms ease-out",
}));

function finishDrag(event?: PointerEvent) {
  if (
    event
    && event.currentTarget instanceof HTMLElement
    && pointerId.value === event.pointerId
    && event.currentTarget.hasPointerCapture(event.pointerId)
  ) {
    event.currentTarget.releasePointerCapture(event.pointerId);
  }
  dragging.value = false;
  pointerId.value = null;
  dragOriginPointer = null;
  dragOriginOffset = null;
}

function startDrag(event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  if (event.currentTarget instanceof HTMLElement) {
    event.currentTarget.setPointerCapture(event.pointerId);
  }
  dragging.value = true;
  pointerId.value = event.pointerId;
  dragOriginPointer = { x: event.clientX, y: event.clientY };
  dragOriginOffset = { ...offset.value };
}

function moveDrag(event: PointerEvent) {
  if (
    !dragging.value
    || pointerId.value !== event.pointerId
    || !dragOriginPointer
    || !dragOriginOffset
  ) return;

  offset.value = getNextMediaPreviewImageOffset(
    dragOriginOffset,
    dragOriginPointer,
    { x: event.clientX, y: event.clientY },
  );
}

function zoom(event: WheelEvent) {
  if (!event.deltaY) return;
  scale.value = getNextMediaPreviewImageScale(scale.value, event.deltaY);
  if (scale.value === 1) offset.value = { x: 0, y: 0 };
}

function resetView() {
  finishDrag();
  scale.value = 1;
  rotation.value = 0;
  offset.value = { x: 0, y: 0 };
}

function imageLoaded(event: Event) {
  if (event.target instanceof HTMLImageElement) {
    imageSize.value = { width: event.target.naturalWidth, height: event.target.naturalHeight };
  }
}

function changeImage(direction: -1 | 1) {
  const item = adjacentMediaPreviewItem(gallery.value, currentHistoryId.value, direction);
  if (item) currentHistoryId.value = item.id;
}

function rotateImage() {
  finishDrag();
  rotation.value = (rotation.value + 90) % 360;
  offset.value = { x: 0, y: 0 };
}

function changeScale(direction: -1 | 1) {
  scale.value = getNextMediaPreviewImageScale(scale.value, direction === 1 ? -1 : 1);
  if (scale.value === 1) offset.value = { x: 0, y: 0 };
}

function handleKeydown(event: KeyboardEvent) {
  if (event.target instanceof HTMLElement && event.target.closest("input, textarea, select, [contenteditable=true]")) return;
  if (event.key === "Tab" && !props.embedded) {
    const buttons = previewRoot.value?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)");
    if (!buttons?.length) return;
    const target = event.shiftKey ? buttons[buttons.length - 1] : buttons[0];
    const boundary = event.shiftKey ? buttons[0] : buttons[buttons.length - 1];
    if (document.activeElement === previewRoot.value || document.activeElement === boundary || !previewRoot.value?.contains(document.activeElement)) {
      event.preventDefault();
      target.focus();
    }
    return;
  }
  if (!["Escape", "ArrowLeft", "ArrowRight", "+", "=", "-", "0", "r", "R"].includes(event.key)) return;
  event.preventDefault();
  event.stopImmediatePropagation();
  if (event.key === "Escape") {
    if (document.fullscreenElement) void document.exitFullscreen();
    else emit("close");
  } else if (event.key === "ArrowLeft") changeImage(-1);
  else if (event.key === "ArrowRight") changeImage(1);
  else if (event.key === "+" || event.key === "=") changeScale(1);
  else if (event.key === "-") changeScale(-1);
  else if (event.key === "0") resetView();
  else rotateImage();
}

watch(() => props.historyId, id => { currentHistoryId.value = id; });
watch(currentHistoryId, id => {
  imageSize.value = { width: 0, height: 0 };
  resetView();
  emit("change", id);
});
onMounted(() => {
  resizeObserver = new ResizeObserver(([entry]) => {
    surfaceSize.value = { width: entry.contentRect.width, height: entry.contentRect.height };
  });
  if (surface.value) resizeObserver.observe(surface.value);
  previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  previewRoot.value?.focus({ preventScroll: true });
  window.addEventListener("keydown", handleKeydown, true);
});
onUnmounted(() => {
  resizeObserver?.disconnect();
  window.removeEventListener("keydown", handleKeydown, true);
  if (previousFocus?.isConnected) previousFocus.focus();
});
</script>

<template>
  <Teleport to="body" :disabled="props.embedded">
    <div
      data-direct-image-preview
      class="flex items-center justify-center bg-transparent"
      :class="props.embedded ? 'h-full w-full' : 'fixed inset-0 z-[80] px-6 py-8 backdrop-blur-sm'"
      @click.self="emit('close')"
    >
      <section
        ref="previewRoot"
        data-direct-image-preview-fixed-canvas
        class="relative bg-transparent outline-none"
        :class="props.embedded ? 'h-full w-full' : 'h-[82vh] w-[90vw] max-h-[720px] max-w-[1200px]'"
        role="dialog"
        tabindex="-1"
        :aria-modal="!props.embedded"
        aria-label="图片预览"
      >
        <button
          v-if="!props.embedded"
          data-direct-image-preview-close
          class="absolute right-2 top-2 z-10 grid h-11 w-11 place-items-center rounded-full border border-white/15 bg-black/55 text-slate-200 shadow-[0_8px_22px_rgba(0,0,0,0.35)] backdrop-blur-md transition hover:bg-black/75 hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/35"
          type="button"
          aria-label="关闭图片预览"
          title="关闭图片预览"
          @click="emit('close')"
        >
          <X class="h-[18px] w-[18px]" />
        </button>
        <div
          ref="surface"
          data-direct-image-preview-surface
          class="grid h-full w-full touch-none place-items-center overflow-hidden bg-transparent"
          :class="dragging ? 'cursor-grabbing' : 'cursor-grab'"
          @wheel.prevent="zoom"
          @pointerdown="startDrag"
          @pointermove="moveDrag"
          @pointerup="finishDrag"
          @pointercancel="finishDrag"
          @lostpointercapture="finishDrag"
          @dragstart.prevent
        >
          <HistoryImageThumb
            data-direct-image-preview-image
            :key="currentHistoryId"
            :history-id="currentHistoryId"
            :max-size="1600"
            variant="preview"
            :alt="currentAlt"
            class="origin-center !h-full !w-full !rounded-none !border-0 !bg-transparent select-none drop-shadow-[0_18px_42px_rgba(0,0,0,0.42)] will-change-transform"
            :style="imageStyle"
            @load.capture="imageLoaded"
          />
        </div>
        <div class="pointer-events-none absolute inset-x-2 bottom-3 z-10 flex justify-center sm:bottom-5">
          <MediaPreviewToolbar
            class="pointer-events-auto"
            kind="image"
            :history-id="currentHistoryId"
            :value="scale"
            :previous-disabled="!previous"
            :next-disabled="!next"
            :decrease-disabled="scale <= MEDIA_PREVIEW_IMAGE_MIN_SCALE"
            :increase-disabled="scale >= MEDIA_PREVIEW_IMAGE_MAX_SCALE"
            :fullscreen-target="previewRoot"
            @previous="changeImage(-1)"
            @next="changeImage(1)"
            @decrease="changeScale(-1)"
            @increase="changeScale(1)"
            @reset="resetView"
            @rotate="rotateImage"
          />
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
@media (max-width: 480px) {
  [data-direct-image-preview-close] { width: 36px; height: 36px; }
}
[data-direct-image-preview-fixed-canvas]:fullscreen {
  width: 100vw;
  height: 100vh;
  max-width: none;
  max-height: none;
  background: #101214;
}
@media (prefers-reduced-motion: reduce) {
  [data-direct-image-preview-image] { transition: none !important; }
}
</style>
