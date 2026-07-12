import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

import { createPinia, setActivePinia } from "pinia";

const storePath = "src/stores/translation.ts";
const pagePath = "src/pages/Translate.vue";

test("translation session state survives page re-entry", async () => {
  assert.equal(existsSync(storePath), true);

  const { useTranslationStore } = await import("../src/stores/translation.ts");
  setActivePinia(createPinia());

  const firstPageSession = useTranslationStore();
  firstPageSession.inputText = "需要保留的输入";
  firstPageSession.targetLang = "zh";
  firstPageSession.result = {
    sourceText: "persisted input",
    targetText: "保留的翻译结果",
    engine: "google",
  };

  const reopenedPageSession = useTranslationStore();
  assert.strictEqual(reopenedPageSession, firstPageSession);
  assert.equal(reopenedPageSession.inputText, "需要保留的输入");
  assert.equal(reopenedPageSession.targetLang, "zh");
  assert.equal(reopenedPageSession.result?.targetText, "保留的翻译结果");
});

test("translation page binds form and result to the session store", () => {
  const page = readFileSync(pagePath, "utf8");

  assert.match(page, /import \{ storeToRefs \} from "pinia"/);
  assert.match(page, /import \{ useTranslationStore \} from "@\/stores\/translation"/);
  assert.match(page, /const translationStore = useTranslationStore\(\)/);
  assert.match(
    page,
    /const \{ inputText, targetLang, loading, error, result \} = storeToRefs\(translationStore\)/,
  );
});
