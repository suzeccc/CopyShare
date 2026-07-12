<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useRoute } from "vue-router";
import { ImageIcon, Minus, PlaySquare, X } from "lucide-vue-next";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import HistoryImageThumb from "@/components/history/HistoryImageThumb.vue";
import {
  getNextMediaPreviewImageOffset,
  getNextMediaPreviewImageScale,
  MEDIA_PREVIEW_IMAGE_MIN_SCALE,
  shouldPanMediaPreviewImage,
  type MediaPreviewImagePoint,
} from "@/lib/mediaPreviewImagePanZoom";
import type { MediaPreviewKind, MediaPreviewPayload } from "@/lib/tauri";
import {
  closeWindow,
  getConfig,
  minimizeWindow,
  onAppEvent,
  openHistoryFileLocation,
  startWindowDrag,
} from "@/lib/tauri";
import { startWindowDragFromMouseEvent } from "@/lib/windowDrag";
import { useToastStore } from "@/stores/toasts";
import type { AppConfig, AppTheme } from "@/types/config";

const route = useRoute();
const toastStore = useToastStore();

const previewKind = ref<MediaPreviewKind>("image");
const historyId = ref("");
const title = ref("媒体预览");
const videoSrc = ref("");
const videoError = ref("");
const videoRef = ref<HTMLVideoElement | null>(null);
const imagePreviewScale = ref(MEDIA_PREVIEW_IMAGE_MIN_SCALE);
const imagePreviewOffset = ref<MediaPreviewImagePoint>({ x: 0, y: 0 });
const isImagePreviewPanning = ref(false);
const imagePreviewDragPointerId = ref<number | null>(null);
let mediaPreviewUnlisten: UnlistenFn | null = null;
let themeUnlisten: UnlistenFn | null = null;
let isUnmounted = false;
let imagePreviewDragOriginPointer: MediaPreviewImagePoint | null = null;
let imagePreviewDragOriginOffset: MediaPreviewImagePoint | null = null;

const isImage = computed(() => previewKind.value === "image");
const isVideo = computed(() => previewKind.value === "video");
const subtitle = computed(() => (isImage.value ? "图片预览" : "视频预览"));
const imagePreviewTransformStyle = computed(() => ({
  cursor: shouldPanMediaPreviewImage(imagePreviewScale.value)
    ? isImagePreviewPanning.value
      ? "grabbing"
      : "grab"
    : "zoom-in",
  transform: `translate3d(${imagePreviewOffset.value.x}px, ${imagePreviewOffset.value.y}px, 0) scale(${imagePreviewScale.value})`,
  transition: isImagePreviewPanning.value ? "none" : "transform 140ms ease",
}));

function queryValue(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function payloadFromRoute(): MediaPreviewPayload {
  const kind = queryValue(route.query.kind) === "video" ? "video" : "image";
  return {
    kind,
    historyId: queryValue(route.query.historyId),
    title: queryValue(route.query.title) || "媒体预览",
    src: queryValue(route.query.src) || undefined,
  };
}

function releaseVideoElement() {
  const video = videoRef.value;
  if (!video) {
    return;
  }

  video.pause();
  video.removeAttribute("src");
  video.load();
}

function resetImagePreviewTransform() {
  finishImagePreviewDrag();
  imagePreviewScale.value = MEDIA_PREVIEW_IMAGE_MIN_SCALE;
  imagePreviewOffset.value = { x: 0, y: 0 };
}

function applyMediaPreviewPayload(payload: MediaPreviewPayload) {
  resetImagePreviewTransform();
  releaseVideoElement();
  previewKind.value = payload.kind;
  historyId.value = payload.historyId;
  title.value = payload.title || "媒体预览";
  videoSrc.value = payload.kind === "video" ? payload.src ?? "" : "";
  videoError.value = "";
}

function applyMediaPreviewTheme(theme: AppTheme) {
  document.documentElement.dataset.appTheme = theme;
  document.body.dataset.appTheme = theme;
}

async function bindMediaPreviewTheme() {
  try {
    const config = await getConfig();
    if (!isUnmounted) {
      applyMediaPreviewTheme(config.theme);
    }
  } catch (error) {
    console.warn("Unable to load media preview theme", error);
  }

  const unlisten = await onAppEvent<AppConfig>("config-updated", (config) => {
    applyMediaPreviewTheme(config.theme);
  });

  if (isUnmounted) {
    unlisten();
    return;
  }

  themeUnlisten = unlisten;
}

async function bindMediaPreviewPayloadUpdates() {
  const unlisten = await listen<MediaPreviewPayload>(
    "media-preview-open",
    (event) => {
      applyMediaPreviewPayload(event.payload);
    },
  );

  if (isUnmounted) {
    unlisten();
    return;
  }

  mediaPreviewUnlisten = unlisten;
}

function handleWindowDrag(event: MouseEvent) {
  startWindowDragFromMouseEvent(event, startWindowDrag);
}

function finishImagePreviewDrag(event?: Event) {
  if (
    event instanceof PointerEvent &&
    event.currentTarget instanceof HTMLElement &&
    imagePreviewDragPointerId.value === event.pointerId &&
    event.currentTarget.hasPointerCapture(event.pointerId)
  ) {
    event.currentTarget.releasePointerCapture(event.pointerId);
  }
  isImagePreviewPanning.value = false;
  imagePreviewDragPointerId.value = null;
  imagePreviewDragOriginPointer = null;
  imagePreviewDragOriginOffset = null;
}

function pointerFromEvent(event: MouseEvent | PointerEvent): MediaPreviewImagePoint {
  return { x: event.clientX, y: event.clientY };
}

function handleImagePreviewWheel(event: WheelEvent) {
  const nextScale = getNextMediaPreviewImageScale(imagePreviewScale.value, event.deltaY);
  imagePreviewScale.value = nextScale;

  if (nextScale === MEDIA_PREVIEW_IMAGE_MIN_SCALE) {
    imagePreviewOffset.value = { x: 0, y: 0 };
    finishImagePreviewDrag();
  }
}

function handleImagePreviewDragPress(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  if (event.currentTarget instanceof HTMLElement) {
    event.currentTarget.setPointerCapture(event.pointerId);
  }
  isImagePreviewPanning.value = true;
  imagePreviewDragPointerId.value = event.pointerId;
  imagePreviewDragOriginPointer = pointerFromEvent(event);
  imagePreviewDragOriginOffset = { ...imagePreviewOffset.value };
}

function handleImagePreviewDragMove(event: PointerEvent) {
  if (
    !isImagePreviewPanning.value ||
    imagePreviewDragPointerId.value !== event.pointerId ||
    !imagePreviewDragOriginPointer ||
    !imagePreviewDragOriginOffset
  ) {
    return;
  }

  imagePreviewOffset.value = getNextMediaPreviewImageOffset(
    imagePreviewDragOriginOffset,
    imagePreviewDragOriginPointer,
    pointerFromEvent(event),
  );
}

function handleVideoPreviewError() {
  if (videoError.value) {
    return;
  }

  videoError.value = "无法播放此视频，可能是文件编码不受当前播放器支持。";
  toastStore.error("无法播放此视频");
}

async function revealSourceFile() {
  if (!historyId.value) {
    return;
  }

  try {
    await openHistoryFileLocation(historyId.value);
  } catch (error) {
    toastStore.error(`打开文件位置失败：${String(error)}`);
  }
}

onMounted(async () => {
  isUnmounted = false;
  applyMediaPreviewPayload(payloadFromRoute());
  void bindMediaPreviewTheme();
  void bindMediaPreviewPayloadUpdates();
});

onUnmounted(() => {
  isUnmounted = true;
  mediaPreviewUnlisten?.();
  mediaPreviewUnlisten = null;
  themeUnlisten?.();
  themeUnlisten = null;
  finishImagePreviewDrag();
  releaseVideoElement();
});
</script>

<template>
  <section
    data-media-preview-window
    class="flex h-screen w-screen flex-col overflow-hidden rounded-xl border border-[color:var(--floating-surface-line)] bg-[color:var(--floating-surface-bg)] text-slate-100 shadow-[0_22px_70px_rgba(0,0,0,0.46)] backdrop-blur-2xl"
  >
    <header
      class="flex shrink-0 items-center justify-between gap-3 border-b border-[color:var(--main-line-soft)] px-3 py-2"
      data-window-drag-region
      @mousedown.capture="handleWindowDrag"
    >
      <div class="flex min-w-0 items-center gap-2" data-window-drag-region>
        <span class="grid h-8 w-8 shrink-0 place-items-center rounded-lg border border-[color:var(--floating-stat-line)] bg-[color:var(--floating-stat-bg)] text-[color:var(--accent-text)]">
          <ImageIcon v-if="isImage" class="h-4 w-4" />
          <PlaySquare v-else class="h-4 w-4" />
        </span>
        <div class="min-w-0" data-window-drag-region>
          <p class="truncate text-sm font-semibold text-[color:var(--floating-strong-text)]">{{ title }}</p>
          <p class="text-[11px] font-medium text-[color:var(--floating-muted-text)]">{{ subtitle }}</p>
        </div>
      </div>
      <div class="flex shrink-0 items-center gap-1.5">
        <button
          data-media-preview-minimize-button
          class="grid h-8 w-8 place-items-center rounded-lg border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-[color:var(--floating-control-bg-hover)]"
          type="button"
          aria-label="隐藏预览"
          title="隐藏"
          data-window-control
          @click="minimizeWindow"
        >
          <Minus class="h-4 w-4" />
        </button>
        <button
          class="grid h-8 w-8 place-items-center rounded-lg border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-red-500/75 hover:text-white"
          type="button"
          aria-label="关闭预览"
          title="关闭"
          data-window-control
          @click="closeWindow"
        >
          <X class="h-4 w-4" />
        </button>
      </div>
    </header>

    <main class="min-h-0 flex-1 overflow-hidden p-3">
      <div
        v-if="isImage"
        class="grid h-full touch-none place-items-center overflow-hidden rounded-xl bg-black/35 p-3"
        data-media-preview-image-drag-surface
        @wheel.prevent="handleImagePreviewWheel"
        @pointerdown.left="handleImagePreviewDragPress"
        @pointermove="handleImagePreviewDragMove"
        @pointerup="finishImagePreviewDrag"
        @pointercancel="finishImagePreviewDrag"
        @lostpointercapture="finishImagePreviewDrag"
        @contextmenu="finishImagePreviewDrag"
        @dragstart.prevent="finishImagePreviewDrag"
      >
        <HistoryImageThumb
          v-if="historyId"
          data-media-preview-image
          :history-id="historyId"
          :max-size="1600"
          variant="preview"
          :alt="title"
          class="max-h-full max-w-full select-none rounded-lg object-contain will-change-transform"
          :style="imagePreviewTransformStyle"
          draggable="false"
        />
      </div>

      <div v-else class="grid h-full grid-rows-[minmax(0,1fr)_auto] gap-3">
        <div class="overflow-hidden rounded-xl bg-black">
          <video
            v-if="videoSrc"
            ref="videoRef"
            data-media-preview-video
            :src="videoSrc"
            class="h-full max-h-full w-full bg-black object-contain"
            preload="metadata"
            controls
            autoplay
            playsinline
            @error="handleVideoPreviewError"
          />
          <p
            v-else
            class="grid h-full place-items-center px-5 text-center text-sm font-medium text-[color:var(--floating-muted-text)]"
          >
            暂无可预览的视频文件
          </p>
        </div>
        <div
          v-if="videoError"
          class="flex items-center justify-between gap-3 rounded-lg border border-amber-300/20 bg-amber-400/[0.08] px-3 py-2 text-xs font-medium text-amber-100"
        >
          <span>{{ videoError }}</span>
          <button
            class="shrink-0 rounded-md border border-amber-200/20 px-2 py-1 transition hover:bg-amber-200/10"
            type="button"
            @click="revealSourceFile"
          >
            打开位置
          </button>
        </div>
      </div>
    </main>
  </section>
</template>
