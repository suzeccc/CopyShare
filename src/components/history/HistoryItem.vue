<script setup lang="ts">
import { ArrowDownLeft, ArrowUpRight } from "lucide-vue-next";

import { formatTime } from "@/lib/format";
import type { HistoryItem } from "@/types/history";

defineProps<{
  item: HistoryItem;
}>();
</script>

<template>
  <article class="flex gap-3 rounded-lg border border-slate-700/70 bg-slate-950/48 p-4">
    <div class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-slate-700 bg-slate-900">
      <ArrowUpRight v-if="item.direction === 'local'" class="h-4 w-4 text-blue-300" />
      <ArrowDownLeft v-else class="h-4 w-4 text-emerald-300" />
    </div>
    <div class="min-w-0 flex-1">
      <div class="flex flex-wrap items-center gap-2">
        <p class="text-sm font-medium text-white">
          {{ item.direction === "local" ? "本机复制" : "远端写入" }}
        </p>
        <span class="rounded-md bg-slate-800 px-2 py-0.5 text-xs text-slate-400">
          {{ item.sourceDevice }}
        </span>
      </div>
      <p class="mt-2 break-words text-sm text-slate-300">{{ item.summary }}</p>
      <p class="mt-2 text-xs text-slate-500">{{ formatTime(item.createdAt) }}</p>
    </div>
  </article>
</template>
