<script setup lang="ts">
import { Clipboard, MoreHorizontal, RefreshCw, X } from "lucide-vue-next";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import ClipboardFileDownloadStatus from "@/components/history/ClipboardFileDownloadStatus.vue";
import CopyTextButton from "@/components/ui/CopyTextButton.vue";
import { resolveFloatingClipboardSelection } from "@/lib/floatingClipboardSelection";
import {
  FLOATING_CLIPBOARD_HISTORY_LIMIT,
  getClipboardLinkUrl,
  shouldShowClipboardItemMore,
  splitClipboardFileSummary,
  type ClipboardPreviewItem,
} from "@/lib/historyPreview";
import {
  FLOATING_CLIPBOARD_HISTORY_STORAGE_KEY,
  closeWindow,
  getConfig,
  type FloatingClipboardHistoryPayload,
  onAppEvent,
  openExternalUrl,
  startWindowDrag,
} from "@/lib/tauri";
import { startWindowDragFromMouseEvent } from "@/lib/windowDrag";
import type { AppConfig, AppTheme } from "@/types/config";

const clipboardItems = ref<ClipboardPreviewItem[]>([]);
const loading = ref(false);
const selectedClipboardItem = ref<ClipboardPreviewItem | null>(null);
let refreshUnlisten: UnlistenFn | null = null;
let themeUnlisten: UnlistenFn | null = null;
let isUnmounted = false;

const itemCountLabel = computed(() => `共 ${clipboardItems.value.length} 条记录`);

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
  selectedClipboardItem.value = resolveFloatingClipboardSelection(
    nextItems,
    selectedClipboardItem.value,
  );
  clipboardItems.value = nextItems;
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

function shouldShowFloatingClipboardHistoryItemMore(item: ClipboardPreviewItem) {
  return shouldShowClipboardItemMore(item, { textLimit: 42 });
}

async function openClipboardLink(item: ClipboardPreviewItem) {
  const url = getClipboardLinkUrl(item.text);
  if (!url) {
    openFullClipboardItem(item);
    return;
  }
  await openExternalUrl(url);
}

onMounted(() => {
  isUnmounted = false;
  void bindFloatingClipboardTheme();
  void bindRefreshEvents();
  void refreshFloatingClipboardItems();
});

onUnmounted(() => {
  isUnmounted = true;
  refreshUnlisten?.();
  refreshUnlisten = null;
  themeUnlisten?.();
  themeUnlisten = null;
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
          <p class="text-[11px] font-medium text-[color:var(--floating-muted-text)]">{{ itemCountLabel }}</p>
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
          v-for="item in clipboardItems"
          :key="item.id"
          data-floating-clipboard-history-row
          class="floating-clipboard-row grid grid-cols-[minmax(0,1fr)_auto] items-start gap-2 border-b border-[color:var(--floating-stat-line)] px-2 py-2.5 last:border-b-0"
        >
          <div data-floating-clipboard-history-content class="min-w-0 overflow-hidden">
            <button
              v-if="getClipboardLinkUrl(item.text)"
              data-floating-clipboard-link-button
              class="floating-link-chip block w-full min-w-0 overflow-hidden line-clamp-2 break-all text-left text-xs font-semibold leading-5 text-[color:var(--floating-strong-text)] underline-offset-2 hover:text-[color:var(--accent-text)] hover:underline"
              type="button"
              @click="openClipboardLink(item)"
            >
              {{ item.text }}
            </button>
            <div
              v-else-if="item.contentType === 'image'"
              data-floating-clipboard-image-summary
              class="flex min-w-0 items-baseline gap-2 text-xs font-semibold leading-5 text-[color:var(--floating-strong-text)]"
            >
              <span class="min-w-0 truncate">{{ clipboardFileName(item) }}</span>
              <span v-if="clipboardFileSize(item)" class="shrink-0 text-[10px] text-[color:var(--floating-muted-text)]">
                {{ clipboardFileSize(item) }}
              </span>
            </div>
            <div
              v-else-if="item.contentType === 'fileList'"
              data-floating-clipboard-file-summary
              class="flex min-w-0 items-baseline gap-2 text-xs font-semibold leading-5 text-[color:var(--floating-strong-text)]"
            >
              <span class="min-w-0 truncate">{{ clipboardFileName(item) }}</span>
              <span v-if="clipboardFileSize(item)" class="shrink-0 text-[10px] text-[color:var(--floating-muted-text)]">
                {{ clipboardFileSize(item) }}
              </span>
            </div>
            <p v-else data-floating-clipboard-history-text class="line-clamp-2 break-all text-xs font-semibold leading-5 text-[color:var(--floating-strong-text)]">
              {{ item.text }}
            </p>
            <p
              v-if="item.sourceDevice"
              class="mt-1 truncate text-[10px] font-medium text-[color:var(--floating-muted-text)]"
            >
              {{ item.sourceDevice }}
            </p>
          </div>
          <div data-floating-clipboard-actions class="flex shrink-0 items-center gap-1">
            <ClipboardFileDownloadStatus
              v-if="item.contentType === 'fileList'"
              :item="item"
              compact
            />
            <button
              v-if="shouldShowFloatingClipboardHistoryItemMore(item)"
              data-floating-clipboard-item-more-button
              class="grid h-7 w-7 place-items-center rounded-md border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-[color:var(--floating-control-bg-hover)]"
              type="button"
              title="查看完整内容"
              @click.stop="openFullClipboardItem(item)"
            >
              <MoreHorizontal class="h-3.5 w-3.5" />
            </button>
            <CopyTextButton
              :text="item.text"
              :content-type="item.contentType"
              :history-item-id="item.id"
              :file-transfer-id="item.fileTransferId"
              :file-transfer-status="item.fileTransferStatus"
              icon-only
              label="复制内容"
              copied-label="已复制"
            />
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
        <div class="mb-2 flex items-center justify-between gap-2">
          <p class="min-w-0 truncate text-sm font-semibold text-[color:var(--floating-strong-text)]">完整内容</p>
          <button
            class="grid h-7 w-7 shrink-0 place-items-center rounded-md border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-[color:var(--floating-control-bg-hover)]"
            type="button"
            title="关闭"
            @click="selectedClipboardItem = null"
          >
            <X class="h-3.5 w-3.5" />
          </button>
        </div>
        <pre class="min-h-0 flex-1 overflow-auto whitespace-pre-wrap break-all rounded-lg bg-black/30 p-3 text-xs leading-5 text-[color:var(--floating-strong-text)]">{{ selectedClipboardItem.text }}</pre>
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
