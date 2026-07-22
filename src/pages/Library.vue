<script setup lang="ts">
import Bookmarks from "lucide-vue-next/dist/esm/icons/bookmark.js";
import Filter from "lucide-vue-next/dist/esm/icons/funnel.js";
import LayoutGrid from "lucide-vue-next/dist/esm/icons/layout-grid.js";
import List from "lucide-vue-next/dist/esm/icons/list.js";
import MessageSquareText from "lucide-vue-next/dist/esm/icons/message-square-text.js";
import Plus from "lucide-vue-next/dist/esm/icons/plus.js";
import Search from "lucide-vue-next/dist/esm/icons/search.js";
import Sparkles from "lucide-vue-next/dist/esm/icons/sparkles.js";
import { storeToRefs } from "pinia";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";

import LibraryCard from "@/components/library/LibraryCard.vue";
import LibraryMetadataDialog from "@/components/library/LibraryMetadataDialog.vue";
import SnippetEditorDialog from "@/components/library/SnippetEditorDialog.vue";
import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import {
  readLibraryLayout,
  writeLibraryLayout,
  type LibraryLayout,
} from "@/lib/libraryLayout";
import { getLibraryStorageSize } from "@/lib/tauri";
import { useLibraryStore } from "@/stores/library";
import { useToastStore } from "@/stores/toasts";
import type {
  CreateSnippetInput,
  LibraryContentFilter,
  LibraryItem,
  LibraryItemUpdate,
  LibraryView,
} from "@/types/library";

const libraryStore = useLibraryStore();
const toastStore = useToastStore();
const {
  items,
  warning,
  loading,
  query,
  activeView,
  contentTypeFilter,
  selectedTags,
  filteredItems,
  availableTags,
} = storeToRefs(libraryStore);

const views: Array<{ value: LibraryView; label: string }> = [
  { value: "snippets", label: "常用片段" },
  { value: "all", label: "全部收藏" },
];
const activeHeader = computed(() => ({
  snippets: {
    title: "常用片段",
    description: "快速保存和复用高频文本内容。",
    icon: MessageSquareText,
  },
  all: {
    title: "收藏夹",
    description: "长期保存常用内容，不受剪贴板历史清理影响。",
    icon: Bookmarks,
  },
})[activeView.value]);
const typeFilters: Array<{ value: LibraryContentFilter; label: string }> = [
  { value: "all", label: "全部类型" },
  { value: "text", label: "文本" },
  { value: "image", label: "图片" },
  { value: "fileList", label: "文件" },
];

const storageSize = ref(0);
const libraryLayout = ref<LibraryLayout>(readLibraryLayout());
const snippetItem = ref<LibraryItem | null>(null);
const snippetOpen = ref(false);
const metadataItem = ref<LibraryItem | null>(null);
const metadataOpen = ref(false);
const draggedPinnedId = ref<string | null>(null);
let storageRefreshId = 0;

function setLibraryLayout(layout: LibraryLayout) {
  libraryLayout.value = layout;
  writeLibraryLayout(layout);
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = units[0];
  for (const next of units.slice(1)) {
    if (value < 1024) break;
    value /= 1024;
    unit = next;
  }
  return `${value >= 10 ? value.toFixed(1) : value.toFixed(2)} ${unit}`;
}

async function refreshStorage() {
  const requestId = ++storageRefreshId;
  try {
    const nextSize = await getLibraryStorageSize();
    if (requestId !== storageRefreshId) return;
    storageSize.value = nextSize;
  } catch {
    // Keep the last successful measurement; library actions remain usable.
  }
}

watch(
  () => items.value
    .map((item) => `${item.id}:${item.updatedAt}:${item.assets.map((asset) => `${asset.assetId}:${asset.size}`).join(",")}`)
    .join("|"),
  () => void refreshStorage(),
  { immediate: true },
);

function openNewSnippet() {
  snippetItem.value = null;
  snippetOpen.value = true;
}

function openSnippetEditor(item: LibraryItem) {
  snippetItem.value = item;
  snippetOpen.value = true;
}

function openMetadataEditor(item: LibraryItem) {
  metadataItem.value = item;
  metadataOpen.value = true;
}

async function saveSnippet(input: CreateSnippetInput) {
  try {
    if (snippetItem.value) {
      await libraryStore.updateItem(snippetItem.value.id, input);
      toastStore.success("片段已更新");
    } else {
      await libraryStore.createSnippet(input);
      toastStore.success("片段已创建");
    }
    snippetOpen.value = false;
  } catch (error) {
    toastStore.error(`保存片段失败：${String(error)}`);
  }
}

async function saveMetadata(update: LibraryItemUpdate) {
  if (!metadataItem.value) return;
  try {
    await libraryStore.updateItem(metadataItem.value.id, update);
    metadataOpen.value = false;
    toastStore.success("收藏信息已更新");
  } catch (error) {
    toastStore.error(`更新收藏失败：${String(error)}`);
  }
}

async function copyItem(item: LibraryItem) {
  try {
    await libraryStore.copyItem(item.id);
    toastStore.success("已复制");
  } catch (error) {
    toastStore.error(`复制失败：${String(error)}`);
  }
}

async function togglePin(item: LibraryItem) {
  try {
    await libraryStore.setPinned(item.id, !item.isPinned);
    toastStore.success(item.isPinned ? "已取消置顶" : "已置顶");
  } catch (error) {
    toastStore.error(`置顶操作失败：${String(error)}`);
  }
}

async function convertSnippet(item: LibraryItem) {
  try {
    await libraryStore.convertToSnippet(item.id);
    toastStore.success("已转换为常用片段");
  } catch (error) {
    toastStore.error(`转换失败：${String(error)}`);
  }
}

async function removeItem(item: LibraryItem) {
  if (!window.confirm(`确定移出“${item.title}”吗？`)) return;
  try {
    await libraryStore.removeItem(item.id);
    toastStore.success("已移出收藏夹");
  } catch (error) {
    toastStore.error(`移出收藏失败：${String(error)}`);
  }
}

function toggleTag(tag: string) {
  selectedTags.value = selectedTags.value.includes(tag)
    ? selectedTags.value.filter((value) => value !== tag)
    : [...selectedTags.value, tag];
}

async function dropPinnedItem(targetId: string) {
  const sourceId = draggedPinnedId.value;
  draggedPinnedId.value = null;
  if (!sourceId || sourceId === targetId) return;
  const ids = items.value
    .filter((item) => item.isPinned)
    .map((item) => item.id);
  const sourceIndex = ids.indexOf(sourceId);
  const targetIndex = ids.indexOf(targetId);
  if (sourceIndex < 0 || targetIndex < 0) return;
  ids.splice(targetIndex, 0, ids.splice(sourceIndex, 1)[0]);
  try {
    await libraryStore.reorderPinned(ids);
  } catch (error) {
    toastStore.error(`调整置顶顺序失败：${String(error)}`);
  }
}

onMounted(async () => {
  try {
    await Promise.all([
      libraryStore.load(),
      libraryStore.subscribe(),
    ]);
  } catch (error) {
    toastStore.error(`收藏夹加载失败：${String(error)}`);
  }
});

onUnmounted(() => libraryStore.disposeSubscription());
</script>

<template>
  <div data-library-page class="grid gap-4 pb-4 text-[13px]">
    <section class="relative overflow-hidden rounded-[14px] border border-[color:var(--main-line)] bg-[color:var(--panel-bg)] p-4">
      <div class="absolute inset-y-0 right-0 w-40 bg-gradient-to-l from-[color:var(--accent-soft)] to-transparent opacity-60" />
      <div class="relative flex flex-wrap items-start justify-between gap-4">
        <div class="flex min-w-0 items-start gap-3">
          <div class="grid h-10 w-10 shrink-0 place-items-center rounded-xl border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]">
            <component :is="activeHeader.icon" class="h-5 w-5" />
          </div>
          <div>
            <h1 class="text-lg font-bold text-white">{{ activeHeader.title }}</h1>
            <p class="mt-1 text-[13px] leading-5 text-[color:var(--muted-text)]">
              {{ activeHeader.description }}
            </p>
          </div>
        </div>
        <Button
          v-if="activeView === 'snippets'"
          data-library-new-snippet
          variant="primary"
          size="sm"
          @click="openNewSnippet"
        >
          <Plus class="h-4 w-4" />
          新建文本片段
        </Button>
      </div>
    </section>

    <Card compact class="grid gap-4">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="library-view-switch">
          <button
            v-for="view in views"
            :key="view.value"
            :data-library-view-all="view.value === 'all' || undefined"
            :data-library-view-snippets="view.value === 'snippets' || undefined"
            type="button"
            class="library-view-button"
            :class="{ active: activeView === view.value }"
            @click="activeView = view.value"
          >
            {{ view.label }}
          </button>
        </div>
        <div class="flex items-center gap-2">
          <span data-library-storage-size class="text-[12px] text-[color:var(--muted-text)]">
            {{ items.length }} 项 · {{ formatBytes(storageSize) }}
          </span>
          <div class="library-layout-switch" aria-label="布局显示方式">
            <button
              data-library-layout-grid
              type="button"
              class="library-layout-button"
              :aria-pressed="libraryLayout === 'grid'"
              title="块状显示"
              @click="setLibraryLayout('grid')"
            >
              <LayoutGrid class="h-3.5 w-3.5" />
            </button>
            <button
              data-library-layout-list
              type="button"
              class="library-layout-button"
              :aria-pressed="libraryLayout === 'list'"
              title="列表显示"
              @click="setLibraryLayout('list')"
            >
              <List class="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      </div>

      <div class="grid gap-3 md:grid-cols-[minmax(0,1fr)_120px]">
        <label class="relative min-w-0">
          <Search class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500" />
          <input
            v-model="query"
            data-library-search
            class="h-10 w-full rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] pl-9 pr-3 text-white outline-none placeholder:text-slate-500 focus:border-[color:var(--accent-line)]"
            placeholder="搜索标题、正文、标签或备注"
          />
        </label>
        <label class="relative">
          <Filter class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500" />
          <select
            v-model="contentTypeFilter"
            data-library-type-filter
            class="h-10 w-full appearance-none rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] pl-9 pr-3 font-medium text-white outline-none focus:border-[color:var(--accent-line)]"
          >
            <option v-for="filter in typeFilters" :key="filter.value" :value="filter.value">
              {{ filter.label }}
            </option>
          </select>
        </label>
      </div>

      <div v-if="availableTags.length" data-library-tag-filter class="flex flex-wrap gap-2">
        <span
          data-library-tag-label
          class="self-center whitespace-nowrap text-[11px] font-semibold text-[color:var(--muted-text)]"
        >
          标签
        </span>
        <button
          v-for="tag in availableTags"
          :key="tag"
          type="button"
          class="rounded-full border px-2.5 py-1 text-[11px] font-semibold transition"
          :class="selectedTags.includes(tag)
            ? 'border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]'
            : 'border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] text-slate-400 hover:text-white'"
          @click="toggleTag(tag)"
        >
          {{ tag }}
        </button>
      </div>
    </Card>

    <p
      v-if="warning"
      data-library-warning
      class="rounded-lg border border-amber-500/35 bg-amber-500/10 px-3 py-2 text-[12px] leading-5 text-amber-100"
    >
      {{ warning }}
    </p>

    <div
      v-if="filteredItems.length"
      data-library-list
      :data-library-layout="libraryLayout"
      :class="libraryLayout === 'grid'
        ? 'grid gap-3 md:grid-cols-2 2xl:grid-cols-3'
        : 'grid gap-2'"
    >
      <LibraryCard
        v-for="item in filteredItems"
        :key="item.id"
        :item="item"
        :busy="libraryStore.isItemBusy(item.id)"
        :layout="libraryLayout"
        :draggable="item.isPinned"
        @dragstart="draggedPinnedId = item.isPinned ? item.id : null"
        @dragend="draggedPinnedId = null"
        @dragover.prevent
        @drop.prevent="dropPinnedItem(item.id)"
        @copy="copyItem"
        @pin="togglePin"
        @edit="openMetadataEditor"
        @convert-snippet="convertSnippet"
        @edit-snippet="openSnippetEditor"
        @remove="removeItem"
      />
    </div>

    <Card
      v-else
      data-library-empty
      class="grid min-h-52 place-items-center text-center"
    >
      <div class="grid justify-items-center gap-3">
        <div class="grid h-12 w-12 place-items-center rounded-xl border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] text-slate-500">
          <Sparkles class="h-5 w-5" />
        </div>
        <div>
          <p class="font-bold text-white">{{ loading ? "正在加载收藏夹" : "这里还没有匹配的内容" }}</p>
          <p class="mt-1 text-[12px] text-[color:var(--muted-text)]">
            {{ items.length ? "调整搜索或筛选条件。" : "从剪贴板历史收藏内容，或新建一个文本片段。" }}
          </p>
        </div>
      </div>
    </Card>

    <SnippetEditorDialog
      :open="snippetOpen"
      :item="snippetItem"
      @submit="saveSnippet"
      @cancel="snippetOpen = false"
    />
    <LibraryMetadataDialog
      :open="metadataOpen"
      :item="metadataItem"
      @submit="saveMetadata"
      @cancel="metadataOpen = false"
    />
  </div>
</template>

<style scoped>
.library-view-switch {
  display: inline-grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.2rem;
  border: 1px solid var(--main-line-soft);
  border-radius: 0.65rem;
  background: var(--field-bg);
  padding: 0.2rem;
}

.library-view-button {
  min-height: 2rem;
  border-radius: 0.45rem;
  padding: 0 0.75rem;
  color: rgb(148 163 184);
  font-size: 0.75rem;
  font-weight: 700;
  transition: 150ms ease;
}

.library-view-button:hover {
  color: white;
}

.library-view-button.active {
  background: var(--main-bg-muted);
  color: var(--accent-text);
  box-shadow: inset 0 0 0 1px var(--main-line);
}

.library-layout-switch {
  display: inline-flex;
  gap: 0.15rem;
  border: 1px solid var(--main-line-soft);
  border-radius: 0.55rem;
  background: var(--field-bg);
  padding: 0.15rem;
}

.library-layout-button {
  display: grid;
  width: 1.9rem;
  height: 1.9rem;
  place-items: center;
  border-radius: 0.4rem;
  color: rgb(148 163 184);
  transition: 150ms ease;
}

.library-layout-button:hover {
  color: white;
}

.library-layout-button[aria-pressed="true"] {
  background: var(--main-bg-muted);
  color: var(--accent-text);
}

.library-layout-button:focus-visible {
  outline: 2px solid var(--accent-line);
  outline-offset: 2px;
}
</style>
