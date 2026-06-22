<script setup lang="ts">
import { Trash2 } from "lucide-vue-next";

import HistoryItem from "@/components/history/HistoryItem.vue";
import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import { useHistoryStore } from "@/stores/history";

const historyStore = useHistoryStore();
</script>

<template>
  <Card>
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div>
        <p class="text-sm font-semibold text-white">同步历史</p>
        <p class="mt-2 max-w-2xl text-sm leading-6 text-slate-400">
          历史记录只保存前 80 个字符摘要，用于确认同步方向、来源设备和时间。
          可以在设置中关闭历史记录。
        </p>
      </div>
      <Button variant="danger" :disabled="!historyStore.items.length || historyStore.loading" @click="historyStore.clear()">
        <Trash2 class="h-4 w-4" />
        清空历史
      </Button>
    </div>

    <p v-if="historyStore.error" class="mt-4 rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-sm text-red-100">
      {{ historyStore.error }}
    </p>

    <div v-if="historyStore.items.length" class="mt-5 grid gap-3">
      <HistoryItem v-for="item in historyStore.items" :key="item.id" :item="item" />
    </div>
    <div v-else class="mt-5 rounded-lg border border-dashed border-[color:var(--main-line-soft)] px-4 py-12 text-center text-sm text-slate-500">
      暂无同步历史。复制文本并完成一次同步后，这里会显示摘要。
    </div>
  </Card>
</template>
