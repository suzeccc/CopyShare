import { createPinia } from "pinia";
import { createApp } from "vue";

import { initializeI18n, setUiLanguage } from "./i18n";
import { getConfig, onAppEvent } from "./lib/tauri";
import { useConfigStore } from "./stores/config";
import type { UiLanguage } from "./types/config";
import "./style.css";

async function bootstrap() {
  const utilityWindow = /^#\/(?:media-preview|floating-clipboard)(?:[?/]|$)/.test(window.location.hash);
  const appModule = utilityWindow ? import("./UtilityApp.vue") : import("./App.vue");
  const [{ default: App }, { default: router }] = await Promise.all([
    appModule,
    import("./router"),
  ]);
  const isTauriRuntime = "__TAURI_INTERNALS__" in window;
  const initialConfig = isTauriRuntime
    ? await getConfig().catch(() => null)
    : null;
  initializeI18n(initialConfig?.uiLanguage ?? "system");
  if (isTauriRuntime) {
    await onAppEvent("config-updated", (config) => {
      const nextConfig = config as { uiLanguage?: UiLanguage };
      setUiLanguage(nextConfig.uiLanguage ?? "system");
    }).catch(() => undefined);
  }

  const pinia = createPinia();
  if (initialConfig) {
    useConfigStore(pinia).config = initialConfig;
  }
  const app = createApp(App).use(pinia).use(router);
  await router.isReady();
  app.mount("#app");
}

void bootstrap();
