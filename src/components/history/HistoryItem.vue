<script setup lang="ts">
import { ArrowDownLeft, ArrowUpRight } from "lucide-vue-next";
import { computed } from "vue";

import { formatTime } from "@/lib/format";
import type { HistoryItem } from "@/types/history";

import CopyTextButton from "../ui/CopyTextButton.vue";

const props = defineProps<{
  item: HistoryItem;
}>();

const copyText = computed(() => props.item.content?.trim() || props.item.summary);
</script>

<template>
  <article class="flex gap-3 rounded-lg border border-[color:var(--main-line-soft)] bg-[rgba(19,34,63,0.58)] p-4">
    <div class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-[color:var(--main-line-soft)] bg-[rgba(28,49,84,0.82)]">
      <ArrowUpRight v-if="item.direction === 'local'" class="h-4 w-4 text-blue-300" />
      <ArrowDownLeft v-else class="h-4 w-4 text-emerald-300" />
    </div>
    <div class="min-w-0 flex-1">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div class="flex flex-wrap items-center gap-2">
          <p class="text-sm font-medium text-white">
            {{ item.direction === "local" ? "本机复制" : "远端写入" }}
          </p>
          <span class="rounded-md bg-[rgba(28,49,84,0.82)] px-2 py-0.5 text-xs text-slate-400">
            {{ item.sourceDevice }}
          </span>
        </div>
        <CopyTextButton :text="copyText" label="复制内容" />
      </div>
      <p class="mt-2 break-words text-sm text-slate-300">{{ item.summary }}</p>
      <p class="mt-2 text-xs text-slate-500">{{ formatTime(item.createdAt) }}</p>
    </div>
  </article>
</template>
