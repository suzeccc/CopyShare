<script setup lang="ts">
import Check from "lucide-vue-next/dist/esm/icons/check.js";
import Copy from "lucide-vue-next/dist/esm/icons/copy.js";
import ImageIcon from "lucide-vue-next/dist/esm/icons/image.js";
import Loader2 from "lucide-vue-next/dist/esm/icons/loader-circle.js";
import ScanText from "lucide-vue-next/dist/esm/icons/scan-text.js";
import Trash2 from "lucide-vue-next/dist/esm/icons/trash-2.js";
import { storeToRefs } from "pinia";
import { computed, ref } from "vue";

import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import { recognizeClipboardImage } from "@/lib/tauri";
import { useOcrStore } from "@/stores/ocr";
import { useToastStore } from "@/stores/toasts";

const ocrStore = useOcrStore();
const toastStore = useToastStore();
const { status, previewBase64, resultText, imageWidth, imageHeight, error } = storeToRefs(ocrStore);
const copied = ref(false);

const loading = computed(() => status.value === "loading");
const previewSrc = computed(() =>
  previewBase64.value ? `data:image/png;base64,${previewBase64.value}` : "",
);
const canCopy = computed(() => resultText.value.trim().length > 0);
const canClear = computed(() =>
  Boolean(previewBase64.value || resultText.value || error.value),
);

async function handlePaste() {
  if (loading.value) return;

  copied.value = false;
  ocrStore.beginRecognition();
  try {
    const response = await recognizeClipboardImage();
    ocrStore.applyResponse(response);
    if (response.error) {
      toastStore.error(response.error);
    }
  } catch (err) {
    const message = String(err);
    ocrStore.failRecognition(message);
    toastStore.error(message);
  }
}

async function copyResult() {
  const text = resultText.value.trim();
  if (!text) return;

  try {
    await navigator.clipboard.writeText(text);
    copied.value = true;
    toastStore.success("识别文字已复制");
    window.setTimeout(() => {
      copied.value = false;
    }, 1600);
  } catch (err) {
    toastStore.error(`复制失败：${String(err)}`);
  }
}

function clearSession() {
  copied.value = false;
  ocrStore.clearSession();
}
</script>

<template>
  <div data-ocr-page class="grid gap-4 pb-4 text-[13px]">
    <section class="rounded-[14px] border border-[color:var(--main-line)] bg-[color:var(--panel-bg)] p-4">
      <div class="flex items-start justify-between gap-4">
        <div class="min-w-0">
          <p class="text-lg font-bold text-white">图片转文字</p>
          <p class="mt-1 text-[13px] leading-5 text-[color:var(--muted-text)]">
            粘贴截图或图片即可在本机识别文字，图片不会上传云端。
          </p>
        </div>
        <div class="grid h-10 w-10 shrink-0 place-items-center rounded-xl border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]">
          <ScanText class="h-5 w-5" />
        </div>
      </div>
    </section>

    <Card class="grid gap-3 p-4">
      <div
        data-ocr-paste-zone
        class="grid min-h-[240px] place-items-center overflow-hidden rounded-xl border border-dashed border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] p-4 outline-none transition focus:border-[color:var(--accent-line)] focus:ring-2 focus:ring-[color:var(--accent-soft)]"
        :class="loading ? 'cursor-wait' : 'cursor-text'"
        role="button"
        tabindex="0"
        aria-label="粘贴图片进行文字识别"
        @paste.prevent="handlePaste"
      >
        <div v-if="loading" data-ocr-loading class="grid justify-items-center gap-3 text-center">
          <Loader2 class="h-7 w-7 animate-spin text-[color:var(--accent-text)]" />
          <p class="font-semibold text-white">正在识别图片文字...</p>
        </div>
        <img
          v-else-if="previewSrc"
          data-ocr-preview
          :src="previewSrc"
          :alt="`待识别图片，${imageWidth} × ${imageHeight}`"
          class="max-h-[360px] max-w-full rounded-lg object-contain"
        />
        <div v-else class="grid justify-items-center gap-3 text-center">
          <div class="grid h-12 w-12 place-items-center rounded-xl bg-[color:var(--main-bg-muted)] text-slate-300">
            <ImageIcon class="h-6 w-6" />
          </div>
          <div>
            <p class="font-semibold text-white">点击此处后按 Ctrl+V 粘贴图片</p>
            <p class="mt-1 text-[12px] text-[color:var(--muted-text)]">支持截图、位图和复制的图片文件</p>
          </div>
        </div>
      </div>
      <p v-if="previewSrc && !loading" class="text-center text-[12px] text-[color:var(--muted-text)]">
        {{ imageWidth }} × {{ imageHeight }} · 可再次粘贴替换当前图片
      </p>
    </Card>

    <p
      v-if="error"
      class="rounded-lg border border-red-500/40 bg-red-500/10 px-3 py-2 text-[13px] leading-5 text-red-100"
    >
      {{ error }}
    </p>

    <Card class="grid gap-3 p-4">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p class="text-[15px] font-bold text-white">识别结果</p>
          <p class="mt-1 text-[12px] text-[color:var(--muted-text)]">{{ resultText.length }} 字符</p>
        </div>
        <div class="flex items-center gap-2">
          <Button
            data-ocr-clear
            size="sm"
            variant="secondary"
            :disabled="!canClear || loading"
            @click="clearSession"
          >
            <Trash2 class="mr-1 h-4 w-4" />
            清空
          </Button>
          <Button
            data-ocr-copy
            size="sm"
            :disabled="!canCopy || loading"
            @click="copyResult"
          >
            <Check v-if="copied" class="mr-1 h-4 w-4" />
            <Copy v-else class="mr-1 h-4 w-4" />
            {{ copied ? "已复制" : "复制文字" }}
          </Button>
        </div>
      </div>
      <textarea
        v-model="resultText"
        data-ocr-result
        class="min-h-[180px] resize-y rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 py-3 text-[13px] leading-6 text-white outline-none transition placeholder:text-[color:var(--muted-text)] focus:border-[color:var(--accent-line)]"
        :disabled="loading"
        :placeholder="status === 'empty' ? '图片中未识别到文字' : '识别结果会显示在这里，可直接编辑'"
      />
    </Card>
  </div>
</template>
