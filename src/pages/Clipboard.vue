<script setup lang="ts">
import {
  Clipboard as ClipboardIcon,
  File,
  Image as ImageIcon,
  Link2,
  Search,
  X,
} from "lucide-vue-next";
import { computed, ref } from "vue";

import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import CopyTextButton from "@/components/ui/CopyTextButton.vue";
import HistoryImageThumb from "@/components/history/HistoryImageThumb.vue";
import {
  CLIPBOARD_CATEGORIES,
  CLIPBOARD_PREVIEW_LIMIT,
  filterClipboardItems,
  getClipboardDisplayType,
  getRecentClipboardItems,
  type ClipboardCategory,
  type ClipboardDisplayType,
  type ClipboardPreviewItem,
} from "@/lib/historyPreview";
import { copyHistoryItem } from "@/lib/tauri";
import { useHistoryStore } from "@/stores/history";
import { useToastStore } from "@/stores/toasts";

const historyStore = useHistoryStore();
const toastStore = useToastStore();
const showClipboardHistoryModal = ref(false);
const clipboardSearch = ref("");
const activeClipboardCategory = ref<ClipboardCategory>(CLIPBOARD_CATEGORIES[0]);
const clipboardCategories = CLIPBOARD_CATEGORIES;
const expandedClipboardItemIds = ref<Set<string>>(new Set());
const previewImageItem = ref<ClipboardPreviewItem | null>(null);
const previewImageScale = ref(1);
const activeClipboardCategoryIndex = computed(() =>
  Math.max(0, clipboardCategories.indexOf(activeClipboardCategory.value)),
);
const previewImageTransform = computed(() => ({
  transform: `scale(${previewImageScale.value})`,
}));

const recentSyncItems = computed(() => getRecentClipboardItems(historyStore.items));
const allClipboardItems = computed(() =>
  getRecentClipboardItems(historyStore.items, historyStore.items.length),
);
const filteredRecentSyncItems = computed(() =>
  filterClipboardItems(recentSyncItems.value, activeClipboardCategory.value, ""),
);
const filteredAllClipboardItems = computed(() =>
  filterClipboardItems(allClipboardItems.value, activeClipboardCategory.value, clipboardSearch.value),
);

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
  return item.text.length > 80 || item.text.includes("\n");
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
  previewImageItem.value = item;
}

function closeClipboardImagePreview() {
  previewImageItem.value = null;
  previewImageScale.value = 1;
}

async function handleClipboardItemClick(item: ClipboardPreviewItem) {
  if (item.contentType !== "fileList") {
    return;
  }

  try {
    const result = await copyHistoryItem(item.id);
    if (result === "downloadStarted") {
      toastStore.success("开始下载，完成后会写入剪贴板");
    } else if (result === "downloading") {
      toastStore.success("文件正在下载");
    } else {
      toastStore.success("文件已复制");
    }
  } catch {
    toastStore.error("文件复制失败");
  }
}

function handleClipboardImagePreviewWheel(event: WheelEvent) {
  const delta = event.deltaY < 0 ? 0.12 : -0.12;
  const next = Math.min(3, Math.max(0.35, previewImageScale.value + delta));
  previewImageScale.value = Number(next.toFixed(2));
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
  };
  return icons[type.tone];
}

function clipboardAccentClass(type: ClipboardDisplayType) {
  const classes = {
    text: "bg-[#007aff]",
    image: "bg-[#34a851]",
    link: "bg-[#7b5520]",
    file: "bg-[#af52de]",
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
          :class="{ 'cursor-pointer': item.contentType === 'fileList' }"
          :style="`--clipboard-row-index: ${index}`"
          @click="handleClipboardItemClick(item)"
        >
          <span class="clipboard-preview-card-accent absolute left-3 top-2.5 bottom-2.5 w-0.5 rounded-full transition duration-150 group-hover:brightness-125" :class="clipboardAccentClass(getClipboardDisplayType(item))" />
          <div data-clipboard-card-main class="min-w-0 pl-4 pr-32">
            <span
              data-clipboard-type-label
              data-clipboard-card-meta
              class="inline-flex items-center gap-1 text-[12px] font-medium"
              :class="clipboardTypeClass(getClipboardDisplayType(item))"
            >
              <component :is="clipboardTypeIcon(getClipboardDisplayType(item))" class="h-3.5 w-3.5" />
              {{ getClipboardDisplayType(item).label }}
            </span>
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
              <p data-clipboard-history-text class="min-w-0 text-[13px] font-medium leading-[19px] text-[color:var(--clipboard-card-text)]">
                {{ item.text }}
              </p>
            </div>
            <p
              v-else
              data-clipboard-history-text
              class="mt-1.5 min-w-0 break-all text-[13px] font-medium leading-[19px]"
              :class="[clipboardTextClass(getClipboardDisplayType(item)), isClipboardItemExpanded(item) ? 'whitespace-pre-wrap' : 'line-clamp-2']"
            >
              {{ item.text }}
            </p>
            <button
              v-if="isClipboardItemExpandable(item)"
              data-clipboard-expand-button
              class="mt-1 inline-flex rounded-full px-2 py-0.5 text-[11px] font-semibold text-[color:var(--clipboard-card-meta-text)] transition hover:bg-[color:var(--clipboard-card-soft-hover)] hover:text-[color:var(--clipboard-card-soft-hover-text)]"
              type="button"
              @click="toggleClipboardItemExpanded(item)"
            >
              {{ isClipboardItemExpanded(item) ? "收起" : "展开" }}
            </button>
            <div data-clipboard-card-footer class="mt-1 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-xs text-[color:var(--clipboard-card-footer-text)]">
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
          </div>
          <div data-clipboard-history-actions class="absolute right-2.5 top-2.5 flex items-center gap-2">
            <CopyTextButton
              data-clipboard-card-action
              :text="item.text"
              :content-type="item.contentType"
              :history-item-id="item.id"
              icon-only
              label="复制内容"
            />
          </div>
          <span
            data-clipboard-history-sync-status
            class="absolute bottom-2.5 right-3 shrink-0 rounded-full border px-2 py-0.5 text-[11px] font-semibold leading-5"
            :class="syncStatusClass(item)"
          >
            {{ syncStatusLabel(item) }}
          </span>
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
                :class="{ 'cursor-pointer': item.contentType === 'fileList' }"
                :style="`--clipboard-row-index: ${index}`"
                @click="handleClipboardItemClick(item)"
              >
                <span class="clipboard-preview-card-accent absolute left-3 top-2.5 bottom-2.5 w-0.5 rounded-full transition duration-150 group-hover:brightness-125" :class="clipboardAccentClass(getClipboardDisplayType(item))" />
                <div data-clipboard-card-main class="min-w-0 pl-4 pr-32">
                  <span
                    data-clipboard-type-label
                    data-clipboard-card-meta
                    class="inline-flex items-center gap-1 text-[12px] font-medium"
                    :class="clipboardTypeClass(getClipboardDisplayType(item))"
                  >
                    <component :is="clipboardTypeIcon(getClipboardDisplayType(item))" class="h-3.5 w-3.5" />
                    {{ getClipboardDisplayType(item).label }}
                  </span>
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
                    <p data-clipboard-history-text class="min-w-0 text-[13px] font-medium leading-[19px] text-[color:var(--clipboard-card-text)]">
                      {{ item.text }}
                    </p>
                  </div>
                  <p
                    v-else
                    data-clipboard-history-text
                    class="mt-1.5 min-w-0 break-all text-[13px] font-medium leading-[19px]"
                    :class="[clipboardTextClass(getClipboardDisplayType(item)), isClipboardItemExpanded(item) ? 'whitespace-pre-wrap' : 'line-clamp-2']"
                  >
                    {{ item.text }}
                  </p>
                  <button
                    v-if="isClipboardItemExpandable(item)"
                    data-clipboard-expand-button
                    class="mt-1 inline-flex rounded-full px-2 py-0.5 text-[11px] font-semibold text-[color:var(--clipboard-card-meta-text)] transition hover:bg-[color:var(--clipboard-card-soft-hover)] hover:text-[color:var(--clipboard-card-soft-hover-text)]"
                    type="button"
                    @click="toggleClipboardItemExpanded(item)"
                  >
                    {{ isClipboardItemExpanded(item) ? "收起" : "展开" }}
                  </button>
                  <div data-clipboard-card-footer class="mt-1 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-xs text-[color:var(--clipboard-card-footer-text)]">
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
                </div>
                <div data-clipboard-history-actions class="absolute right-2.5 top-2.5 flex items-center gap-2">
                  <CopyTextButton
                    data-clipboard-card-action
                    :text="item.text"
                    :content-type="item.contentType"
                    :history-item-id="item.id"
                    icon-only
                    label="复制内容"
                  />
                </div>
                <span
                  data-clipboard-history-sync-status
                  class="absolute bottom-2.5 right-3 shrink-0 rounded-full border px-2 py-0.5 text-[11px] font-semibold leading-5"
                  :class="syncStatusClass(item)"
                >
                  {{ syncStatusLabel(item) }}
                </span>
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
            data-clipboard-image-preview-zoom-area
            @wheel.prevent="handleClipboardImagePreviewWheel"
          >
            <HistoryImageThumb
              :history-id="previewImageItem.id"
              :max-size="1400"
              variant="preview"
              :alt="previewImageItem.text"
              class="origin-center transition-transform duration-75 ease-out"
              :style="previewImageTransform"
            />
          </div>
        </section>
      </div>
    </Transition>
  </div>
</template>
