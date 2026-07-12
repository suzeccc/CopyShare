import assert from "node:assert/strict";
import test from "node:test";

import { createPinia, setActivePinia } from "pinia";

test("OCR store owns the complete page session lifecycle", async () => {
  const { useOcrStore } = await import("../src/stores/ocr.ts");
  setActivePinia(createPinia());
  const store = useOcrStore();

  assert.deepEqual(store.$state, {
    status: "idle",
    previewBase64: "",
    resultText: "",
    imageWidth: 0,
    imageHeight: 0,
    error: null,
  });

  store.previewBase64 = "old-preview";
  store.resultText = "old text";
  store.error = "old error";
  store.beginRecognition();
  assert.equal(store.status, "loading");
  assert.equal(store.previewBase64, "");
  assert.equal(store.resultText, "");
  assert.equal(store.error, null);

  store.applyResponse({
    text: "recognized text",
    previewBase64: "preview",
    imageWidth: 640,
    imageHeight: 480,
    error: null,
  });
  assert.equal(store.status, "success");
  assert.equal(store.resultText, "recognized text");
  assert.equal(store.previewBase64, "preview");
  assert.equal(store.imageWidth, 640);
  assert.equal(store.imageHeight, 480);

  store.resultText = "corrected text";
  assert.equal(useOcrStore().resultText, "corrected text");

  store.applyResponse({
    text: "",
    previewBase64: "blank-preview",
    imageWidth: 320,
    imageHeight: 200,
    error: null,
  });
  assert.equal(store.status, "empty");

  store.applyResponse({
    text: "",
    previewBase64: "failed-preview",
    imageWidth: 800,
    imageHeight: 600,
    error: "Windows OCR 不可用，请安装系统语言包后重试。",
  });
  assert.equal(store.status, "error");
  assert.equal(store.previewBase64, "failed-preview");
  assert.equal(store.error, "Windows OCR 不可用，请安装系统语言包后重试。");

  store.failRecognition("剪贴板中没有图片，请先复制或截图。");
  assert.equal(store.status, "error");
  assert.equal(store.previewBase64, "");
  assert.equal(store.error, "剪贴板中没有图片，请先复制或截图。");

  store.clearSession();
  assert.equal(store.status, "idle");
  assert.equal(store.previewBase64, "");
  assert.equal(store.resultText, "");
  assert.equal(store.error, null);
});
