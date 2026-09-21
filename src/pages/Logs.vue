<script setup lang="ts">
import Trash2 from "lucide-vue-next/dist/esm/icons/trash-2.js";
import { computed, ref } from "vue";

import ActivityLogItem from "@/components/activity/ActivityLogItem.vue";
import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import { filterActivities } from "@/lib/activityLog";
import { resumeFileTransfer } from "@/lib/tauri";
import { useActivityLogStore } from "@/stores/activityLog";
import { useToastStore } from "@/stores/toasts";
import type { ActivityLogEntry, ActivityLogFilter } from "@/types/activityLog";

const activityLogStore = useActivityLogStore();
const toastStore = useToastStore();
const activeFilter = ref<ActivityLogFilter>("all");
const resumingTransferId = ref<string | null>(null);
const filters: Array<{ value: ActivityLogFilter; label: string }> = [
  { value: "all", label: "全部" },
  { value: "issues", label: "仅异常" },
  { value: "device", label: "设备事件" },
  { value: "file", label: "文件传输" },
];
const filteredItems = computed(() => filterActivities(activityLogStore.items, activeFilter.value));

async function resumeTransfer(entry: ActivityLogEntry) {
  if (!entry.transferId || resumingTransferId.value) return;
  resumingTransferId.value = entry.transferId;
  try {
    const task = await resumeFileTransfer(entry.transferId);
    activityLogStore.recordFileTransfer(task);
    toastStore.success(task.status === "waitingForPeer" ? "已继续等待对方设备上线" : "正在继续传输");
  } catch (error) {
    toastStore.error(`继续传输失败：${String(error)}`);
  } finally {
    resumingTransferId.value = null;
  }
}
</script>

<template>
  <div class="grid gap-5">
    <Card>
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <p class="text-sm font-semibold text-white">日志</p>
          <p class="mt-2 text-sm leading-6 text-[color:var(--muted-text)]">
            记录同步方向、设备状态、文件传输和异常，不保存或展示剪贴板正文
          </p>
        </div>
        <Button
          variant="danger"
          :disabled="!activityLogStore.items.length"
          @click="activityLogStore.clear()"
        >
          <Trash2 class="h-4 w-4" />
          清空日志
        </Button>
      </div>

      <div data-activity-log-filters class="mt-5 flex flex-wrap gap-2 border-b border-[color:var(--main-line-soft)] pb-4">
        <button
          v-for="filter in filters"
          :key="filter.value"
          type="button"
          class="h-8 rounded-md border px-3 text-[13px] font-semibold transition"
          :class="activeFilter === filter.value
            ? 'border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]'
            : 'border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] text-slate-400 hover:border-[color:var(--main-line)] hover:text-white'"
          :aria-pressed="activeFilter === filter.value"
          @click="activeFilter = filter.value"
        >
          {{ filter.label }}
        </button>
      </div>

      <div v-if="filteredItems.length" class="mt-4 grid gap-2">
        <ActivityLogItem
          v-for="item in filteredItems"
          :key="item.id"
          :item="item"
          :resuming="resumingTransferId === item.transferId"
          @resume="resumeTransfer"
        />
      </div>
      <div v-else class="mt-4 rounded-lg border border-dashed border-[color:var(--main-line-soft)] px-4 py-12 text-center text-sm text-[color:var(--subtle-text)]">
        {{ activityLogStore.items.length ? "当前筛选下没有记录" : "暂无同步日志，设备连接或发生同步后会显示在这里" }}
      </div>
    </Card>
  </div>
</template>
