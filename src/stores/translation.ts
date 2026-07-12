import { defineStore } from "pinia";

import type { TranslateResponse } from "@/types/translation";

export const useTranslationStore = defineStore("translation", {
  state: () => ({
    inputText: "",
    targetLang: "en",
    loading: false,
    error: null as string | null,
    result: null as TranslateResponse | null,
  }),
});
