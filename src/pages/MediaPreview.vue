<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useRoute } from "vue-router";
import FolderOpen from "lucide-vue-next/dist/esm/icons/folder-open.js";
import Minus from "lucide-vue-next/dist/esm/icons/minus.js";
import X from "lucide-vue-next/dist/esm/icons/x.js";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { MediaPreviewPayload } from "@/lib/tauri";
import type { ClipboardPreviewItem } from "@/lib/historyPreview";
import { splitClipboardFileSummary } from "@/lib/historyPreview";
import MediaPreviewToolbar from "@/components/history/MediaPreviewToolbar.vue";
import DirectImagePreview from "@/components/history/DirectImagePreview.vue";
import { adjacentMediaPreviewItem, mediaPreviewItems, nextVideoPlaybackRate } from "@/lib/mediaPreviewControls";
import {
  closeWindow,
  convertLocalFileSrc,
  getHistoryFilePreviewPath,
  MEDIA_PREVIEW_ITEMS_STORAGE_KEY,
  getConfig,
  hideWindow,
  onAppEvent,
  openHistoryFileLocation,
  startWindowDrag,
} from "@/lib/tauri";
import { startWindowDragFromMouseEvent } from "@/lib/windowDrag";
import { useToastStore } from "@/stores/toasts";
import type { AppConfig, AppTheme } from "@/types/config";

const route = useRoute();
const toastStore = useToastStore();

const historyId = ref("");
const kind = ref<MediaPreviewPayload["kind"]>("video");
const title = ref("视频预览");
const videoSrc = ref("");
const videoError = ref("");
const videoRef = ref<HTMLVideoElement | null>(null);
const videoSession = ref(0);
const playlist = ref<ClipboardPreviewItem[]>([]);
const playbackRate = ref(1);
const navigating = ref(false);
let videoRequest = 0;
const previousVideo = computed(() => adjacentMediaPreviewItem(playlist.value, historyId.value, -1));
const nextVideo = computed(() => adjacentMediaPreviewItem(playlist.value, historyId.value, 1));
let mediaPreviewUnlisten: UnlistenFn | null = null;
let themeUnlisten: UnlistenFn | null = null;
let isUnmounted = false;

function queryValue(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function payloadFromRoute(): MediaPreviewPayload {
  const kind = route.query.kind === "image" ? "image" : "video";
  let items: ClipboardPreviewItem[] = [];
  try {
    const stored = JSON.parse(window.localStorage.getItem(MEDIA_PREVIEW_ITEMS_STORAGE_KEY) ?? "[]");
    if (Array.isArray(stored)) items = stored.filter(item =>
      item && typeof item.id === "string" && typeof item.text === "string"
        && item.contentType === (kind === "image" ? "image" : "fileList"),
    );
  } catch {
    // A stale playlist must not prevent opening the requested video.
  }
  return {
    kind,
    historyId: queryValue(route.query.historyId),
    title: queryValue(route.query.title) || (kind === "image" ? "图片预览" : "视频预览"),
    src: queryValue(route.query.src),
    items,
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

function applyMediaPreviewPayload(payload: MediaPreviewPayload) {
  videoRequest++;
  navigating.value = false;
  releaseVideoElement();
  videoSession.value++;
  historyId.value = payload.historyId;
  kind.value = payload.kind;
  title.value = payload.title || (payload.kind === "image" ? "图片预览" : "视频预览");
  videoSrc.value = payload.src || "";
  videoError.value = "";
  playbackRate.value = 1;
  playlist.value = mediaPreviewItems(payload.items ?? [], payload.kind);
}

async function changeVideo(direction: -1 | 1) {
  if (navigating.value) return;
  const item = adjacentMediaPreviewItem(playlist.value, historyId.value, direction);
  if (!item) return;
  const request = ++videoRequest;
  navigating.value = true;
  try {
    const path = await getHistoryFilePreviewPath(item.id);
    if (request !== videoRequest || isUnmounted) return;
    applyMediaPreviewPayload({
      kind: "video", historyId: item.id,
      title: splitClipboardFileSummary(item.text).name || "视频预览",
      src: convertLocalFileSrc(path), items: playlist.value,
    });
  } catch (error) {
    if (request === videoRequest && !isUnmounted) toastStore.error(`无法预览视频：${String(error)}`);
  } finally {
    if (request === videoRequest) navigating.value = false;
  }
}

function setPlaybackRate(rate: number) {
  playbackRate.value = rate;
  if (videoRef.value) videoRef.value.playbackRate = rate;
}

function replayVideo() {
  const video = videoRef.value;
  if (!video || !Number.isFinite(video.duration)) return;
  video.currentTime = 0;
  void video.play().catch(error => toastStore.error(`无法播放此视频：${String(error)}`));
}

function applyMediaPreviewTheme(theme: AppTheme) {
  document.documentElement.dataset.appTheme = theme;
  document.body.dataset.appTheme = theme;
}

function applyPreviewCanvas() {
  document.documentElement.dataset.windowMode = "media-preview";
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

function handleVideoPreviewError() {
  if (videoError.value) {
    return;
  }

  videoError.value = "无法播放此视频，可能是文件编码不受当前播放器支持";
  toastStore.error("无法播放此视频");
}

function handleImageChange(id: string) {
  historyId.value = id;
  title.value = splitClipboardFileSummary(playlist.value.find(item => item.id === id)?.text ?? "").name || "图片预览";
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

function handlePreviewKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    if (document.fullscreenElement) void document.exitFullscreen();
    else void closeWindow();
  }
}

onMounted(async () => {
  isUnmounted = false;
  applyPreviewCanvas();
  applyMediaPreviewPayload(payloadFromRoute());
  window.addEventListener("keydown", handlePreviewKeydown);
  void bindMediaPreviewTheme();
  void bindMediaPreviewPayloadUpdates();
});

onUnmounted(() => {
  isUnmounted = true;
  videoRequest++;
  mediaPreviewUnlisten?.();
  mediaPreviewUnlisten = null;
  themeUnlisten?.();
  themeUnlisten = null;
  window.removeEventListener("keydown", handlePreviewKeydown);
  releaseVideoElement();
});
</script>

<template>
  <section
    data-media-preview-window
    class="media-preview-glass relative grid h-screen w-screen grid-rows-[auto_minmax(0,1fr)] overflow-hidden text-[color:var(--clipboard-card-text)]"
  >
    <header
      data-media-preview-titlebar
      class="media-preview-glass-titlebar z-30 flex h-[52px] min-w-0 items-center justify-between gap-3 px-3"
      data-window-drag-region
      @mousedown.capture="handleWindowDrag"
    >
      <div
        class="flex min-w-0 items-center"
        data-window-drag-region
      >
        <div class="flex min-w-0 flex-row-reverse items-center justify-end gap-2.5" data-window-drag-region>
          <p data-i18n-ignore class="truncate border-l border-[color:var(--main-line)] pl-2.5 text-[12px] text-[color:var(--muted-text)]">{{ title }}</p>
          <p class="shrink-0 text-[14px] font-semibold">{{ kind === "image" ? "图片预览" : "视频预览" }}</p>
        </div>
      </div>
      <div
        class="flex shrink-0 items-center gap-1"
      >
        <button
          v-if="historyId"
          data-media-preview-open-location-button
          class="media-preview-window-control"
          type="button"
          aria-label="打开文件位置"
          title="打开文件位置"
          @click="revealSourceFile"
        >
          <FolderOpen class="h-4 w-4" />
        </button>
        <button
          data-media-preview-minimize-button
          class="media-preview-window-control"
          type="button"
          aria-label="隐藏预览"
          title="隐藏"
          data-window-control
          @click="hideWindow"
        >
          <Minus class="h-4 w-4" />
        </button>
        <button
          class="media-preview-window-control"
          type="button"
          aria-label="关闭预览"
          title="关闭"
          data-window-control
          @click="closeWindow"
        >
          <X class="h-[17px] w-[17px]" />
        </button>
      </div>
    </header>

    <main v-if="kind === 'image'" data-media-preview-image class="min-h-0 min-w-0 overflow-hidden">
      <DirectImagePreview :key="videoSession" embedded :history-id="historyId" :alt="title" :items="playlist" @change="handleImageChange" @close="closeWindow" />
    </main>
    <main v-else class="min-h-0 min-w-0 overflow-hidden p-3">
      <div class="flex h-full flex-col gap-3">
        <div data-media-preview-stage class="min-h-0 flex-1 overflow-hidden rounded-xl bg-transparent">
          <video
            v-if="videoSrc"
            ref="videoRef"
            :key="videoSession"
            data-media-preview-video
            :src="videoSrc"
            class="h-full max-h-full w-full rounded-xl bg-transparent object-contain shadow-[0_16px_48px_rgba(0,0,0,0.42)]"
            preload="metadata"
            controls
            autoplay
            playsinline
            @error="handleVideoPreviewError"
            @loadedmetadata="setPlaybackRate(playbackRate)"
          />
          <p
            v-else
            class="grid h-full place-items-center px-5 text-center text-sm font-medium text-[color:var(--floating-muted-text)]"
          >
            暂无可预览的视频文件
          </p>
        </div>
        <div class="z-30 flex shrink-0 justify-center" data-media-preview-video-toolbar>
          <MediaPreviewToolbar
            kind="video"
            :history-id="historyId"
            :value="playbackRate"
            :previous-disabled="!previousVideo || navigating"
            :next-disabled="!nextVideo || navigating"
            :decrease-disabled="playbackRate <= 0.25"
            :increase-disabled="playbackRate >= 3"
            :fullscreen-target="videoRef"
            @previous="changeVideo(-1)"
            @next="changeVideo(1)"
            @decrease="setPlaybackRate(nextVideoPlaybackRate(playbackRate, -1))"
            @increase="setPlaybackRate(nextVideoPlaybackRate(playbackRate, 1))"
            @reset="setPlaybackRate(1)"
            @rotate="replayVideo"
          />
        </div>
        <div
          v-if="videoError"
          class="flex items-center justify-between gap-3 rounded-xl border border-[color:var(--main-line)] bg-[color:var(--floating-control-bg)] px-3 py-2 text-xs font-medium text-[color:var(--clipboard-card-text)]"
        >
          <span>{{ videoError }}</span>
          <button
            v-if="historyId"
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

<style scoped>
:global(html[data-window-mode="media-preview"]),
:global(html[data-window-mode="media-preview"] body),
:global(html[data-window-mode="media-preview"] #app) {
  background: transparent !important;
}

.media-preview-glass {
  border: 1px solid var(--floating-control-line);
  border-radius: 12px;
  background: color-mix(in srgb, var(--main-bg) 58%, transparent);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 8%);
  backdrop-filter: blur(24px) saturate(120%);
}

.media-preview-glass-titlebar {
  border-bottom: 1px solid var(--main-line-soft);
  background: color-mix(in srgb, var(--panel-bg) 60%, transparent);
}

.media-preview-glass :deep(.preview-toolbar),
.media-preview-glass :deep(.preview-navigation) {
  border-color: var(--floating-control-line);
  background: color-mix(in srgb, var(--field-bg) 78%, transparent);
  color: var(--clipboard-card-text);
}

.media-preview-glass :deep(.preview-action:hover:not(:disabled)),
.media-preview-glass :deep(.preview-value:hover) {
  background: var(--floating-control-bg-hover);
  color: var(--clipboard-card-text);
}

[data-media-preview-video]:fullscreen { background: #000; }

.media-preview-window-control {
  display: grid;
  width: 32px;
  height: 32px;
  place-items: center;
  border-radius: 6px;
  color: var(--muted-text);
}

.media-preview-window-control:hover {
  background: var(--floating-control-bg-hover);
  color: var(--clipboard-card-text);
}

.media-preview-window-control:focus-visible {
  outline: 2px solid var(--accent-text);
  outline-offset: 2px;
}
</style>
