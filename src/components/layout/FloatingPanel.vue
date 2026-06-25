<script setup lang="ts">
import { Activity, Clipboard, Gauge, LayoutDashboard, Minus, MoreHorizontal, Wifi, X } from "lucide-vue-next";
import { computed, ref } from "vue";

import CopyTextButton from "@/components/ui/CopyTextButton.vue";
import type { ClipboardPreviewItem } from "@/lib/historyPreview";
import { startWindowDrag } from "@/lib/tauri";
import { startWindowDragFromMouseEvent } from "@/lib/windowDrag";

const props = defineProps<{
  statusLabel: string;
  running: boolean;
  connectedCount: number;
  latencyLabel: string;
  clipboardItems: ClipboardPreviewItem[];
  clipboardHistoryItems: ClipboardPreviewItem[];
}>();

const emit = defineEmits<{
  (event: "restore"): void;
  (event: "hide"): void;
  (event: "close"): void;
}>();

const showClipboardHistoryModal = ref(false);

const statusClass = computed(() =>
  props.running
    ? "bg-emerald-400 shadow-[0_0_14px_rgba(16,185,129,0.65)]"
    : "bg-slate-400",
);

function handleWindowDrag(event: MouseEvent) {
  startWindowDragFromMouseEvent(event, startWindowDrag);
}
</script>

<template>
  <section class="floating-window-surface relative flex h-full w-full flex-col overflow-hidden rounded-lg p-3 text-slate-100">
    <header
      class="-mx-3 -mt-3 flex items-center justify-between gap-2 px-3 pb-1.5 pt-3"
      data-window-drag-region
      @mousedown.capture="handleWindowDrag"
    >
      <div class="flex min-w-0 items-center gap-2" data-window-drag-region>
        <span class="h-2.5 w-2.5 shrink-0 rounded-full" :class="statusClass" data-window-drag-region />
        <div class="min-w-0">
          <p class="truncate text-sm font-semibold leading-4" data-window-drag-region>Copy-Sharer</p>
          <p class="truncate text-[11px] font-medium text-[color:var(--floating-muted-text)]" data-window-drag-region>{{ statusLabel }}</p>
        </div>
      </div>

      <div class="flex shrink-0 items-center gap-1">
        <button
          class="grid h-7 w-7 place-items-center rounded-md border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-[color:var(--floating-control-bg-hover)]"
          type="button"
          aria-label="隐藏窗口"
          title="隐藏窗口"
          data-window-control
          @click="emit('hide')"
        >
          <Minus class="h-3.5 w-3.5" />
        </button>
        <button
          class="inline-flex h-7 items-center gap-1 rounded-md border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] px-2 text-[11px] font-semibold text-[color:var(--floating-control-text)] transition hover:bg-[color:var(--floating-control-bg-hover)]"
          type="button"
          title="返回主面板"
          data-window-control
          @click="emit('restore')"
        >
          <LayoutDashboard class="h-3.5 w-3.5" />
          主面板
        </button>
        <button
          class="grid h-7 w-7 place-items-center rounded-md border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-red-500/72 hover:text-white"
          type="button"
          aria-label="关闭"
          title="关闭"
          data-window-control
          @click="emit('close')"
        >
          <X class="h-3.5 w-3.5" />
        </button>
      </div>
    </header>

    <div class="mt-3 grid grid-cols-3 gap-2">
      <div class="floating-stat">
        <Activity class="h-3.5 w-3.5 text-[color:var(--accent-text)]" />
        <span>启动</span>
        <strong>{{ running ? "运行" : "停止" }}</strong>
      </div>
      <div class="floating-stat">
        <Wifi class="h-3.5 w-3.5 text-[color:var(--accent-text)]" />
        <span>连接</span>
        <strong>{{ connectedCount }} 台</strong>
      </div>
      <div class="floating-stat">
        <Gauge class="h-3.5 w-3.5 text-[color:var(--accent-text)]" />
        <span>延迟</span>
        <strong>{{ latencyLabel }}</strong>
      </div>
    </div>

    <div class="mt-3 flex min-h-0 flex-1 flex-col">
      <div class="mb-1.5 flex items-center justify-between gap-2 text-[11px] font-semibold text-[color:var(--floating-muted-text)]">
        <div class="flex min-w-0 items-center gap-1.5">
          <Clipboard class="h-3.5 w-3.5" />
          <span>剪贴板内容</span>
        </div>
        <button
          data-floating-more-clipboard-button
          class="inline-flex h-6 shrink-0 items-center gap-1 rounded-md border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] px-2 text-[11px] font-semibold text-[color:var(--floating-control-text)] transition hover:bg-[color:var(--floating-control-bg-hover)] disabled:opacity-45"
          type="button"
          title="查看更多剪贴板内容"
          :disabled="!clipboardHistoryItems.length"
          @click="showClipboardHistoryModal = true"
        >
          <MoreHorizontal class="h-3.5 w-3.5" />
          更多
        </button>
      </div>
      <div v-if="clipboardItems.length" class="min-h-0 flex-1 overflow-y-auto pr-1">
        <div
          v-for="item in clipboardItems"
          :key="item.id"
          class="flex min-h-6 items-center gap-2 border-b border-[color:var(--main-line-soft)] py-0.5 last:border-b-0"
        >
          <p data-floating-clipboard-text class="line-clamp-1 min-w-0 flex-1 break-words text-xs font-semibold leading-4 text-[color:var(--floating-strong-text)]">
            {{ item.text }}
          </p>
          <CopyTextButton :text="item.text" icon-only label="复制内容" copied-label="已复制" />
        </div>
      </div>
      <p v-else class="break-words text-xs font-semibold leading-5 text-[color:var(--floating-strong-text)]">
        暂无剪贴板内容
      </p>
    </div>

    <Transition name="trust-prompt">
      <div
        v-if="showClipboardHistoryModal"
        data-floating-clipboard-modal
        class="absolute inset-2 z-30 flex flex-col overflow-hidden rounded-lg border border-[color:var(--floating-control-line)] bg-[color:var(--floating-surface-bg)] p-3 shadow-[0_18px_46px_rgba(0,0,0,0.45)] backdrop-blur-xl"
        @click.self="showClipboardHistoryModal = false"
      >
        <div class="mb-3 flex items-start justify-between gap-3">
          <div class="min-w-0">
            <p class="text-sm font-semibold text-[color:var(--floating-strong-text)]">全部剪贴板内容</p>
            <p class="mt-0.5 text-[11px] font-medium text-[color:var(--floating-muted-text)]">共 {{ clipboardHistoryItems.length }} 条记录</p>
          </div>
          <button
            class="grid h-7 w-7 shrink-0 place-items-center rounded-md border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] text-[color:var(--floating-control-text)] transition hover:bg-[color:var(--floating-control-bg-hover)]"
            type="button"
            aria-label="关闭"
            title="关闭"
            @click="showClipboardHistoryModal = false"
          >
            <X class="h-3.5 w-3.5" />
          </button>
        </div>

        <div v-if="clipboardHistoryItems.length" class="min-h-0 flex-1 overflow-y-auto pr-1">
          <div
            v-for="item in clipboardHistoryItems"
            :key="item.id"
            data-floating-clipboard-history-row
            class="grid grid-cols-[minmax(0,1fr)_auto] gap-2 border-b border-[color:var(--floating-stat-line)] py-2 last:border-b-0"
          >
            <p data-floating-clipboard-history-text class="min-w-0 whitespace-pre-wrap break-all text-xs font-semibold leading-5 text-[color:var(--floating-strong-text)]">
              {{ item.text }}
            </p>
            <div class="flex shrink-0 flex-col items-end gap-1.5">
              <CopyTextButton :text="item.text" icon-only label="复制内容" copied-label="已复制" />
              <span
                v-if="item.sourceDevice"
                class="max-w-20 truncate rounded-full border border-[color:var(--floating-control-line)] bg-[color:var(--floating-control-bg)] px-2 py-0.5 text-[10px] font-semibold text-[color:var(--floating-muted-text)]"
                :title="item.sourceDevice"
              >
                {{ item.sourceDevice }}
              </span>
            </div>
          </div>
        </div>
        <p v-else class="rounded-md border border-[color:var(--floating-stat-line)] bg-[color:var(--floating-stat-bg)] px-3 py-6 text-center text-xs font-semibold text-[color:var(--floating-muted-text)]">
          暂无剪贴板内容
        </p>
      </div>
    </Transition>
  </section>
</template>