<script setup lang="ts">
import ClipboardIcon from "lucide-vue-next/dist/esm/icons/clipboard.js";
import File from "lucide-vue-next/dist/esm/icons/file.js";
import ImageIcon from "lucide-vue-next/dist/esm/icons/image.js";
import Link2 from "lucide-vue-next/dist/esm/icons/link-2.js";
import Pin from "lucide-vue-next/dist/esm/icons/pin.js";
import Search from "lucide-vue-next/dist/esm/icons/search.js";
import Star from "lucide-vue-next/dist/esm/icons/star.js";
import Video from "lucide-vue-next/dist/esm/icons/video.js";
import X from "lucide-vue-next/dist/esm/icons/x.js";
import { computed, onMounted, onUnmounted, ref } from "vue";

import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import CopyTextButton from "@/components/ui/CopyTextButton.vue";
import ClipboardFileDownloadStatus from "@/components/history/ClipboardFileDownloadStatus.vue";
import HistoryFileThumb from "@/components/history/HistoryFileThumb.vue";
import HistoryImageThumb from "@/components/history/HistoryImageThumb.vue";
import {
  getClipboardFileCardAction,
  isClipboardFileCardInteractive,
} from "@/lib/clipboardFileDownload";
import {
  CLIPBOARD_CATEGORIES,
  CLIPBOARD_PREVIEW_LIMIT,
  filterClipboardItems,
  getClipboardDisplayType,
  getClipboardLinkUrl,
  getRecentClipboardItems,
  isClipboardVideoFile,
  splitClipboardFileSummary,
  type ClipboardCategory,
  type ClipboardDisplayType,
  type ClipboardPreviewItem,
} from "@/lib/historyPreview";
import {
  convertLocalFileSrc,
  copyHistoryItem,
  getHistoryFilePreviewPath,
  openExternalUrl,
  openHistoryFileLocation,
  openTransferFolder,
} from "@/lib/tauri";
import { useHistoryStore } from "@/stores/history";
import { useLibraryStore } from "@/stores/library";
import { useToastStore } from "@/stores/toasts";

const historyStore = useHistoryStore();
const libraryStore = useLibraryStore();
const toastStore = useToastStore();
const showClipboardHistoryModal = ref(false);
const clipboardSearch = ref("");
const activeClipboardCategory = ref<ClipboardCategory>(CLIPBOARD_CATEGORIES[0]);
const clipboardCategories = CLIPBOARD_CATEGORIES;
const expandedClipboardItemIds = ref<Set<string>>(new Set());
const previewImageItem = ref<ClipboardPreviewItem | null>(null);
const previewImageScale = ref(1);
const previewImageOffset = {
  x: ref(0),
  y: ref(0),
};
const previewImageDrag = {
  active: ref(false),
  pointerId: ref<number | null>(null),
  startClientX: ref(0),
  startClientY: ref(0),
  startOffsetX: ref(0),
  startOffsetY: ref(0),
};
const previewVideoItem = ref<ClipboardPreviewItem | null>(null);
const previewVideoSrc = ref("");
const previewVideoError = ref("");
const activeClipboardCategoryIndex = computed(() =>
  Math.max(0, clipboardCategories.indexOf(activeClipboardCategory.value)),
);
const previewImageTransform = computed(() => ({
  transform: `translate(${previewImageOffset.x.value}px, ${previewImageOffset.y.value}px) scale(${previewImageScale.value})`,
}));

const allClipboardItems = computed(() =>
  getRecentClipboardItems(historyStore.items, historyStore.items.length),
);
const recentSyncItems = computed(() =>
  allClipboardItems.value.slice(0, CLIPBOARD_PREVIEW_LIMIT),
);
const filteredRecentSyncItems = computed(() =>
  filterClipboardItems(recentSyncItems.value, activeClipboardCategory.value, ""),
);
const filteredAllClipboardItems = computed(() =>
  filterClipboardItems(allClipboardItems.value, activeClipboardCategory.value, clipboardSearch.value),
);

function savedLibraryItem(item: ClipboardPreviewItem) {
  return libraryStore.savedItemForHistory(item.id);
}

function isHistoryFavoriteBusy(item: ClipboardPreviewItem) {
  const saved = savedLibraryItem(item);
  return libraryStore.isItemBusy(item.id)
    || Boolean(saved && libraryStore.isItemBusy(saved.id));
}

async function toggleHistoryFavorite(item: ClipboardPreviewItem) {
  const saved = savedLibraryItem(item);
  if (saved) {
    if (!window.confirm(`确定移出“${saved.title}”吗？`)) return;
    try {
      await libraryStore.removeItem(saved.id);
      toastStore.success("已移出收藏夹");
    } catch (error) {
      toastStore.error(`移出收藏失败：${String(error)}`);
    }
    return;
  }

  try {
    await libraryStore.collectHistoryItem(item.id, false);
    toastStore.success("已加入收藏夹");
  } catch (error) {
    toastStore.error(`收藏失败：${String(error)}`);
  }
}

async function toggleHistoryPin(item: ClipboardPreviewItem) {
  try {
    await historyStore.setPinned(item.id, !item.isPinned);
    toastStore.success(item.isPinned ? "已取消置顶" : "已置顶");
  } catch (error) {
    toastStore.error(`置顶失败：${String(error)}`);
  }
}

onMounted(async () => {
  try {
    await Promise.all([
      libraryStore.loaded ? Promise.resolve() : libraryStore.load(),
      libraryStore.subscribe(),
    ]);
  } catch (error) {
    toastStore.error(`收藏状态加载失败：${String(error)}`);
  }
});

onUnmounted(() => libraryStore.disposeSubscription());

type SyncStatusPreviewItem = { syncStatus: "synced" | "unsynced" };

function syncStatusLabel(item: SyncStatusPreviewItem) {
  return item.syncStatus === "synced" ? "已同步" : "未同步";
}

function syncStatusClass(item: SyncStatusPreviewItem) {
  return item.syncStatus === "synced"
    ? "border-emerald-300/20 bg-emerald-400/[0.08] text-emerald-100"
    : "border-amber-300/20 bg-amber-400/[0.08] text-amber-100";
}

function setActiveClipboardCategory(category: ClipboardCategory) {
  activeClipboardCategory.value = category;
}

function isClipboardItemExpandable(item: ClipboardPreviewItem) {
  if (item.contentType === "fileList") {
    return false;
  }
  return item.text.length > 80 || item.text.includes("\n");
}

function clipboardFileSummary(item: ClipboardPreviewItem) {
  return splitClipboardFileSummary(item.text);
}

function clipboardFileNameClass(item: ClipboardPreviewItem) {
  return isClipboardFileCardInteractive(
    item,
    historyStore.fileDownloadActivity(item.fileTransferId),
  )
    ? "cursor-pointer underline-offset-2 transition-colors duration-150 hover:text-[color:var(--accent-text)] hover:underline"
    : "";
}

function isClipboardItemExpanded(item: ClipboardPreviewItem) {
  return expandedClipboardItemIds.value.has(item.id);
}

function toggleClipboardItemExpanded(item: ClipboardPreviewItem) {
  const next = new Set(expandedClipboardItemIds.value);
  if (next.has(item.id)) {
    next.delete(item.id);
  } else {
    next.add(item.id);
  }
  expandedClipboardItemIds.value = next;
}

function openClipboardImagePreview(item: ClipboardPreviewItem) {
  if (item.contentType !== "image") {
    return;
  }

  previewImageScale.value = 1;
  previewImageOffset.x.value = 0;
  previewImageOffset.y.value = 0;
  previewImageDrag.active.value = false;
  previewImageDrag.pointerId.value = null;
  previewImageItem.value = item;
}

function closeClipboardImagePreview() {
  previewImageItem.value = null;
  previewImageScale.value = 1;
  previewImageOffset.x.value = 0;
  previewImageOffset.y.value = 0;
  previewImageDrag.active.value = false;
  previewImageDrag.pointerId.value = null;
}

function closeClipboardVideoPreview() {
  previewVideoItem.value = null;
  previewVideoSrc.value = "";
  previewVideoError.value = "";
}

async function openClipboardVideoFallbackLocation(item: ClipboardPreviewItem) {
  try {
    if (item.direction === "local") {
      await openHistoryFileLocation(item.id);
      return;
    }
    if (item.fileTransferStatus === "completed") {
      await openTransferFolder();
    }
  } catch {
    // The original preview error already tells the user what happened.
  }
}

async function openClipboardVideoPreview(item: ClipboardPreviewItem) {
  if (!isClipboardVideoFile(item)) {
    return;
  }

  try {
    previewVideoError.value = "";
    const filePath = await getHistoryFilePreviewPath(item.id);
    previewVideoSrc.value = convertLocalFileSrc(filePath);
    previewVideoItem.value = item;
  } catch (error) {
    const action = getClipboardFileCardAction(
      item,
      historyStore.fileDownloadActivity(item.fileTransferId),
    );
    if (action === "download") {
      await handleClipboardItemClick(item);
      return;
    }
    if (action === "downloading") {
      toastStore.info("文件正在下载");
      return;
    }
    toastStore.error(`无法预览视频：${String(error)}`);
    await openClipboardVideoFallbackLocation(item);
  }
}

async function handleClipboardVideoPreviewError() {
  if (!previewVideoItem.value || previewVideoError.value) {
    return;
  }

  previewVideoError.value = "无法播放此视频，可能是文件编码不受当前播放器支持。";
  toastStore.error("无法播放此视频，已打开文件位置");
  await openClipboardVideoFallbackLocation(previewVideoItem.value);
}

function handleClipboardVideoLoaded() {
  previewVideoError.value = "";
}

async function openClipboardLink(item: ClipboardPreviewItem) {
  const url = getClipboardLinkUrl(item.text);
  if (!url) {
    return;
  }

  try {
    await openExternalUrl(url);
  } catch (error) {
    toastStore.error(`打开链接失败：${String(error)}`);
  }
}

async function handleClipboardItemClick(item: ClipboardPreviewItem) {
  if (item.contentType !== "fileList") {
    return;
  }

  const action = getClipboardFileCardAction(
    item,
    historyStore.fileDownloadActivity(item.fileTransferId),
  );
  if (action === "none") {
    return;
  }
  if (action === "openSourceLocation") {
    try {
      await openHistoryFileLocation(item.id);
      toastStore.success("已打开文件位置");
    } catch (error) {
      toastStore.error(`打开文件位置失败：${String(error)}`);
    }
    return;
  }
  if (action === "downloading") {
    toastStore.info("文件正在下载");
    return;
  }
  if (action === "openDownloadFolder") {
    try {
      await openTransferFolder();
      toastStore.success("已打开文件下载位置");
    } catch (error) {
      toastStore.error(`打开文件下载位置失败：${String(error)}`);
    }
    return;
  }
  if (action === "unavailable") {
    toastStore.error("文件下载已失效");
    return;
  }

  if (action === "download") {
    historyStore.beginFileDownload(item.fileTransferId);
  }

  try {
    const result = await copyHistoryItem(item.id);
    if (result === "downloadStarted") {
      historyStore.beginFileDownload(item.fileTransferId);
      toastStore.success("开始下载");
    } else if (result === "downloading") {
      historyStore.beginFileDownload(item.fileTransferId);
      toastStore.info("文件正在下载");
    } else {
      toastStore.success("文件已复制");
    }
  } catch (error) {
    historyStore.failFileDownload(item.fileTransferId, String(error));
    toastStore.error("文件下载失败");
  }
}

function handleClipboardImagePreviewWheel(event: WheelEvent) {
  const delta = event.deltaY < 0 ? 0.12 : -0.12;
  const next = Math.min(3, Math.max(0.35, previewImageScale.value + delta));
  previewImageScale.value = Number(next.toFixed(2));
}

function startClipboardImageDrag(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }

  event.preventDefault();
  previewImageDrag.active.value = true;
  previewImageDrag.pointerId.value = event.pointerId;
  previewImageDrag.startClientX.value = event.clientX;
  previewImageDrag.startClientY.value = event.clientY;
  previewImageDrag.startOffsetX.value = previewImageOffset.x.value;
  previewImageDrag.startOffsetY.value = previewImageOffset.y.value;
  if (event.currentTarget instanceof HTMLElement) {
    event.currentTarget.setPointerCapture(event.pointerId);
  }
}

function moveClipboardImageDrag(event: PointerEvent) {
  if (!previewImageDrag.active.value || previewImageDrag.pointerId.value !== event.pointerId) {
    return;
  }

  previewImageOffset.x.value =
    previewImageDrag.startOffsetX.value + event.clientX - previewImageDrag.startClientX.value;
  previewImageOffset.y.value =
    previewImageDrag.startOffsetY.value + event.clientY - previewImageDrag.startClientY.value;
}

function endClipboardImageDrag(event: PointerEvent) {
  if (previewImageDrag.pointerId.value !== event.pointerId) {
    return;
  }

  previewImageDrag.active.value = false;
  previewImageDrag.pointerId.value = null;
  if (event.currentTarget instanceof HTMLElement && event.currentTarget.hasPointerCapture(event.pointerId)) {
    event.currentTarget.releasePointerCapture(event.pointerId);
  }
}

function clipboardTypeClass(_type: ClipboardDisplayType) {
  return "text-[color:var(--clipboard-card-meta-text)]";
}

function clipboardTypeIcon(type: ClipboardDisplayType) {
  const icons = {
    text: ClipboardIcon,
    image: ImageIcon,
    link: Link2,
    file: File,
    video: Video,
  };
  return icons[type.tone];
}

function clipboardAccentClass(type: ClipboardDisplayType) {
  const classes = {
    text: "bg-[#007aff]",
    image: "bg-[#34a851]",
    link: "bg-[#7b5520]",
    file: "bg-[#af52de]",
    video: "bg-[#ff9f0a]",
  };
  return classes[type.tone];
}

function clipboardTextClass(type: ClipboardDisplayType) {
  return type.tone === "link"
    ? "text-[color:var(--clipboard-card-link-text)]"
    : "text-[color:var(--clipboard-card-text)]";
}

function clipboardTime(value: string | undefined) {
  if (!value) {
    return "暂无";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}
</script>

<template>
  <div data-clipboard-page class="grid min-w-0 max-w-full gap-4 overflow-x-hidden">
    <Card>
      <div class="mb-4 flex flex-wrap items-start justify-between gap-4">
        <div>
          <p class="text-base font-semibold text-white">最近同步内容</p>
          <p class="mt-1 text-sm text-slate-500">最近 {{ CLIPBOARD_PREVIEW_LIMIT }} 条历史记录</p>
        </div>
        <Button
          data-more-clipboard-button
          size="sm"
          variant="ghost"
          @click="showClipboardHistoryModal = true"
        >
          更多
        </Button>
      </div>

      <div class="mb-4 grid gap-3">
        <div
          data-clipboard-category-tabs
          class="clipboard-category-tabs"
          :style="{
            '--clipboard-category-index': activeClipboardCategoryIndex,
            '--clipboard-category-count': clipboardCategories.length,
          }"
        >
          <span data-clipboard-category-indicator class="clipboard-category-indicator" />
          <button
            v-for="category in clipboardCategories"
            :key="category"
            data-clipboard-category-button
            class="clipboard-category-chip"
            :class="{ active: activeClipboardCategory === category }"
            type="button"
            @click="setActiveClipboardCategory(category)"
          >
            {{ category }}
          </button>
        </div>
      </div>

      <TransitionGroup
        v-if="filteredRecentSyncItems.length"
        name="clipboard-card-stagger"
        tag="div"
        data-clipboard-stagger-list
        class="relative grid min-w-0 gap-3"
      >
        <article
          v-for="(item, index) in filteredRecentSyncItems"
          :key="item.id"
          data-clipboard-history-row
          class="clipboard-preview-card group relative min-h-[86px] overflow-hidden rounded-xl border border-[color:var(--clipboard-card-line)] bg-[color:var(--clipboard-card-bg)] px-5 py-2.5 shadow-[var(--clipboard-card-shadow)] transition duration-150 ease-out hover:z-10 hover:scale-[1.01] hover:border-[color:var(--clipboard-card-line-hover)] hover:bg-[color:var(--clipboard-card-bg-hover)] hover:shadow-[var(--clipboard-card-shadow-hover)]"
          :class="{
            'cursor-pointer': isClipboardFileCardInteractive(item, historyStore.fileDownloadActivity(item.fileTransferId)),
            'cursor-wait': isClipboardFileCardInteractive(item, historyStore.fileDownloadActivity(item.fileTransferId)) && historyStore.isFileDownloadActive(item.fileTransferId),
          }"
          :style="`--clipboard-row-index: ${index}`"
          @click="handleClipboardItemClick(item)"
        >
          <span class="clipboard-preview-card-accent absolute left-3 top-2.5 bottom-2.5 w-0.5 rounded-full transition duration-150 group-hover:brightness-125" :class="clipboardAccentClass(getClipboardDisplayType(item))" />
          <div data-clipboard-card-main class="min-w-0 pl-4">
            <div data-clipboard-card-header class="flex min-w-0 items-start justify-between gap-3">
              <span
                data-clipboard-type-label
                data-clipboard-card-meta
                class="inline-flex items-center gap-1 text-[12px] font-medium"
                :class="clipboardTypeClass(getClipboardDisplayType(item))"
              >
                <component :is="clipboardTypeIcon(getClipboardDisplayType(item))" class="h-3.5 w-3.5" />
                {{ getClipboardDisplayType(item).label }}
              </span>
              <div data-clipboard-history-actions class="flex shrink-0 items-center gap-1.5">
                <button
                  data-history-favorite
                  type="button"
                  class="clipboard-card-library-action clipboard-card-favorite-action"
                  :class="{ active: libraryStore.isHistoryItemSaved(item.id) }"
                  :disabled="isHistoryFavoriteBusy(item)"
                  :aria-label="libraryStore.isHistoryItemSaved(item.id) ? '移出收藏夹' : '收藏'"
                  @click.stop="toggleHistoryFavorite(item)"
                >
                  <Star
                    class="h-3.5 w-3.5"
                    :fill="libraryStore.isHistoryItemSaved(item.id) ? 'currentColor' : 'none'"
                  />
                </button>
                <button
                  data-history-pin
                  type="button"
                  class="clipboard-card-library-action clipboard-card-pin-action"
                  :class="{ active: item.isPinned }"
                  :disabled="historyStore.isPinning(item.id)"
                  :aria-label="item.isPinned ? '取消置顶' : '置顶历史记录'"
                  @click.stop="toggleHistoryPin(item)"
                >
                  <Pin
                    class="h-3.5 w-3.5"
                    :fill="item.isPinned ? 'currentColor' : 'none'"
                  />
                </button>
                <CopyTextButton
                  data-clipboard-card-action
                  class="clipboard-card-copy-action"
                  :text="item.text"
                  :content-type="item.contentType"
                  :history-item-id="item.id"
                  :file-transfer-id="item.fileTransferId"
                  :file-transfer-status="item.fileTransferStatus"
                  icon-only
                  label="复制内容"
                />
              </div>
            </div>
            <div data-clipboard-card-content class="relative min-w-0">
            <div v-if="item.contentType === 'image'" class="mt-2 flex min-w-0 items-center gap-3">
              <button
                data-clipboard-image-preview-button
                class="rounded-lg outline-none transition hover:scale-[1.03] focus-visible:ring-2 focus-visible:ring-[#60cdff]/60"
                type="button"
                aria-label="鏀惧ぇ棰勮鍥剧墖"
                @click="openClipboardImagePreview(item)"
              >
                <HistoryImageThumb :history-id="item.id" />
              </button>
              <div
                data-clipboard-image-summary
                class="flex min-w-0 items-baseline gap-2.5 text-[13px] font-medium leading-[19px] text-[color:var(--clipboard-card-text)]"
              >
                <span data-clipboard-image-name class="min-w-0 truncate">
                  {{ clipboardFileSummary(item).name }}
                </span>
                <span
                  v-if="clipboardFileSummary(item).size"
                  data-clipboard-image-size
                  class="shrink-0 text-[12px] text-[color:var(--clipboard-card-footer-text)]"
                >
                  {{ clipboardFileSummary(item).size }}
                </span>
              </div>
            </div>
            <div
              v-else-if="item.contentType === 'fileList'"
              data-clipboard-file-summary
              class="mt-2 flex min-w-0 select-none items-center gap-3 text-[13px] font-medium leading-[19px] text-[color:var(--clipboard-card-text)]"
            >
              <button
                v-if="isClipboardVideoFile(item)"
                data-clipboard-file-media-button
                class="rounded-lg outline-none transition hover:scale-[1.03] focus-visible:ring-2 focus-visible:ring-[#60cdff]/60"
                type="button"
                aria-label="预览视频"
                @click.stop="openClipboardVideoPreview(item)"
              >
                <HistoryFileThumb
                  data-clipboard-file-media
                  :history-id="item.id"
                  :file-name="clipboardFileSummary(item).name"
                />
              </button>
              <HistoryFileThumb
                v-else
                data-clipboard-file-media
                :history-id="item.id"
                :file-name="clipboardFileSummary(item).name"
              />
              <div class="flex min-w-0 items-baseline gap-2.5">
                <button
                  v-if="isClipboardVideoFile(item)"
                  data-clipboard-file-name
                  class="min-w-0 truncate text-left"
                  :class="clipboardFileNameClass(item)"
                  type="button"
                  @click.stop="openClipboardVideoPreview(item)"
                >
                  {{ clipboardFileSummary(item).name }}
                </button>
                <span
                  v-else
                  data-clipboard-file-name
                  class="min-w-0 truncate"
                  :class="clipboardFileNameClass(item)"
                >
                  {{ clipboardFileSummary(item).name }}
                </span>
                <span
                  v-if="clipboardFileSummary(item).size"
                  data-clipboard-file-size
                  class="shrink-0 text-[12px] text-[color:var(--clipboard-card-footer-text)]"
                >
                  {{ clipboardFileSummary(item).size }}
                </span>
              </div>
            </div>
            <button
              v-else-if="getClipboardLinkUrl(item.text)"
              data-clipboard-link-button
              class="mt-1.5 block w-fit max-w-full min-w-0 cursor-pointer select-none break-all text-left text-[13px] font-medium leading-[19px] underline-offset-2 transition-colors duration-150 hover:text-[color:var(--accent-text)] hover:underline"
              :class="[
                clipboardTextClass(getClipboardDisplayType(item)),
                isClipboardItemExpanded(item) ? 'whitespace-pre-wrap' : 'line-clamp-2',
                isClipboardItemExpandable(item) ? 'pr-14' : '',
              ]"
              type="button"
              @click.stop="openClipboardLink(item)"
            >
              {{ item.text }}
            </button>
            <p
              v-else
              data-clipboard-history-text
              class="mt-1.5 min-w-0 break-all text-[13px] font-medium leading-[19px]"
              :class="[
                clipboardTextClass(getClipboardDisplayType(item)),
                isClipboardItemExpanded(item) ? 'whitespace-pre-wrap' : 'line-clamp-2',
                isClipboardItemExpandable(item) ? 'pr-14' : '',
              ]"
            >
              {{ item.text }}
            </p>
            <button
              v-if="isClipboardItemExpandable(item)"
              data-clipboard-expand-button
              class="absolute bottom-0 right-0 inline-flex h-[19px] items-center rounded px-1.5 text-[11px] font-semibold text-[color:var(--clipboard-card-meta-text)] transition hover:bg-[color:var(--clipboard-card-soft-hover)] hover:text-[color:var(--clipboard-card-soft-hover-text)]"
              type="button"
              @click.stop="toggleClipboardItemExpanded(item)"
            >
              {{ isClipboardItemExpanded(item) ? "收起" : "展开" }}
            </button>
            </div>
            <div
              data-clipboard-card-footer-row
              class="mt-1 flex min-w-0 flex-wrap items-end justify-between gap-x-3 gap-y-1"
            >
              <div data-clipboard-card-footer class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-xs text-[color:var(--clipboard-card-footer-text)]">
                <p data-clipboard-card-time class="shrink-0">{{ clipboardTime(item.createdAt) }}</p>
                <span
                  v-if="item.sourceDevice"
                  data-clipboard-history-device
                  class="min-w-0 max-w-[9rem] truncate text-[color:var(--clipboard-card-footer-text)]"
                  :title="item.sourceDevice"
                >
                  · {{ item.sourceDevice }}
                </span>
              </div>
              <div
                data-clipboard-file-statuses
                class="flex shrink-0 items-center justify-end gap-1.5"
              >
                <ClipboardFileDownloadStatus
                  v-if="item.contentType === 'fileList'"
                  :item="item"
                  inline-badge
                />
                <span
                  data-clipboard-history-sync-status
                  class="shrink-0 rounded-full border px-2 py-0.5 text-[11px] font-semibold leading-5"
                  :class="syncStatusClass(item)"
                >
                  {{ syncStatusLabel(item) }}
                </span>
              </div>
            </div>
          </div>
        </article>
      </TransitionGroup>
      <p v-else class="rounded-xl border border-dashed border-[color:var(--main-line-soft)] px-3 py-8 text-center text-sm text-slate-500">
        暂无匹配内容。
      </p>
    </Card>

    <Transition name="trust-prompt">
      <div
        v-if="showClipboardHistoryModal"
        data-clipboard-history-modal
        class="fixed inset-0 z-50 flex items-center justify-center bg-[color:var(--dialog-overlay-bg)] px-6 py-8 backdrop-blur-sm"
        @click.self="showClipboardHistoryModal = false"
      >
        <section
          class="flex max-h-full w-full max-w-4xl flex-col rounded-xl border border-[color:var(--main-line)] bg-[color:var(--dialog-bg)] shadow-[0_24px_80px_rgba(0,0,0,0.5)]"
          role="dialog"
          aria-modal="true"
          aria-label="全部剪贴内容"
        >
          <div class="border-b border-[color:var(--main-line-soft)] px-5 py-4">
            <div class="flex items-start justify-between gap-4">
              <div>
                <p class="text-base font-semibold text-white">全部剪贴内容</p>
                <p class="mt-1 text-xs text-[color:var(--muted-text)]">
                  共 {{ allClipboardItems.length }} 条历史记录，当前显示 {{ filteredAllClipboardItems.length }} 条
                </p>
              </div>
              <button
                class="grid h-8 w-8 shrink-0 place-items-center rounded-md text-slate-300 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white"
                type="button"
                aria-label="关闭"
                title="关闭"
                @click="showClipboardHistoryModal = false"
              >
                <X class="h-4 w-4" />
              </button>
            </div>
            <div class="mt-4 grid gap-3">
              <label class="relative block">
                <Search class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500" />
                <input
                  v-model="clipboardSearch"
                  data-clipboard-search-input
                  class="h-10 w-full rounded-xl border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] pl-9 pr-3 text-sm text-slate-100 outline-none transition placeholder:text-slate-500 focus:border-[#4A5050] focus:ring-2 focus:ring-[#2F3333]"
                  type="search"
                  placeholder="搜索剪切板..."
                />
              </label>
              <div
                data-clipboard-category-tabs
                class="clipboard-category-tabs"
                :style="{
                  '--clipboard-category-index': activeClipboardCategoryIndex,
                  '--clipboard-category-count': clipboardCategories.length,
                }"
              >
                <span data-clipboard-category-indicator class="clipboard-category-indicator" />
                <button
                  v-for="category in clipboardCategories"
                  :key="category"
                  class="clipboard-category-chip"
                  :class="{ active: activeClipboardCategory === category }"
                  type="button"
                  @click="setActiveClipboardCategory(category)"
                >
                  {{ category }}
                </button>
              </div>
            </div>
          </div>

          <div v-if="filteredAllClipboardItems.length" class="min-h-0 overflow-x-hidden overflow-y-auto p-5">
            <TransitionGroup
              name="clipboard-card-stagger"
              tag="div"
              data-clipboard-stagger-list
              class="relative grid gap-3"
            >
              <article
                v-for="(item, index) in filteredAllClipboardItems"
                :key="item.id"
                data-clipboard-history-row
                class="clipboard-preview-card group relative min-h-[86px] overflow-hidden rounded-xl border border-[color:var(--clipboard-card-line)] bg-[color:var(--clipboard-card-bg)] px-5 py-2.5 shadow-[var(--clipboard-card-shadow)] transition duration-150 ease-out hover:z-10 hover:scale-[1.01] hover:border-[color:var(--clipboard-card-line-hover)] hover:bg-[color:var(--clipboard-card-bg-hover)] hover:shadow-[var(--clipboard-card-shadow-hover)]"
                :class="{
                  'cursor-pointer': isClipboardFileCardInteractive(item, historyStore.fileDownloadActivity(item.fileTransferId)),
                  'cursor-wait': isClipboardFileCardInteractive(item, historyStore.fileDownloadActivity(item.fileTransferId)) && historyStore.isFileDownloadActive(item.fileTransferId),
                }"
                :style="`--clipboard-row-index: ${index}`"
                @click="handleClipboardItemClick(item)"
              >
                <span class="clipboard-preview-card-accent absolute left-3 top-2.5 bottom-2.5 w-0.5 rounded-full transition duration-150 group-hover:brightness-125" :class="clipboardAccentClass(getClipboardDisplayType(item))" />
                <div data-clipboard-card-main class="min-w-0 pl-4">
                  <div data-clipboard-card-header class="flex min-w-0 items-start justify-between gap-3">
                    <span
                      data-clipboard-type-label
                      data-clipboard-card-meta
                      class="inline-flex items-center gap-1 text-[12px] font-medium"
                      :class="clipboardTypeClass(getClipboardDisplayType(item))"
                    >
                      <component :is="clipboardTypeIcon(getClipboardDisplayType(item))" class="h-3.5 w-3.5" />
                      {{ getClipboardDisplayType(item).label }}
                    </span>
                    <div data-clipboard-history-actions class="flex shrink-0 items-center gap-1.5">
                      <button
                        data-history-favorite
                        type="button"
                        class="clipboard-card-library-action clipboard-card-favorite-action"
                        :class="{ active: libraryStore.isHistoryItemSaved(item.id) }"
                        :disabled="isHistoryFavoriteBusy(item)"
                        :aria-label="libraryStore.isHistoryItemSaved(item.id) ? '移出收藏夹' : '收藏'"
                        @click.stop="toggleHistoryFavorite(item)"
                      >
                        <Star
                          class="h-3.5 w-3.5"
                          :fill="libraryStore.isHistoryItemSaved(item.id) ? 'currentColor' : 'none'"
                        />
                      </button>
                      <button
                        data-history-pin
                        type="button"
                        class="clipboard-card-library-action clipboard-card-pin-action"
                        :class="{ active: item.isPinned }"
                        :disabled="historyStore.isPinning(item.id)"
                        :aria-label="item.isPinned ? '取消置顶' : '置顶历史记录'"
                        @click.stop="toggleHistoryPin(item)"
                      >
                        <Pin
                          class="h-3.5 w-3.5"
                          :fill="item.isPinned ? 'currentColor' : 'none'"
                        />
                      </button>
                      <CopyTextButton
                        data-clipboard-card-action
                        class="clipboard-card-copy-action"
                        :text="item.text"
                        :content-type="item.contentType"
                        :history-item-id="item.id"
                        :file-transfer-id="item.fileTransferId"
                        :file-transfer-status="item.fileTransferStatus"
                        icon-only
                        label="复制内容"
                      />
                    </div>
                  </div>
                  <div data-clipboard-card-content class="relative min-w-0">
                  <div v-if="item.contentType === 'image'" class="mt-2 flex min-w-0 items-center gap-3">
                    <button
                      data-clipboard-image-preview-button
                      class="rounded-lg outline-none transition hover:scale-[1.03] focus-visible:ring-2 focus-visible:ring-[#60cdff]/60"
                      type="button"
                      aria-label="鏀惧ぇ棰勮鍥剧墖"
                      @click="openClipboardImagePreview(item)"
                    >
                      <HistoryImageThumb :history-id="item.id" />
                    </button>
                    <div
                      data-clipboard-image-summary
                      class="flex min-w-0 items-baseline gap-2.5 text-[13px] font-medium leading-[19px] text-[color:var(--clipboard-card-text)]"
                    >
                      <span data-clipboard-image-name class="min-w-0 truncate">
                        {{ clipboardFileSummary(item).name }}
                      </span>
                      <span
                        v-if="clipboardFileSummary(item).size"
                        data-clipboard-image-size
                        class="shrink-0 text-[12px] text-[color:var(--clipboard-card-footer-text)]"
                      >
                        {{ clipboardFileSummary(item).size }}
                      </span>
                    </div>
                  </div>
                  <div
                    v-else-if="item.contentType === 'fileList'"
                    data-clipboard-file-summary
                    class="mt-2 flex min-w-0 select-none items-center gap-3 text-[13px] font-medium leading-[19px] text-[color:var(--clipboard-card-text)]"
                  >
                    <button
                      v-if="isClipboardVideoFile(item)"
                      data-clipboard-file-media-button
                      class="rounded-lg outline-none transition hover:scale-[1.03] focus-visible:ring-2 focus-visible:ring-[#60cdff]/60"
                      type="button"
                      aria-label="预览视频"
                      @click.stop="openClipboardVideoPreview(item)"
                    >
                      <HistoryFileThumb
                        data-clipboard-file-media
                        :history-id="item.id"
                        :file-name="clipboardFileSummary(item).name"
                      />
                    </button>
                    <HistoryFileThumb
                      v-else
                      data-clipboard-file-media
                      :history-id="item.id"
                      :file-name="clipboardFileSummary(item).name"
                    />
                    <div class="flex min-w-0 items-baseline gap-2.5">
                      <button
                        v-if="isClipboardVideoFile(item)"
                        data-clipboard-file-name
                        class="min-w-0 truncate text-left"
                        :class="clipboardFileNameClass(item)"
                        type="button"
                        @click.stop="openClipboardVideoPreview(item)"
                      >
                        {{ clipboardFileSummary(item).name }}
                      </button>
                      <span
                        v-else
                        data-clipboard-file-name
                        class="min-w-0 truncate"
                        :class="clipboardFileNameClass(item)"
                      >
                        {{ clipboardFileSummary(item).name }}
                      </span>
                      <span
                        v-if="clipboardFileSummary(item).size"
                        data-clipboard-file-size
                        class="shrink-0 text-[12px] text-[color:var(--clipboard-card-footer-text)]"
                      >
                        {{ clipboardFileSummary(item).size }}
                      </span>
                    </div>
                  </div>
                  <button
                    v-else-if="getClipboardLinkUrl(item.text)"
                    data-clipboard-link-button
                    class="mt-1.5 block w-fit max-w-full min-w-0 cursor-pointer select-none break-all text-left text-[13px] font-medium leading-[19px] underline-offset-2 transition-colors duration-150 hover:text-[color:var(--accent-text)] hover:underline"
                    :class="[
                      clipboardTextClass(getClipboardDisplayType(item)),
                      isClipboardItemExpanded(item) ? 'whitespace-pre-wrap' : 'line-clamp-2',
                      isClipboardItemExpandable(item) ? 'pr-14' : '',
                    ]"
                    type="button"
                    @click.stop="openClipboardLink(item)"
                  >
                    {{ item.text }}
                  </button>
                  <p
                    v-else
                    data-clipboard-history-text
                    class="mt-1.5 min-w-0 break-all text-[13px] font-medium leading-[19px]"
                    :class="[
                      clipboardTextClass(getClipboardDisplayType(item)),
                      isClipboardItemExpanded(item) ? 'whitespace-pre-wrap' : 'line-clamp-2',
                      isClipboardItemExpandable(item) ? 'pr-14' : '',
                    ]"
                  >
                    {{ item.text }}
                  </p>
                  <button
                    v-if="isClipboardItemExpandable(item)"
                    data-clipboard-expand-button
                    class="absolute bottom-0 right-0 inline-flex h-[19px] items-center rounded px-1.5 text-[11px] font-semibold text-[color:var(--clipboard-card-meta-text)] transition hover:bg-[color:var(--clipboard-card-soft-hover)] hover:text-[color:var(--clipboard-card-soft-hover-text)]"
                    type="button"
                    @click.stop="toggleClipboardItemExpanded(item)"
                  >
                    {{ isClipboardItemExpanded(item) ? "收起" : "展开" }}
                  </button>
                  </div>
                  <div
                    data-clipboard-card-footer-row
                    class="mt-1 flex min-w-0 flex-wrap items-end justify-between gap-x-3 gap-y-1"
                  >
                    <div data-clipboard-card-footer class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-xs text-[color:var(--clipboard-card-footer-text)]">
                      <p data-clipboard-card-time class="shrink-0">{{ clipboardTime(item.createdAt) }}</p>
                      <span
                        v-if="item.sourceDevice"
                        data-clipboard-history-device
                        class="min-w-0 max-w-[9rem] truncate text-[color:var(--clipboard-card-footer-text)]"
                        :title="item.sourceDevice"
                      >
                        · {{ item.sourceDevice }}
                      </span>
                    </div>
                    <div
                      data-clipboard-file-statuses
                      class="flex shrink-0 items-center justify-end gap-1.5"
                    >
                      <ClipboardFileDownloadStatus
                        v-if="item.contentType === 'fileList'"
                        :item="item"
                        inline-badge
                      />
                      <span
                        data-clipboard-history-sync-status
                        class="shrink-0 rounded-full border px-2 py-0.5 text-[11px] font-semibold leading-5"
                        :class="syncStatusClass(item)"
                      >
                        {{ syncStatusLabel(item) }}
                      </span>
                    </div>
                  </div>
                </div>
              </article>
            </TransitionGroup>
          </div>
          <p v-else class="m-5 rounded-xl border border-dashed border-[color:var(--main-line-soft)] px-3 py-8 text-center text-sm text-[color:var(--subtle-text)]">
            暂无匹配内容。
          </p>
        </section>
      </div>
    </Transition>
    <Transition name="trust-prompt">
      <div
        v-if="previewImageItem"
        data-clipboard-image-preview-modal
        class="fixed inset-0 z-[60] flex items-center justify-center bg-black/70 px-6 py-8 backdrop-blur-md"
        @click.self="closeClipboardImagePreview"
      >
        <section
          class="relative grid max-h-full max-w-[min(92vw,1100px)] rounded-xl border border-[color:var(--main-line)] bg-[#202020] p-4 shadow-[0_24px_90px_rgba(0,0,0,0.62)]"
          role="dialog"
          aria-modal="true"
          aria-label="图片预览"
        >
          <button
            data-clipboard-image-preview-close
            class="absolute right-3 top-3 z-10 grid h-8 w-8 shrink-0 place-items-center rounded-md bg-black/20 text-slate-300 transition hover:bg-white/[0.08] hover:text-white"
            type="button"
            aria-label="关闭图片预览"
            title="关闭"
            @click="closeClipboardImagePreview"
          >
            <X class="h-4 w-4" />
          </button>
          <div
            class="grid max-h-[78vh] max-w-[86vw] place-items-center overflow-auto rounded-xl bg-black/20 p-2"
            :class="previewImageDrag.active.value ? 'cursor-grabbing' : 'cursor-grab'"
            data-clipboard-image-preview-zoom-area
            @wheel.prevent="handleClipboardImagePreviewWheel"
            @pointerdown="startClipboardImageDrag"
            @pointermove="moveClipboardImageDrag"
            @pointerup="endClipboardImageDrag"
            @pointercancel="endClipboardImageDrag"
          >
            <HistoryImageThumb
              :history-id="previewImageItem.id"
              :max-size="1400"
              variant="preview"
              :alt="previewImageItem.text"
              class="origin-center select-none transition-transform duration-75 ease-out"
              :style="previewImageTransform"
              draggable="false"
            />
          </div>
        </section>
      </div>
    </Transition>
    <Transition name="trust-prompt">
      <div
        v-if="previewVideoItem"
        data-clipboard-video-preview-modal
        class="fixed inset-0 z-[60] flex items-center justify-center bg-black/75 px-6 py-8 backdrop-blur-md"
        @click.self="closeClipboardVideoPreview"
      >
        <section
          class="relative grid w-full max-w-[min(92vw,1100px)] gap-3 rounded-xl border border-[color:var(--main-line)] bg-[#202020] p-4 shadow-[0_24px_90px_rgba(0,0,0,0.62)]"
          role="dialog"
          aria-modal="true"
          aria-label="视频预览"
        >
          <div class="flex min-w-0 items-center justify-between gap-3 pr-10">
            <div class="min-w-0">
              <p class="truncate text-sm font-semibold text-white">
                {{ clipboardFileSummary(previewVideoItem).name }}
              </p>
              <p class="mt-0.5 text-xs text-[color:var(--muted-text)]">
                本地视频预览
              </p>
            </div>
          </div>
          <button
            data-clipboard-video-preview-close
            class="absolute right-3 top-3 z-10 grid h-8 w-8 shrink-0 place-items-center rounded-md bg-black/20 text-slate-300 transition hover:bg-white/[0.08] hover:text-white"
            type="button"
            aria-label="关闭视频预览"
            title="关闭"
            @click="closeClipboardVideoPreview"
          >
            <X class="h-4 w-4" />
          </button>
          <div class="overflow-hidden rounded-xl bg-black shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]">
            <video
              v-if="previewVideoSrc"
              :src="previewVideoSrc"
              class="max-h-[74vh] w-full bg-black"
              preload="metadata"
              controls
              autoplay
              playsinline
              @loadedmetadata="handleClipboardVideoLoaded"
              @error="handleClipboardVideoPreviewError"
            />
          </div>
          <p
            v-if="previewVideoError"
            data-clipboard-video-preview-error
            class="rounded-lg border border-amber-300/20 bg-amber-400/[0.08] px-3 py-2 text-xs font-medium text-amber-100"
          >
            {{ previewVideoError }}
          </p>
        </section>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.clipboard-card-library-action {
  display: grid;
  width: 1.8rem;
  height: 1.8rem;
  place-items: center;
  border-radius: 0.4rem;
  color: var(--clipboard-card-meta-text);
  transition: 150ms ease;
}

.clipboard-card-library-action:hover:not(:disabled) {
  background: var(--main-bg-muted);
  color: white;
  transform: translateY(-1px) scale(1.08);
}

.clipboard-card-library-action:active:not(:disabled) {
  transform: scale(0.95);
}

.clipboard-card-library-action.active {
  background: var(--accent-soft);
  color: var(--accent-text);
}

.clipboard-card-favorite-action:hover:not(:disabled) {
  background: transparent;
  color: white;
}

.clipboard-card-favorite-action.active {
  background: transparent;
  color: white;
}

.clipboard-card-pin-action:hover:not(:disabled) {
  background: transparent;
  color: white;
}

.clipboard-card-pin-action.active {
  background: transparent;
  color: white;
}

.clipboard-card-copy-action {
  transition: transform 150ms ease, background-color 150ms ease, color 150ms ease;
}

.clipboard-card-copy-action:hover:not(:disabled) {
  transform: translateY(-1px) scale(1.08);
}

.clipboard-card-copy-action:active:not(:disabled) {
  transform: scale(0.95);
}

.clipboard-card-library-action:focus-visible {
  outline: 2px solid var(--accent-line);
  outline-offset: 2px;
}

.clipboard-card-library-action:disabled {
  cursor: wait;
  opacity: 0.45;
}
</style>
