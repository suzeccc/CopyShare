<script setup lang="ts">
import { computed, ref, watch } from "vue";

import Button from "@/components/ui/Button.vue";
import type { CreateSnippetInput, LibraryItem } from "@/types/library";

const props = defineProps<{
  open: boolean;
  item: LibraryItem | null;
}>();

const emit = defineEmits<{
  submit: [CreateSnippetInput];
  cancel: [];
}>();

const title = ref("");
const content = ref("");
const tagsText = ref("");
const note = ref("");
const submitted = ref(false);

watch(
  () => [props.open, props.item] as const,
  () => {
    submitted.value = false;
    title.value = props.item?.title ?? "";
    content.value = props.item?.content ?? "";
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
const titleError = computed(() => {
  if (!title.value.trim()) return "请输入标题";
  if (title.value.trim().length > 120) return "标题不能超过 120 个字符";
  return "";
});
const contentError = computed(() =>
  content.value.trim() ? "" : "请输入正文",
);
const metadataError = computed(() => {
  if (tags.value.length > 20) return "每个片段最多 20 个标签";
  if (tags.value.some((tag) => tag.length > 32)) return "单个标签不能超过 32 个字符";
  if (note.value.trim().length > 2000) return "备注不能超过 2000 个字符";
  return "";
});

function submit() {
  submitted.value = true;
  if (titleError.value || contentError.value || metadataError.value) return;
  emit("submit", {
    title: title.value.trim(),
    content: content.value.trim(),
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
      aria-labelledby="snippet-dialog-title"
      @mousedown="cancelFromBackdrop"
    >
      <form
        class="grid max-h-[88vh] w-full max-w-xl gap-4 overflow-y-auto rounded-2xl border border-[color:var(--main-line)] bg-[color:var(--main-bg)] p-5 shadow-2xl"
        @submit.prevent="submit"
      >
        <div>
          <h2 id="snippet-dialog-title" class="text-lg font-bold text-white">
            {{ item ? "编辑常用片段" : "新建文本片段" }}
          </h2>
          <p class="mt-1 text-[12px] text-[color:var(--muted-text)]">
            保存经常发送的回复、地址或说明，之后可一键复制。
          </p>
        </div>

        <label class="grid gap-1.5 text-[13px] font-medium text-slate-200">
          标题
          <input
            v-model="title"
            autofocus
            class="h-10 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 text-white outline-none focus:border-[color:var(--accent-line)]"
            placeholder="例如：售后确认回复"
          />
          <span v-if="submitted && titleError" class="text-[12px] text-red-300">{{ titleError }}</span>
        </label>

        <label class="grid gap-1.5 text-[13px] font-medium text-slate-200">
          正文
          <textarea
            v-model="content"
            class="min-h-36 resize-y rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 py-2 leading-6 text-white outline-none focus:border-[color:var(--accent-line)]"
            placeholder="输入需要重复使用的完整文本"
          />
          <span v-if="submitted && contentError" class="text-[12px] text-red-300">{{ contentError }}</span>
        </label>

        <label class="grid gap-1.5 text-[13px] font-medium text-slate-200">
          标签
          <input
            v-model="tagsText"
            class="h-10 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 text-white outline-none focus:border-[color:var(--accent-line)]"
            placeholder="回复, 客户, 售后"
          />
        </label>

        <label class="grid gap-1.5 text-[13px] font-medium text-slate-200">
          备注
          <textarea
            v-model="note"
            class="min-h-20 resize-y rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 py-2 text-white outline-none focus:border-[color:var(--accent-line)]"
            placeholder="仅自己可见的使用说明"
          />
          <span v-if="submitted && metadataError" class="text-[12px] text-red-300">{{ metadataError }}</span>
        </label>

        <div class="flex justify-end gap-2 border-t border-[color:var(--main-line-soft)] pt-4">
          <Button variant="ghost" @click="emit('cancel')">取消</Button>
          <Button variant="primary" type="submit">保存片段</Button>
        </div>
      </form>
    </div>
  </Teleport>
</template>
