import { defineStore } from "pinia";

import type { OcrResponse, OcrStatus } from "@/types/ocr";

type OcrState = {
  status: OcrStatus;
  previewBase64: string;
  resultText: string;
  imageWidth: number;
  imageHeight: number;
  error: string | null;
};

const initialState = (): OcrState => ({
  status: "idle",
  previewBase64: "",
  resultText: "",
  imageWidth: 0,
  imageHeight: 0,
  error: null,
});

export const useOcrStore = defineStore("ocr", {
  state: initialState,
  actions: {
    beginRecognition() {
      this.$patch({
        status: "loading",
        previewBase64: "",
        resultText: "",
        imageWidth: 0,
        imageHeight: 0,
        error: null,
      });
    },
    applyResponse(response: OcrResponse) {
      this.$patch({
        status: response.error
          ? "error"
          : response.text.trim()
            ? "success"
            : "empty",
        previewBase64: response.previewBase64,
        resultText: response.text,
        imageWidth: response.imageWidth,
        imageHeight: response.imageHeight,
        error: response.error,
      });
    },
    failRecognition(error: string) {
      this.$patch({
        status: "error",
        previewBase64: "",
        resultText: "",
        imageWidth: 0,
        imageHeight: 0,
        error,
      });
    },
    clearSession() {
      this.$reset();
    },
  },
});
