<script setup lang="ts">
import Clipboard from "lucide-vue-next/dist/esm/icons/clipboard.js";
import MoreHorizontal from "lucide-vue-next/dist/esm/icons/ellipsis.js";
import Minus from "lucide-vue-next/dist/esm/icons/minus.js";
import Pin from "lucide-vue-next/dist/esm/icons/pin.js";
import RefreshCw from "lucide-vue-next/dist/esm/icons/refresh-cw.js";
import X from "lucide-vue-next/dist/esm/icons/x.js";
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import ClipboardFileDownloadStatus from "@/components/history/ClipboardFileDownloadStatus.vue";
import { mediaPreviewItems } from "@/lib/mediaPreviewControls";
import HistoryFileThumb from "@/components/history/HistoryFileThumb.vue";
import HistoryImageThumb from "@/components/history/HistoryImageThumb.vue";
import CopyTextButton from "@/components/ui/CopyTextButton.vue";
import { vClipboardOverflow } from "@/lib/clipboardOverflow";
import { resolveFloatingClipboardSelection } from "@/lib/floatingClipboardSelection";
import { formatTime } from "@/lib/format";
import {
  FLOATING_CLIPBOARD_HISTORY_LIMIT,
  getClipboardLinkUrl,
  getClipboardDisplayType,
  getFloatingClipboardItems,
  isClipboardVideoFile,
  splitClipboardFileSummary,
  type ClipboardPreviewItem,
} from "@/lib/historyPreview";
import {
  FLOATING_CLIPBOARD_HISTORY_STORAGE_KEY,
  closeWindow,
  hideWindow,
  convertLocalFileSrc,
  getConfig,
  getHistoryFilePreviewPath,
  type FloatingClipboardHistoryPayload,
  onAppEvent,
  openExternalUrl,
  openMediaPreviewWindow,
  startWindowDrag,
} from "@/lib/tauri";
import { startWindowDragFromMouseEvent } from "@/lib/windowDrag";
import { useToastStore } from "@/stores/toasts";
import { useHistoryStore } from "@/stores/history";
import type { HistoryItem } from "@/types/history";
import type { AppConfig, AppTheme } from "@/types/config";

const toastStore = useToastStore();
const historyStore = useHistoryStore();
const clipboardItems = ref<ClipboardPreviewItem[]>([]);
const loading = ref(false);
const selectedClipboardItem = ref<ClipboardPreviewItem | null>(null);
const activeClipboardIndex = ref(0);
let refreshUnlisten: UnlistenFn | null = null;
let themeUnlisten: UnlistenFn | null = null;
let isUnmounted = false;
const historyUnlisteners: UnlistenFn[] = [];

const itemCountLabel = computed(() => `共 ${clipboardItems.value.length} 条记录`);
const activeClipboardItem = computed(() => clipboardItems.value[activeClipboardIndex.value] ?? null);

function applyFloatingClipboardTheme(theme: AppTheme) {
  document.documentElement.dataset.appTheme = theme;
  document.body.dataset.appTheme = theme;
}

async function bindFloatingClipboardTheme() {
  try {
    const config = await getConfig();
    if (!isUnmounted) {
      applyFloatingClipboardTheme(config.theme);
    }
  } catch (error) {
    console.warn("Unable to load floating clipboard theme", error);
  }

  const unlisten = await onAppEvent<AppConfig>("config-updated", (config) => {
    applyFloatingClipboardTheme(config.theme);
  });

  if (isUnmounted) {
    unlisten();
    return;
  }
  themeUnlisten = unlisten;
}

function readFloatingClipboardHistoryPayload(): FloatingClipboardHistoryPayload | null {
  const rawPayload = window.localStorage.getItem(FLOATING_CLIPBOARD_HISTORY_STORAGE_KEY);
  if (!rawPayload) {
    return null;
  }

  try {
    const payload = JSON.parse(rawPayload) as FloatingClipboardHistoryPayload;
    return Array.isArray(payload.items) ? payload : null;
  } catch (error) {
    console.warn("Unable to read floating clipboard payload", error);
    return null;
  }
}

function applyFloatingClipboardPayload(payload: FloatingClipboardHistoryPayload) {
  const nextItems = payload.items.slice(0, FLOATING_CLIPBOARD_HISTORY_LIMIT);
  const activeItemId = activeClipboardItem.value?.id;
  selectedClipboardItem.value = resolveFloatingClipboardSelection(
    nextItems,
    selectedClipboardItem.value,
  );
  clipboardItems.value = nextItems;
  const refreshedIndex = activeItemId
    ? nextItems.findIndex((item) => item.id === activeItemId)
    : -1;
  activeClipboardIndex.value = refreshedIndex >= 0
    ? refreshedIndex
    : Math.min(activeClipboardIndex.value, Math.max(nextItems.length - 1, 0));
}

function refreshFloatingClipboardItems() {
  loading.value = true;
  try {
    const payload = readFloatingClipboardHistoryPayload();
    if (payload) {
      applyFloatingClipboardPayload(payload);
    }
  } finally {
    loading.value = false;
  }
}

async function bindRefreshEvents() {
  const unlisten = await listen<FloatingClipboardHistoryPayload>("floating-clipboard-refresh", (event) => {
    if (event.payload) {
      applyFloatingClipboardPayload(event.payload);
      return;
    }
    refreshFloatingClipboardItems();
  });

  if (isUnmounted) {
    unlisten();
    return;
  }
  refreshUnlisten = unlisten;
  const historyListeners = await Promise.all([
    onAppEvent<HistoryItem[]>("history-updated", applyHistoryItems),
    onAppEvent<HistoryItem>("clipboard-synced", async () => {
      await historyStore.refresh();
      if (!isUnmounted && !historyStore.error) applyHistoryItems(historyStore.items);
    }),
  ]);
  if (isUnmounted) historyListeners.forEach((listener) => listener());
  else historyUnlisteners.push(...historyListeners);
}

function applyHistoryItems(items: HistoryItem[]) {
  const payload = {
    items: getFloatingClipboardItems(
      clipboardItems.value.filter((item) => !item.contentHash),
      items,
      FLOATING_CLIPBOARD_HISTORY_LIMIT,
    ),
  };
  window.localStorage.setItem(FLOATING_CLIPBOARD_HISTORY_STORAGE_KEY, JSON.stringify(payload));
  applyFloatingClipboardPayload(payload);
}

async function toggleHistoryPin(item: ClipboardPreviewItem) {
  try {
    await historyStore.setPinned(item.id, !item.isPinned);
    if (!isUnmounted) applyHistoryItems(historyStore.items);
  } catch (error) {
    toastStore.error(`置顶失败：${String(error)}`);
  }
}

function handleWindowDrag(event: MouseEvent) {
  startWindowDragFromMouseEvent(event, startWindowDrag);
}

function clipboardFileName(item: ClipboardPreviewItem) {
  return splitClipboardFileSummary(item.text).name;
}

function clipboardFileSize(item: ClipboardPreviewItem) {
  return splitClipboardFileSummary(item.text).size;
}

function openFullClipboardItem(item: ClipboardPreviewItem) {
  selectedClipboardItem.value = item;
}

async function openClipboardLink(item: ClipboardPreviewItem) {
  const url = getClipboardLinkUrl(item.text);
  if (!url) {
    openFullClipboardItem(item);
    return;
  }
  await openExternalUrl(url);
}

async function openHistoryImagePreview(item: ClipboardPreviewItem) {
  try {
    await openMediaPreviewWindow({
      kind: "image", historyId: item.id, title: clipboardFileName(item) || "图片预览", src: "",
      items: mediaPreviewItems(clipboardItems.value, "image"),
    });
  } catch (error) {
    toastStore.error(`无法预览图片：${String(error)}`);
  }
}

async function openHistoryVideoPreview(item: ClipboardPreviewItem) {
  try {
    const filePath = await getHistoryFilePreviewPath(item.id);
    await openMediaPreviewWindow({
      kind: "video",
      historyId: item.id,
      title: clipboardFileName(item) || "视频预览",
      src: convertLocalFileSrc(filePath),
      items: mediaPreviewItems(clipboardItems.value, "video"),
    });
  } catch (error) {
    toastStore.error(`无法预览视频，请先下载文件：${String(error)}`);
  }
}

function focusActiveClipboardRow() {
  void nextTick(() => {
    document
      .querySelector<HTMLElement>('[data-floating-clipboard-history-row][data-active="true"]')
      ?.scrollIntoView({ block: "nearest" });
  });
}

function moveActiveClipboardItem(offset: number) {
  if (!clipboardItems.value.length) return;
  const itemCount = clipboardItems.value.length;
  activeClipboardIndex.value = (activeClipboardIndex.value + offset + itemCount) % itemCount;
  focusActiveClipboardRow();
}

function copyActiveClipboardItem() {
  const row = document.querySelector<HTMLElement>(
    '[data-floating-clipboard-history-row][data-active="true"]',
  );
  row?.querySelector<HTMLButtonElement>("[data-quick-panel-copy] button")?.click();
}

function handleQuickPanelKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    if (selectedClipboardItem.value) {
      selectedClipboardItem.value = null;
    } else {
      void closeWindow();
    }
    return;
  }

  if (selectedClipboardItem.value) return;
  if (event.key === "ArrowDown") {
    event.preventDefault();
    moveActiveClipboardItem(1);
    return;
  }
  if (event.key === "ArrowUp") {
    event.preventDefault();
    moveActiveClipboardItem(-1);
    return;
  }
  if (event.key === "Enter") {
    const target = event.target instanceof HTMLElement ? event.target : null;
    if (target?.closest("button, input, textarea, select, a")) return;
    event.preventDefault();
    copyActiveClipboardItem();
  }
}

onMounted(() => {
  isUnmounted = false;
  void bindFloatingClipboardTheme();
  void bindRefreshEvents();
  void refreshFloatingClipboardItems();
  window.addEventListener("keydown", handleQuickPanelKeydown);
});

onUnmounted(() => {
  isUnmounted = true;
  refreshUnlisten?.();
  refreshUnlisten = null;
  themeUnlisten?.();
  themeUnlisten = null;
  historyUnlisteners.splice(0).forEach((unlisten) => unlisten());
  window.removeEventListener("keydown", handleQuickPanelKeydown);
});
</script>

<template>
  <section
    data-floating-clipboard-window
    class="floating-clipboard-history-surface flex h-screen w-screen flex-col overflow-hidden rounded-xl border border-[color:var(--floating-surface-line)] text-slate-100 shadow-[0_22px_70px_rgba(0,0,0,0.46)] backdrop-blur-2xl"
  >
    <header
      class="flex shrink-0 items-center justify-between gap-3 border-b border-[color:var(--main-line-soft)] px-3 py-2"
      data-window-drag-region
      @mousedown.capture="handleWindowDrag"
    >
      <div class="flex min-w-0 items-center gap-2" data-window-drag-region>
        <span class="grid h-8 w-8 shrink-0 place-items-center rounded-lg border border-[color:var(--floating-stat-line)] bg-[color:var(--floating-stat-bg)] text-[color:var(--accent-text)]">
          <Clipboard class="h-4 w-4" />
        </span>
        <div class="min-w-0" data-window-drag-region>
          <p class="truncate text-sm font-semibold text-[color:var(--floating-strong-text)]">剪贴板内容</p>
          <p class="text-[11px] font-medium text-[color:var(--floating-muted-text)]">
            {{ itemCountLabel }} · ↑↓ 选择 · Enter 复制
          </p>
        </div>
      </div>
      <div class="flex shrink-0 items-center gap-1.5">
        <button
          class="grid h-8 w-8 place-items-center rounded-lg border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-[color:var(--floating-control-bg-hover)]"
          type="button"
          title="刷新"
          data-window-control
          @click="refreshFloatingClipboardItems"
        >
          <RefreshCw class="h-4 w-4" :class="{ 'animate-spin': loading }" />
        </button>
        <button
          class="grid h-8 w-8 place-items-center rounded-lg border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-[color:var(--floating-control-bg-hover)]"
          type="button"
          aria-label="隐藏窗口"
          title="隐藏窗口"
          data-window-control
          @click="hideWindow"
        >
          <Minus class="h-4 w-4" />
        </button>
        <button
          class="grid h-8 w-8 place-items-center rounded-lg border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-red-500/75 hover:text-white"
          type="button"
          aria-label="关闭"
          title="关闭"
          data-window-control
          @click="closeWindow"
        >
          <X class="h-4 w-4" />
        </button>
      </div>
    </header>

    <main class="min-h-0 flex-1 overflow-y-auto p-3">
      <div v-if="clipboardItems.length" class="space-y-1">
        <article
          v-for="(item, index) in clipboardItems"
          :key="item.id"
          v-clipboard-overflow
          data-floating-clipboard-history-row
          :data-active="activeClipboardIndex === index"
          class="floating-clipboard-row grid grid-cols-[minmax(0,1fr)_auto] items-start gap-2 rounded-md border-b border-[color:var(--floating-stat-line)] px-2 py-2.5 transition last:border-b-0"
          @mouseenter="activeClipboardIndex = index"
          @mousedown="activeClipboardIndex = index"
        >
          <div data-floating-clipboard-history-content class="min-w-0 overflow-hidden">
            <div class="mb-1 flex items-center text-[10px] font-medium text-[color:var(--floating-muted-text)]">
              <span data-floating-clipboard-type class="shrink-0 rounded border border-[color:var(--floating-stat-line)] px-1.5 leading-4">{{ getClipboardDisplayType(item).label }}</span>
            </div>
            <button
              v-if="getClipboardLinkUrl(item.text)"
              data-i18n-ignore
              data-floating-clipboard-link-button
              data-clipboard-preview-text
              class="floating-link-chip w-full min-w-0 overflow-hidden line-clamp-2 break-all text-left text-xs font-semibold leading-5 text-[color:var(--floating-strong-text)] underline-offset-2 hover:text-[color:var(--accent-text)] hover:underline"
              type="button"
              @click="openClipboardLink(item)"
            >
              {{ item.text }}
            </button>
            <button
              v-else-if="item.contentType === 'image'"
              data-i18n-ignore
              data-floating-clipboard-image-summary
              data-floating-history-media-preview-button
              class="group/media flex w-full min-w-0 items-center gap-2.5 text-left text-xs font-semibold leading-5 text-[color:var(--floating-strong-text)]"
              type="button"
              title="预览图片"
              @click="openHistoryImagePreview(item)"
            >
              <HistoryImageThumb
                :history-id="item.id"
                :max-size="96"
                class="!h-10 !w-12 transition group-hover/media:ring-1 group-hover/media:ring-[color:var(--accent-line)]"
              />
              <span class="flex min-w-0 flex-1 items-baseline gap-2">
                <span class="min-w-0 truncate underline underline-offset-2">{{ clipboardFileName(item) }}</span>
                <span v-if="clipboardFileSize(item)" class="shrink-0 text-[10px] text-[color:var(--floating-muted-text)]">
                  {{ clipboardFileSize(item) }}
                </span>
              </span>
            </button>
            <button
              v-else-if="isClipboardVideoFile(item)"
              data-i18n-ignore
              data-floating-clipboard-file-summary
              data-floating-history-media-preview-button
              class="group/media flex w-full min-w-0 items-center gap-2.5 text-left text-xs font-semibold leading-5 text-[color:var(--floating-strong-text)]"
              type="button"
              title="预览视频"
              @click="openHistoryVideoPreview(item)"
            >
              <HistoryFileThumb
                :history-id="item.id"
                :file-name="clipboardFileName(item)"
                :max-size="96"
                compact
                class="transition group-hover/media:ring-1 group-hover/media:ring-[color:var(--accent-line)]"
              />
              <span class="flex min-w-0 flex-1 items-baseline gap-2">
                <span class="min-w-0 truncate underline underline-offset-2">{{ clipboardFileName(item) }}</span>
                <span v-if="clipboardFileSize(item)" class="shrink-0 text-[10px] text-[color:var(--floating-muted-text)]">
                  {{ clipboardFileSize(item) }}
                </span>
              </span>
            </button>
            <div
              v-else-if="item.contentType === 'fileList'"
              data-i18n-ignore
              data-floating-clipboard-file-summary
              class="flex min-w-0 items-baseline gap-2 text-xs font-semibold leading-5 text-[color:var(--floating-strong-text)]"
            >
              <span class="min-w-0 truncate underline underline-offset-2">{{ clipboardFileName(item) }}</span>
              <span v-if="clipboardFileSize(item)" class="shrink-0 text-[10px] text-[color:var(--floating-muted-text)]">
                {{ clipboardFileSize(item) }}
              </span>
            </div>
            <p v-else data-floating-clipboard-history-text data-clipboard-preview-text data-i18n-ignore class="line-clamp-2 break-all text-xs font-semibold leading-5 text-[color:var(--floating-strong-text)]">
              {{ item.text }}
            </p>
          </div>
          <div data-floating-clipboard-actions class="flex shrink-0 items-center gap-1">
            <button
              data-floating-history-pin
              class="grid h-7 w-7 place-items-center rounded-md border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] transition hover:bg-[color:var(--floating-control-bg-hover)] disabled:opacity-50"
              :class="item.isPinned ? 'text-[color:var(--accent-text)]' : 'text-[color:var(--floating-control-text)]'"
              type="button"
              :aria-label="item.isPinned ? '取消置顶' : '置顶历史记录'"
              :title="item.isPinned ? '取消置顶' : '置顶历史记录'"
              :aria-pressed="Boolean(item.isPinned)"
              :disabled="historyStore.isPinning(item.id)"
              @click.stop="toggleHistoryPin(item)"
            >
              <Pin class="h-3.5 w-3.5" :fill="item.isPinned ? 'currentColor' : 'none'" />
            </button>
            <ClipboardFileDownloadStatus
              v-if="item.contentType === 'fileList'"
              :item="item"
              compact
            />
            <button
              v-if="item.contentType === 'text'"
              style="display: none"
              data-floating-clipboard-item-more-button
              class="grid h-7 w-7 place-items-center rounded-md border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-[color:var(--floating-control-bg-hover)]"
              type="button"
              title="查看完整内容"
              @click.stop="openFullClipboardItem(item)"
            >
              <MoreHorizontal class="h-3.5 w-3.5" />
            </button>
            <span data-quick-panel-copy class="contents">
              <CopyTextButton
                :text="item.text"
                :content-type="item.contentType"
                :history-item-id="item.id"
                :file-transfer-id="item.fileTransferId"
                :file-transfer-file-id="item.fileTransferFileId"
                :file-transfer-status="item.fileTransferStatus"
                icon-only
                label="复制内容"
                copied-label="已复制"
              />
            </span>
          </div>
          <div data-floating-clipboard-footer class="col-span-2 flex min-w-0 items-center justify-between gap-2 text-[10px] font-medium text-[color:var(--floating-muted-text)]">
            <div class="flex min-w-0 items-center gap-2">
              <time v-if="item.createdAt" data-floating-clipboard-time :datetime="item.createdAt" :title="new Date(item.createdAt).toLocaleString()" class="shrink-0 tabular-nums">{{ formatTime(item.createdAt) }}</time>
              <span v-if="item.sourceDevice" data-floating-clipboard-user data-i18n-ignore class="min-w-0 truncate">{{ item.sourceDevice }}</span>
            </div>
            <span data-floating-clipboard-sync-status class="shrink-0" :class="item.syncStatus === 'synced' ? 'text-emerald-300' : 'text-amber-300'">{{ item.syncStatus === 'synced' ? '已同步' : '未同步' }}</span>
          </div>
        </article>
      </div>
      <p v-else class="rounded-lg border border-[color:var(--floating-stat-line)] bg-[color:var(--floating-stat-bg)] px-4 py-10 text-center text-xs font-semibold text-[color:var(--floating-muted-text)]">
        暂无剪贴板内容
      </p>
    </main>

    <Transition name="trust-prompt">
      <div
        v-if="selectedClipboardItem"
        data-floating-clipboard-full-content
        class="floating-clipboard-history-surface absolute inset-3 z-20 flex flex-col overflow-hidden rounded-xl border border-[color:var(--floating-control-line)] p-3 shadow-[0_18px_46px_rgba(0,0,0,0.45)] backdrop-blur-xl"
      >
        <header
          class="mb-2 flex cursor-move select-none items-center justify-between gap-2"
          data-window-drag-region
          @mousedown.capture="handleWindowDrag"
        >
          <p class="min-w-0 truncate text-sm font-semibold text-[color:var(--floating-strong-text)]">完整内容</p>
          <button
            class="grid h-7 w-7 shrink-0 place-items-center rounded-md border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-[color:var(--floating-control-bg-hover)]"
            type="button"
            title="关闭"
            @click="selectedClipboardItem = null"
          >
            <X class="h-3.5 w-3.5" />
          </button>
        </header>
        <pre data-floating-clipboard-full-text data-i18n-ignore class="min-h-0 flex-1 overflow-auto whitespace-pre-wrap break-all rounded-lg bg-black/30 p-3 text-xs leading-5 text-[color:var(--floating-strong-text)]">{{ selectedClipboardItem.text }}</pre>
        <div class="mt-2 flex justify-end">
          <CopyTextButton
            :text="selectedClipboardItem.text"
            content-type="text"
            label="复制完整内容"
            copied-label="已复制"
          />
        </div>
      </div>
    </Transition>
  </section>
</template>
