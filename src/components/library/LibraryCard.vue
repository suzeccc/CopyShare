<script setup lang="ts">
import {
  Copy,
  FileText,
  Files,
  GripVertical,
  Image as ImageIcon,
  MessageSquareText,
  Pencil,
  Pin,
  PinOff,
  Trash2,
} from "lucide-vue-next";
import { computed, ref, watch } from "vue";

import type { LibraryLayout } from "@/lib/libraryLayout";
import { getLibraryImageThumbnail } from "@/lib/tauri";
import type { LibraryItem } from "@/types/library";

const props = withDefaults(defineProps<{
  item: LibraryItem;
  busy: boolean;
  layout?: LibraryLayout;
}>(), {
  layout: "grid",
});

const emit = defineEmits<{
  copy: [LibraryItem];
  pin: [LibraryItem];
  edit: [LibraryItem];
  remove: [LibraryItem];
  "convert-snippet": [LibraryItem];
  "edit-snippet": [LibraryItem];
}>();

const thumbnail = ref("");
const unavailable = ref("");
const canEditSnippet = computed(() => props.item.role === "snippet");
const canConvertSnippet = computed(() =>
  props.item.role === "saved" && props.item.contentType === "text",
);
const previewText = computed(() => props.item.content || props.item.summary);
const typeLabel = computed(() => ({
  text: props.item.role === "snippet" ? "常用片段" : "文本",
  image: "图片",
  fileList: "文件",
})[props.item.contentType]);
const typeIcon = computed(() => ({
  text: props.item.role === "snippet" ? MessageSquareText : FileText,
  image: ImageIcon,
  fileList: Files,
})[props.item.contentType]);

function formatTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

async function loadThumbnail() {
  thumbnail.value = "";
  unavailable.value = "";
  if (props.item.contentType !== "image") return;
  try {
    thumbnail.value = await getLibraryImageThumbnail(props.item.id, 320);
  } catch (error) {
    unavailable.value = `图片资源不可用：${String(error)}`;
  }
}

watch(() => props.item.id, loadThumbnail, { immediate: true });
</script>

<template>
  <article
    data-library-card
    class="library-card group relative grid min-w-0 gap-3 overflow-hidden rounded-xl border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg)] p-4 transition duration-150 hover:border-[color:var(--main-line)] hover:bg-[color:var(--main-bg-soft)]"
    :class="{
      'library-card--pinned': item.isPinned,
      'library-card--list': layout === 'list',
    }"
  >
    <span v-if="item.isPinned" class="library-pin-rail" aria-hidden="true" />
    <div data-library-card-header class="flex min-w-0 items-start justify-between gap-3">
      <div class="flex min-w-0 items-start gap-3">
        <div class="grid h-9 w-9 shrink-0 place-items-center rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] text-[color:var(--accent-text)]">
          <component :is="typeIcon" class="h-4 w-4" />
        </div>
        <div class="min-w-0">
          <div class="flex min-w-0 items-center gap-2">
            <GripVertical v-if="item.isPinned" class="h-4 w-4 shrink-0 cursor-grab text-slate-500" />
            <h3 class="truncate text-[14px] font-bold text-white">{{ item.title }}</h3>
          </div>
          <p class="mt-1 text-[11px] font-semibold uppercase tracking-[0.14em] text-[color:var(--muted-text)]">
            {{ item.role === "snippet" ? formatTime(item.createdAt) : `${typeLabel} · ${formatTime(item.createdAt)}` }}
          </p>
        </div>
      </div>
      <span
        v-if="item.isPinned"
        class="shrink-0 rounded-full border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] px-2 py-0.5 text-[11px] font-bold text-[color:var(--accent-text)]"
      >
        置顶
      </span>
    </div>

    <div data-library-card-preview class="grid min-w-0 gap-3">
      <img
        v-if="thumbnail"
        :src="`data:image/png;base64,${thumbnail}`"
        :alt="item.title"
        class="library-image-preview max-h-48 w-full rounded-lg border border-[color:var(--main-line-soft)] bg-black/20 object-contain"
      />
      <div
        v-else-if="item.contentType === 'fileList'"
        class="library-file-preview grid gap-1 overflow-hidden rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 py-2"
      >
        <div v-for="asset in item.assets" :key="asset.assetId" class="flex min-w-0 items-center gap-2 text-[12px] text-slate-300">
          <Files class="h-3.5 w-3.5 shrink-0 text-slate-500" />
          <span class="truncate">{{ asset.fileName }}</span>
        </div>
      </div>
      <p
        v-else
        class="line-clamp-4 whitespace-pre-wrap break-words text-[13px] leading-6 text-slate-200"
      >
        {{ previewText }}
      </p>

      <p v-if="unavailable" class="rounded-lg border border-red-500/35 bg-red-500/10 px-3 py-2 text-[12px] text-red-200">
        {{ unavailable }}
      </p>
      <p v-if="item.note" class="text-[12px] leading-5 text-[color:var(--muted-text)]">{{ item.note }}</p>

      <div v-if="item.tags.length" class="flex flex-wrap gap-1.5">
        <span
          v-for="tag in item.tags"
          :key="tag"
          class="rounded-full border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-2 py-0.5 text-[11px] text-slate-300"
        >
          {{ tag }}
        </span>
      </div>
    </div>

    <div data-library-card-actions class="flex flex-wrap items-center gap-1.5 border-t border-[color:var(--main-line-soft)] pt-3">
      <button
        data-library-copy
        type="button"
        class="library-action"
        :disabled="busy || Boolean(unavailable)"
        @click="emit('copy', item)"
      >
        <Copy class="h-3.5 w-3.5" />复制
      </button>
      <button
        data-library-pin
        type="button"
        class="library-action"
        :disabled="busy"
        @click="emit('pin', item)"
      >
        <PinOff v-if="item.isPinned" class="h-3.5 w-3.5" />
        <Pin v-else class="h-3.5 w-3.5" />
        {{ item.isPinned ? "取消置顶" : "置顶" }}
      </button>
      <button
        data-library-edit
        type="button"
        class="library-action"
        :disabled="busy"
        @click="emit('edit', item)"
      >
        <Pencil class="h-3.5 w-3.5" />信息
      </button>
      <button
        v-if="canConvertSnippet"
        data-library-convert-snippet
        type="button"
        class="library-action"
        :disabled="busy"
        @click="emit('convert-snippet', item)"
      >
        <MessageSquareText class="h-3.5 w-3.5" />转为片段
      </button>
      <button
        v-if="canEditSnippet"
        data-library-edit-snippet
        type="button"
        class="library-action"
        :disabled="busy"
        @click="emit('edit-snippet', item)"
      >
        <MessageSquareText class="h-3.5 w-3.5" />编辑正文
      </button>
      <button
        data-library-remove
        type="button"
        class="library-action library-action--danger ml-auto"
        :disabled="busy"
        @click="emit('remove', item)"
      >
        <Trash2 class="h-3.5 w-3.5" />移出
      </button>
    </div>
  </article>
</template>

<style scoped>
.library-card--pinned {
  padding-left: 1.25rem;
}

.library-pin-rail {
  position: absolute;
  inset: 0.65rem auto 0.65rem 0.35rem;
  width: 3px;
  border-radius: 999px;
  background: linear-gradient(180deg, var(--accent-text), transparent 88%);
  box-shadow: 0 0 14px color-mix(in srgb, var(--accent-text) 42%, transparent);
}

.library-card--list {
  grid-template-columns: minmax(180px, 0.8fr) minmax(0, 1.4fr) auto;
  align-items: center;
  gap: 1rem;
  padding-block: 0.75rem;
}

.library-card--list [data-library-card-preview] {
  min-width: 0;
}

.library-card--list [data-library-card-preview] > p {
  display: -webkit-box;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.library-card--list .library-image-preview {
  max-height: 4rem;
}

.library-card--list .library-file-preview {
  max-height: 4rem;
}

.library-card--list [data-library-card-actions] {
  justify-content: flex-end;
  border-top: 0;
  padding-top: 0;
}

.library-action {
  display: inline-flex;
  height: 1.8rem;
  align-items: center;
  gap: 0.3rem;
  border-radius: 0.4rem;
  padding: 0 0.55rem;
  color: rgb(203 213 225);
  font-size: 0.72rem;
  font-weight: 600;
  transition: 150ms ease;
}

.library-action:hover:not(:disabled) {
  background: var(--main-bg-muted);
  color: white;
}

.library-action:focus-visible {
  outline: 2px solid var(--accent-line);
  outline-offset: 2px;
}

.library-action:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.library-action--danger:hover:not(:disabled) {
  background: rgb(239 68 68 / 0.12);
  color: rgb(254 202 202);
}

@media (max-width: 720px) {
  .library-card--list {
    grid-template-columns: minmax(0, 1fr);
    align-items: stretch;
  }

  .library-card--list [data-library-card-actions] {
    justify-content: flex-start;
    border-top: 1px solid var(--main-line-soft);
    padding-top: 0.75rem;
  }
}
</style>
