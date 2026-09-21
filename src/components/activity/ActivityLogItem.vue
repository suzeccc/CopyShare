<script setup lang="ts">
import ArrowDownLeft from "lucide-vue-next/dist/esm/icons/arrow-down-left.js";
import ArrowUpRight from "lucide-vue-next/dist/esm/icons/arrow-up-right.js";
import ChevronDown from "lucide-vue-next/dist/esm/icons/chevron-down.js";
import CircleAlert from "lucide-vue-next/dist/esm/icons/circle-alert.js";
import FileStack from "lucide-vue-next/dist/esm/icons/files.js";
import Monitor from "lucide-vue-next/dist/esm/icons/monitor.js";
import Radio from "lucide-vue-next/dist/esm/icons/radio.js";
import { computed, ref } from "vue";

import Button from "@/components/ui/Button.vue";
import {
  activityContentTypeLabel,
  activityStatusLabel,
  canResumeActivity,
  formatActivitySize,
  formatActivityTimestamp,
} from "@/lib/activityLog";
import type { ActivityLogEntry } from "@/types/activityLog";

const props = defineProps<{ item: ActivityLogEntry; resuming?: boolean }>();
const emit = defineEmits<{ resume: [entry: ActivityLogEntry] }>();
const detailOpen = ref(false);

const icon = computed(() => {
  if (props.item.category === "file") return FileStack;
  if (props.item.category === "device") return Monitor;
  if (props.item.category === "system") return props.item.status === "error" ? CircleAlert : Radio;
  return props.item.direction === "receive" ? ArrowDownLeft : ArrowUpRight;
});
const contentTypeLabel = computed(() => activityContentTypeLabel(props.item));
const sizeLabel = computed(() => formatActivitySize(props.item.sizeBytes));
const resumable = computed(() => canResumeActivity(props.item));
const statusClass = computed(() => ({
  success: "border-emerald-400/25 bg-emerald-400/10 text-emerald-200",
  warning: "border-amber-400/25 bg-amber-400/10 text-amber-200",
  error: "border-red-400/30 bg-red-400/10 text-red-200",
  info: "border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] text-slate-300",
}[props.item.status]));
</script>

<template>
  <article data-activity-log-item class="rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] px-4 py-3">
    <div class="flex items-start gap-3">
      <div class="mt-0.5 grid h-9 w-9 shrink-0 place-items-center rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--stat-bg)] text-[color:var(--accent-text)]">
        <component :is="icon" class="h-4 w-4" />
      </div>
      <div class="min-w-0 flex-1">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div class="min-w-0">
            <p class="text-sm font-semibold text-white">{{ item.title }}</p>
            <div class="mt-1.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-[color:var(--muted-text)]">
              <span v-if="item.deviceName" data-i18n-ignore class="max-w-52 truncate rounded-md bg-[color:var(--field-bg)] px-2 py-0.5 text-slate-300">
                {{ item.deviceName }}
              </span>
              <span v-if="contentTypeLabel">{{ contentTypeLabel }}</span>
              <span v-if="sizeLabel" data-i18n-ignore>· {{ sizeLabel }}</span>
              <span data-i18n-ignore>· {{ formatActivityTimestamp(item.createdAt) }}</span>
            </div>
          </div>
          <div class="flex shrink-0 flex-col items-end gap-1">
            <span class="rounded-md border px-2 py-1 text-[11px] font-semibold" :class="statusClass">
              {{ activityStatusLabel(item.status) }}
            </span>
            <Button
              v-if="resumable"
              size="sm"
              variant="secondary"
              :disabled="resuming"
              @click="emit('resume', item)"
            >
              {{ resuming ? "正在继续" : "继续传输" }}
            </Button>
            <button
              v-if="item.detail"
              type="button"
              class="grid h-7 w-7 place-items-center rounded-md text-slate-400 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white"
              :aria-expanded="detailOpen"
              title="查看详情"
              @click="detailOpen = !detailOpen"
            >
              <ChevronDown class="h-4 w-4 transition" :class="detailOpen ? 'rotate-180' : ''" />
            </button>
          </div>
        </div>
        <p v-if="detailOpen && item.detail" class="mt-3 break-words rounded-md bg-black/20 px-3 py-2 text-xs leading-5 text-red-100">
          {{ item.detail }}
        </p>
      </div>
    </div>
  </article>
</template>
