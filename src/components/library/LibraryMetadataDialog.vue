<script setup lang="ts">
import { computed, ref, watch } from "vue";

import Button from "@/components/ui/Button.vue";
import type { LibraryItem, LibraryItemUpdate } from "@/types/library";

const props = defineProps<{
  open: boolean;
  item: LibraryItem | null;
}>();

const emit = defineEmits<{
  submit: [LibraryItemUpdate];
  cancel: [];
}>();

const title = ref("");
const tagsText = ref("");
const note = ref("");

watch(
  () => [props.open, props.item] as const,
  () => {
    title.value = props.item?.title ?? "";
    tagsText.value = props.item?.tags.join(", ") ?? "";
    note.value = props.item?.note ?? "";
  },
  { immediate: true },
);

const tags = computed(() =>
  tagsText.value
    .split(",")
    .map((tag) => tag.trim())
    .filter(Boolean),
);
const error = computed(() => {
  if (!title.value.trim()) return "请输入标题";
  if (title.value.trim().length > 120) return "标题不能超过 120 个字符";
  if (tags.value.length > 20) return "每个收藏最多 20 个标签";
  if (tags.value.some((tag) => tag.length > 32)) return "单个标签不能超过 32 个字符";
  if (note.value.trim().length > 2000) return "备注不能超过 2000 个字符";
  return "";
});

function submit() {
  if (error.value) return;
  emit("submit", {
    title: title.value.trim(),
    content: null,
    tags: tags.value,
    note: note.value.trim(),
  });
}

function cancelFromBackdrop(event: MouseEvent) {
  if (event.target === event.currentTarget) emit("cancel");
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-[90] grid place-items-center bg-slate-950/70 p-4 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      aria-labelledby="metadata-dialog-title"
      @mousedown="cancelFromBackdrop"
    >
      <form
        class="grid w-full max-w-lg gap-4 rounded-2xl border border-[color:var(--main-line)] bg-[color:var(--main-bg)] p-5 shadow-2xl"
        @submit.prevent="submit"
      >
        <div>
          <h2 id="metadata-dialog-title" class="text-lg font-bold text-white">编辑收藏信息</h2>
          <p class="mt-1 text-[12px] text-[color:var(--muted-text)]">正文和文件不会改变。</p>
        </div>

        <label class="grid gap-1.5 text-[13px] font-medium text-slate-200">
          标题
          <input
            v-model="title"
            autofocus
            class="h-10 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 text-white outline-none focus:border-[color:var(--accent-line)]"
          />
        </label>
        <label class="grid gap-1.5 text-[13px] font-medium text-slate-200">
          标签
          <input
            v-model="tagsText"
            class="h-10 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 text-white outline-none focus:border-[color:var(--accent-line)]"
            placeholder="工作, 常用"
          />
        </label>
        <label class="grid gap-1.5 text-[13px] font-medium text-slate-200">
          备注
          <textarea
            v-model="note"
            class="min-h-24 resize-y rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 py-2 text-white outline-none focus:border-[color:var(--accent-line)]"
          />
        </label>
        <p v-if="error" class="text-[12px] text-red-300">{{ error }}</p>

        <div class="flex justify-end gap-2 border-t border-[color:var(--main-line-soft)] pt-4">
          <Button variant="ghost" @click="emit('cancel')">取消</Button>
          <Button variant="primary" type="submit">保存修改</Button>
        </div>
      </form>
    </div>
  </Teleport>
</template>
