<script setup lang="ts">
import { Check, Copy, Languages, Loader2 } from "lucide-vue-next";
import { storeToRefs } from "pinia";
import { computed, ref } from "vue";

import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import { translateText } from "@/lib/tauri";
import { useToastStore } from "@/stores/toasts";
import { useTranslationStore } from "@/stores/translation";
import type { TranslateTargetLanguage } from "@/types/translation";

const toastStore = useToastStore();
const translationStore = useTranslationStore();
const { inputText, targetLang, loading, error, result } = storeToRefs(translationStore);

const targetLanguages: TranslateTargetLanguage[] = [
  { code: "zh", label: "中文", badge: "ZH" },
  { code: "en", label: "English", badge: "EN" },
  { code: "ja", label: "日本語", badge: "JA" },
  { code: "ko", label: "한국어", badge: "KO" },
  { code: "fr", label: "Français", badge: "FR" },
  { code: "de", label: "Deutsch", badge: "DE" },
  { code: "es", label: "Español", badge: "ES" },
  { code: "ru", label: "Русский", badge: "RU" },
];

const copied = ref(false);

const trimmedInput = computed(() => inputText.value.trim());
const canTranslate = computed(() => trimmedInput.value.length > 0 && !loading.value);
const engineLabel = computed(() => {
  if (!result.value) return "";
  return result.value.engine === "ai" ? "AI" : "Google";
});

async function submitTranslation() {
  if (!canTranslate.value) return;

  loading.value = true;
  error.value = null;
  copied.value = false;

  try {
    result.value = await translateText(trimmedInput.value, targetLang.value);
  } catch (err) {
    error.value = String(err);
    result.value = null;
  } finally {
    loading.value = false;
  }
}

async function copyResult() {
  const text = result.value?.targetText;
  if (!text) return;

  try {
    await navigator.clipboard.writeText(text);
    copied.value = true;
    toastStore.success("译文已复制");
    window.setTimeout(() => {
      copied.value = false;
    }, 1600);
  } catch (err) {
    toastStore.error(`复制失败：${String(err)}`);
  }
}
</script>

<template>
  <div data-translate-page class="grid gap-4 pb-4 text-[13px]">
    <section class="rounded-[14px] border border-[color:var(--main-line)] bg-[color:var(--panel-bg)] p-4">
      <div class="flex items-start justify-between gap-4">
        <div class="min-w-0">
          <p class="text-lg font-bold text-white">翻译</p>
          <p class="mt-1 text-[13px] leading-5 text-[color:var(--muted-text)]">
            输入文本后选择目标语言，默认使用 Google 免费翻译；可在设置中切换 AI 翻译。
          </p>
        </div>
        <div class="grid h-10 w-10 shrink-0 place-items-center rounded-xl border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]">
          <Languages class="h-5 w-5" />
        </div>
      </div>
    </section>

    <Card class="grid gap-3 p-4">
      <label class="grid gap-2">
        <span class="text-[15px] font-bold text-white">待翻译文本</span>
        <textarea
          v-model="inputText"
          data-translate-input
          class="min-h-[150px] resize-y rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 py-2 text-[13px] leading-6 text-white outline-none transition focus:border-[color:var(--accent-line)]"
          placeholder="输入或粘贴需要翻译的文本"
          @keydown.ctrl.enter.prevent="submitTranslation"
          @keydown.meta.enter.prevent="submitTranslation"
        />
      </label>

      <div class="flex flex-wrap items-center justify-between gap-3">
        <span class="text-[13px] text-[color:var(--muted-text)]">{{ inputText.length }} 字符</span>
        <div class="flex flex-wrap items-center justify-end gap-2">
          <label class="flex items-center gap-2 text-[13px] text-[color:var(--muted-text)]">
            目标语言
            <select
              v-model="targetLang"
              data-translate-target-lang
              class="h-8 rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-2 text-[13px] font-bold text-white outline-none"
            >
              <option v-for="language in targetLanguages" :key="language.code" :value="language.code">
                {{ language.badge }} · {{ language.label }}
              </option>
            </select>
          </label>
          <Button
            data-translate-submit
            size="sm"
            :disabled="!canTranslate"
            @click="submitTranslation"
          >
            <Loader2 v-if="loading" class="mr-1 h-4 w-4 animate-spin" />
            <Languages v-else class="mr-1 h-4 w-4" />
            翻译
          </Button>
        </div>
      </div>
    </Card>

    <p
      v-if="error"
      class="rounded-lg border border-red-500/40 bg-red-500/10 px-3 py-2 text-[13px] leading-5 text-red-100"
    >
      {{ error }}
    </p>

    <Card class="grid gap-3 p-4">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <span class="text-[15px] font-bold text-white">翻译结果</span>
          <span
            v-if="engineLabel"
            class="rounded-full border border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-muted)] px-2 py-0.5 text-[12px] font-bold text-slate-300"
          >
            {{ engineLabel }}
          </span>
        </div>
        <Button
          v-if="result?.targetText"
          data-translate-copy
          size="sm"
          variant="secondary"
          @click="copyResult"
        >
          <Check v-if="copied" class="mr-1 h-4 w-4" />
          <Copy v-else class="mr-1 h-4 w-4" />
          {{ copied ? "已复制" : "复制" }}
        </Button>
      </div>

      <div
        data-translate-result
        class="min-h-[120px] whitespace-pre-wrap rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 py-3 text-[13px] leading-6 text-slate-100"
      >
        <span v-if="result?.targetText">{{ result.targetText }}</span>
        <span v-else class="text-[color:var(--muted-text)]">翻译结果会显示在这里</span>
      </div>
    </Card>
  </div>
</template>
