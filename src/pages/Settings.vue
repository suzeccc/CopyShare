<script setup lang="ts">
import CheckCircle2 from "lucide-vue-next/dist/esm/icons/circle-check.js";
import ChevronDown from "lucide-vue-next/dist/esm/icons/chevron-down.js";
import ChevronRight from "lucide-vue-next/dist/esm/icons/chevron-right.js";
import Globe2 from "lucide-vue-next/dist/esm/icons/earth.js";
import Keyboard from "lucide-vue-next/dist/esm/icons/keyboard.js";
import Sparkles from "lucide-vue-next/dist/esm/icons/sparkles.js";
import type { Component } from "vue";
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";

import ShortcutSettingsDialog from "@/components/settings/ShortcutSettingsDialog.vue";
import Button from "@/components/ui/Button.vue";
import Switch from "@/components/ui/Switch.vue";
import { getEffectiveLocale, setUiLanguage } from "@/i18n";
import { clampPort } from "@/lib/format";
import type { AppWindowMode } from "@/lib/windowMode";
import {
  clearCache,
  getCacheSize,
  getTransferSaveDir,
  openTransferFolder,
  resetTransferSaveDir,
  selectTransferSaveDir,
} from "@/lib/tauri";
import { useConfigStore } from "@/stores/config";
import { useStatusStore } from "@/stores/status";
import { useToastStore } from "@/stores/toasts";
import type {
  AppConfig,
  AppTheme,
  CloseAction,
  SyncDirection,
  TranslationEngine,
  UiLanguage,
} from "@/types/config";

const configStore = useConfigStore();
const statusStore = useStatusStore();
const toastStore = useToastStore();

const draft = reactive<AppConfig>({
  ...configStore.config,
  uiLanguage: (configStore.config.uiLanguage === "system" ? getEffectiveLocale() : configStore.config.uiLanguage) as UiLanguage,
});
const themeOptions: Array<{ value: AppTheme; label: string; hint: string }> = [
  { value: "win11Dark", label: "Win11 深色", hint: "深灰卡片与系统设置风格" },
  { value: "macosDark", label: "午夜玻璃", hint: "深色半透明面板与 Apple 风格蓝色强调" },
  { value: "macosLight", label: "石墨白雾", hint: "浅色毛玻璃与 Apple 风格蓝色强调" },
  { value: "copyBlue", label: "清雅茶绿", hint: "茶话间深黑绿风格" },
];
const closeActionOptions: Array<{ value: CloseAction; label: string; hint: string }> = [
  { value: "ask", label: "每次询问", hint: "点击关闭时弹出选择提示" },
  { value: "minimize", label: "最小化到托盘", hint: "关闭窗口后继续在后台同步" },
  { value: "exit", label: "直接退出", hint: "关闭窗口时结束应用进程" },
];
const translationEngineOptions: Array<{
  value: TranslationEngine;
  label: string;
  hint: string;
  icon: Component;
}> = [
  { value: "google", label: "Google 翻译", hint: "免费 · 无需配置", icon: Globe2 },
  { value: "ai", label: "AI 翻译", hint: "使用自有 API", icon: Sparkles },
];
const startupWindowOptions: Array<{ value: Exclude<AppWindowMode, "ball">; label: string }> = [
  { value: "floating", label: "浮窗界面" },
  { value: "main", label: "主界面" },
];
const syncDirectionOptions: Array<{ value: SyncDirection; label: string }> = [
  { value: "bidirectional", label: "发送和接收" },
  { value: "sendOnly", label: "只发送" },
  { value: "receiveOnly", label: "只接收" },
];
const languageOptions: Array<{ value: UiLanguage; label: string }> = [
  { value: "zh-CN", label: "简体中文" },
  { value: "zh-TW", label: "繁體中文" },
  { value: "en-US", label: "English" },
  { value: "ja-JP", label: "日本語" },
];
const basicSettingsSaving = ref(false);
const syncContentSaving = ref(false);
const notificationSettingsSaving = ref(false);
const downloadLocationSaving = ref(false);
const cacheSizeBytes = ref<number | null>(null);
const cacheSizeLoading = ref(false);
const cacheClearing = ref(false);
const shortcutDialogOpen = ref(false);
const defaultTransferSaveDir = ref("");
const displayedTransferSaveDir = computed(() =>
  draft.fileSaveDir || defaultTransferSaveDir.value || "默认下载目录",
);
const configMutationSaving = computed(() =>
  configStore.saving
  || basicSettingsSaving.value
  || syncContentSaving.value
  || notificationSettingsSaving.value
  || downloadLocationSaving.value,
);

type BasicSettingKey =
  | "uiLanguage"
  | "deviceName"
  | "port"
  | "theme"
  | "closeAction"
  | "startupWindowMode"
  | "autoStart"
  | "autoSync"
  | "autoOpenFolderAfterSave"
  | "translationEngine"
  | "translationApiUrl"
  | "translationApiKey"
  | "translationModel";
type NotificationSettingKey =
  | "desktopNotifications"
  | "notifyClipboard"
  | "notifyTrustRequired"
  | "notifyDeviceStatus"
  | "notifySyncError"
  | "notificationClipboardPreview";

const shortcutEnabledCount = computed(() => [
  draft.quickPanelShortcutEnabled,
  draft.ocrShortcutEnabled,
  draft.translateShortcutEnabled,
  draft.snippetsShortcutEnabled,
  draft.toggleSyncShortcutEnabled,
].filter(Boolean).length);

function applyThemePreview(theme: AppTheme) {
  document.documentElement.dataset.appTheme = theme;
  document.body.dataset.appTheme = theme;
}

function restoreDraftFromConfig() {
  Object.assign(draft, {
    ...configStore.config,
    uiLanguage: configStore.config.uiLanguage === "system" ? getEffectiveLocale() : configStore.config.uiLanguage,
  });
  applyThemePreview(configStore.config.theme);
}

function formatCacheSize(bytes: number) {
  if (bytes < 1024) {
    return `${bytes} B`;
  }

  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  return `${value >= 10 ? value.toFixed(1) : value.toFixed(2)} ${units[unitIndex]}`;
}

const cacheSizeLabel = computed(() => {
  if (cacheSizeBytes.value === null) {
    return cacheSizeLoading.value ? "缓存大小计算中" : "未计算";
  }
  return formatCacheSize(cacheSizeBytes.value);
});

watch(
  () => configStore.config,
  (next) => {
    if (syncContentSaving.value) {
      draft.syncText = next.syncText;
      draft.syncImage = next.syncImage;
      draft.syncFiles = next.syncFiles;
      draft.syncDirection = next.syncDirection;
      draft.maxSendFileSizeMib = next.maxSendFileSizeMib;
      draft.maxReceiveFileSizeMib = next.maxReceiveFileSizeMib;
      draft.deduplicateSyncContent = next.deduplicateSyncContent;
      draft.trustedDevices = next.trustedDevices;
      return;
    }

    if (notificationSettingsSaving.value) {
      draft.desktopNotifications = next.desktopNotifications;
      draft.notifyClipboard = next.notifyClipboard;
      draft.notifyTrustRequired = next.notifyTrustRequired;
      draft.notifyDeviceStatus = next.notifyDeviceStatus;
      draft.notifySyncError = next.notifySyncError;
      draft.notificationClipboardPreview = next.notificationClipboardPreview;
      return;
    }

    Object.assign(draft, next);
  },
  { deep: true },
);

watch(
  () => draft.theme,
  (theme) => {
    applyThemePreview(theme);
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  applyThemePreview(configStore.config.theme);
});

onMounted(() => {
  void loadCacheSize();
  void loadDefaultTransferSaveDir();
});

async function loadDefaultTransferSaveDir() {
  try {
    defaultTransferSaveDir.value = await getTransferSaveDir();
  } catch {
    // Keep the localized fallback when the native path cannot be resolved.
  }
}

async function saveBasicSettings(
  patch: Partial<Pick<AppConfig, BasicSettingKey>>,
  options: { keepSaving?: boolean } = {},
) {
  if (configStore.saving || (basicSettingsSaving.value && !options.keepSaving)) {
    restoreDraftFromConfig();
    return;
  }

  if (!options.keepSaving) {
    basicSettingsSaving.value = true;
  }

  try {
    await configStore.save({
      ...configStore.config,
      ...patch,
      deviceName: (patch.deviceName ?? configStore.config.deviceName).trim(),
      port: clampPort(patch.port ?? configStore.config.port),
    });

    if (configStore.error) {
      restoreDraftFromConfig();
      toastStore.error("保存失败");
    } else {
      toastStore.success("保存成功");
    }
  } finally {
    if (!options.keepSaving) {
      basicSettingsSaving.value = false;
    }
  }
}

async function saveDeviceName() {
  const deviceName = draft.deviceName.trim();
  if (!deviceName) {
    draft.deviceName = configStore.config.deviceName;
    toastStore.error("设备名称不能为空");
    return;
  }
  draft.deviceName = deviceName;
  if (deviceName === configStore.config.deviceName) return;
  await saveBasicSettings({ deviceName });
}

async function savePort() {
  if (configStore.saving || basicSettingsSaving.value) {
    restoreDraftFromConfig();
    return;
  }

  const port = clampPort(draft.port);
  draft.port = port;
  if (port === configStore.config.port) return;

  basicSettingsSaving.value = true;
  const wasRunning = statusStore.status.running;
  try {
    if (wasRunning) {
      await statusStore.stop();
      if (statusStore.error) {
        draft.port = configStore.config.port;
        toastStore.error(`停止同步失败：${statusStore.error}`);
        return;
      }
    }

    await saveBasicSettings({ port }, { keepSaving: true });

    if (wasRunning && !configStore.error) {
      await statusStore.start();
      if (statusStore.error) {
        toastStore.error(`端口已保存，同步启动失败：${statusStore.error}`);
      }
    }

    if (wasRunning && configStore.error) {
      await statusStore.start();
    }
  } finally {
    basicSettingsSaving.value = false;
  }
}

async function saveTheme(theme: AppTheme) {
  if (theme === configStore.config.theme) return;
  draft.theme = theme;
  applyThemePreview(theme);
  await saveBasicSettings({ theme });
}

async function saveUiLanguage(uiLanguage: UiLanguage) {
  if (uiLanguage === configStore.config.uiLanguage) return;
  const previousLanguage = configStore.config.uiLanguage;
  draft.uiLanguage = uiLanguage;
  setUiLanguage(uiLanguage);
  await saveBasicSettings({ uiLanguage });
  if (configStore.error) {
    draft.uiLanguage = previousLanguage;
    setUiLanguage(previousLanguage);
  }
}

async function saveCloseAction(closeAction: CloseAction) {
  if (closeAction === configStore.config.closeAction) return;
  draft.closeAction = closeAction;
  await saveBasicSettings({ closeAction });
}

async function saveTranslationSetting(
  patch: Partial<Pick<
    AppConfig,
    | "translationEngine"
    | "translationApiUrl"
    | "translationApiKey"
    | "translationModel"
  >>,
) {
  const normalizedPatch = { ...patch };
  if ("translationApiUrl" in normalizedPatch) {
    normalizedPatch.translationApiUrl = normalizedPatch.translationApiUrl?.trim() ?? "";
  }
  if ("translationApiKey" in normalizedPatch) {
    normalizedPatch.translationApiKey = normalizedPatch.translationApiKey?.trim() ?? "";
  }
  if ("translationModel" in normalizedPatch) {
    normalizedPatch.translationModel = normalizedPatch.translationModel?.trim() || "gpt-4o-mini";
  }

  await saveBasicSettings(normalizedPatch);
}

async function saveTranslationEngine(engine: TranslationEngine) {
  if (engine === configStore.config.translationEngine) return;
  draft.translationEngine = engine;
  await saveTranslationSetting({ translationEngine: engine });
}

async function saveTranslationApiUrl() {
  const translationApiUrl = draft.translationApiUrl.trim();
  draft.translationApiUrl = translationApiUrl;
  if (translationApiUrl === configStore.config.translationApiUrl) return;
  await saveTranslationSetting({ translationApiUrl });
}

async function saveTranslationApiKey() {
  const translationApiKey = draft.translationApiKey.trim();
  draft.translationApiKey = translationApiKey;
  if (translationApiKey === configStore.config.translationApiKey) return;
  await saveTranslationSetting({ translationApiKey });
}

async function saveTranslationModel() {
  const translationModel = draft.translationModel.trim() || "gpt-4o-mini";
  draft.translationModel = translationModel;
  if (translationModel === configStore.config.translationModel) return;
  await saveTranslationSetting({ translationModel });
}

async function saveAutoStart(autoStart: boolean) {
  draft.autoStart = autoStart;
  await saveBasicSettings({ autoStart });
}

async function saveAutoSync(autoSync: boolean) {
  draft.autoSync = autoSync;
  await saveBasicSettings({ autoSync });
}

async function saveAutoOpenFolderAfterSave(autoOpenFolderAfterSave: boolean) {
  draft.autoOpenFolderAfterSave = autoOpenFolderAfterSave;
  await saveBasicSettings({ autoOpenFolderAfterSave });
}

async function saveSyncSetting(
  patch: Partial<
    Pick<
      AppConfig,
      | "syncText"
      | "syncImage"
      | "syncFiles"
      | "syncDirection"
      | "deduplicateSyncContent"
    >
  >,
) {
  if (configStore.saving || syncContentSaving.value) {
    restoreDraftFromConfig();
    return;
  }

  syncContentSaving.value = true;
  Object.assign(draft, patch);

  try {
    await configStore.save({
      ...configStore.config,
      ...patch,
    });

    if (configStore.error) {
      restoreDraftFromConfig();
      toastStore.error("保存失败");
    } else {
      toastStore.success("保存成功");
    }
  } finally {
    syncContentSaving.value = false;
  }
}

async function saveSyncImage(syncImage: boolean) {
  await saveSyncSetting({ syncImage });
}

async function saveSyncText(syncText: boolean) {
  await saveSyncSetting({ syncText });
}

async function saveSyncFiles(syncFiles: boolean) {
  await saveSyncSetting({ syncFiles });
}

async function saveSyncDirection(syncDirection: SyncDirection) {
  await saveSyncSetting({ syncDirection });
}

async function saveDeduplicateSyncContent(deduplicateSyncContent: boolean) {
  await saveSyncSetting({ deduplicateSyncContent });
}

function applySavedConfig(config: AppConfig) {
  configStore.config = config;
  Object.assign(draft, config);
}

async function chooseDownloadLocation() {
  if (downloadLocationSaving.value) return;

  downloadLocationSaving.value = true;
  try {
    const config = await selectTransferSaveDir();
    if (!config) return;
    applySavedConfig(config);
    toastStore.success("下载位置已更新");
  } catch (error) {
    toastStore.error(`设置下载位置失败：${String(error)}`);
  } finally {
    downloadLocationSaving.value = false;
  }
}

async function resetDownloadLocation() {
  if (downloadLocationSaving.value) return;

  downloadLocationSaving.value = true;
  try {
    const config = await resetTransferSaveDir();
    applySavedConfig(config);
    await loadDefaultTransferSaveDir();
    toastStore.success("已恢复默认下载位置");
  } catch (error) {
    toastStore.error(`恢复默认下载位置失败：${String(error)}`);
  } finally {
    downloadLocationSaving.value = false;
  }
}

async function openDownloadLocation() {
  try {
    await openTransferFolder();
  } catch (error) {
    toastStore.error(`打开下载位置失败：${String(error)}`);
  }
}

async function saveNotificationSetting(
  patch: Partial<Pick<AppConfig, NotificationSettingKey>>,
) {
  if (configStore.saving || notificationSettingsSaving.value) {
    restoreDraftFromConfig();
    return;
  }

  notificationSettingsSaving.value = true;
  Object.assign(draft, patch);

  try {
    await configStore.save({
      ...configStore.config,
      ...patch,
    });

    if (configStore.error) {
      restoreDraftFromConfig();
      toastStore.error("保存失败");
    } else {
      toastStore.success("保存成功");
    }
  } finally {
    notificationSettingsSaving.value = false;
  }
}

async function saveDesktopNotifications(desktopNotifications: boolean) {
  await saveNotificationSetting({ desktopNotifications });
}

async function saveNotifyClipboard(notifyClipboard: boolean) {
  await saveNotificationSetting({ notifyClipboard });
}

async function saveNotifyTrustRequired(notifyTrustRequired: boolean) {
  await saveNotificationSetting({ notifyTrustRequired });
}

async function saveNotifyDeviceStatus(notifyDeviceStatus: boolean) {
  await saveNotificationSetting({ notifyDeviceStatus });
}

async function saveNotifySyncError(notifySyncError: boolean) {
  await saveNotificationSetting({ notifySyncError });
}

async function saveNotificationClipboardPreview(notificationClipboardPreview: boolean) {
  await saveNotificationSetting({ notificationClipboardPreview });
}

async function loadCacheSize() {
  if (cacheSizeLoading.value) return;

  cacheSizeLoading.value = true;
  try {
    cacheSizeBytes.value = await getCacheSize();
  } catch (error) {
    toastStore.error(`计算缓存大小失败：${String(error)}`);
  } finally {
    cacheSizeLoading.value = false;
  }
}

async function clearLocalCache() {
  if (cacheClearing.value) return;

  cacheClearing.value = true;
  try {
    cacheSizeBytes.value = await clearCache();
    toastStore.success("缓存已清除");
  } catch (error) {
    toastStore.error(`清除缓存失败：${String(error)}`);
  } finally {
    cacheClearing.value = false;
  }
}
</script>

<template>
  <div data-settings-image2-page class="grid w-full gap-4 pb-4 text-[13px]">
    <section data-settings-image2-section="basic" class="grid gap-2">
      <p class="text-[13px] font-bold text-[color:var(--subtle-text)]">基础设置</p>
      <div
        data-settings-image2-card
        class="overflow-hidden rounded-[10px] border border-[color:var(--main-line)] bg-[color:var(--panel-bg)]"
      >
        <div data-startup-settings data-settings-image2-row class="flex min-h-[50px] items-center justify-between gap-4 px-3 py-3">
          <span class="text-[15px] font-bold text-white">开机启动</span>
          <Switch
            control-only
            :model-value="draft.autoStart"
            label="开机启动"
            :disabled="configMutationSaving"
            @update:model-value="saveAutoStart"
          />
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[50px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="text-[15px] font-bold text-white">启动后自动同步</span>
          <Switch
            control-only
            :model-value="draft.autoSync"
            label="启动后自动同步"
            :disabled="configMutationSaving"
            @update:model-value="saveAutoSync"
          />
        </div>
        <div data-startup-window-setting data-settings-image2-row class="flex min-h-[58px] flex-col items-stretch justify-between gap-3 border-t border-[color:var(--main-line-soft)] px-3 py-3 sm:flex-row sm:items-center sm:gap-4">
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">启动界面</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">下次启动时生效</span>
          </span>
          <div class="flex flex-wrap justify-end gap-2" role="group" aria-label="启动界面">
            <button
              v-for="option in startupWindowOptions"
              :key="option.value"
              type="button"
              class="h-8 rounded-md border px-3 text-[13px] font-bold transition"
              :class="draft.startupWindowMode === option.value
                ? 'border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]'
                : 'border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-muted)] text-slate-300 hover:border-[color:var(--main-line)] hover:text-white'"
              :aria-pressed="draft.startupWindowMode === option.value"
              :disabled="configMutationSaving"
              @click="saveBasicSettings({ startupWindowMode: option.value })"
            >
              {{ option.label }}
            </button>
          </div>
        </div>
        <div
          data-ui-language-setting
          data-settings-image2-row
          class="flex min-h-[58px] items-start justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="pt-1 text-[15px] font-bold text-white">界面语言</span>
          <div data-settings-image2-select class="relative w-full shrink-0 sm:w-[220px]">
            <select
              aria-label="界面语言"
              class="h-9 w-full appearance-none rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] pl-3 pr-9 text-[13px] font-semibold text-[color:var(--clipboard-card-text)] outline-none transition hover:border-[color:var(--main-line)] focus-visible:border-[color:var(--accent-line)] focus-visible:ring-2 focus-visible:ring-[color:var(--accent-soft)]"
              :value="draft.uiLanguage"
              :disabled="configMutationSaving"
              @change="saveUiLanguage(($event.target as HTMLSelectElement).value as UiLanguage)"
            >
              <option
                v-for="option in languageOptions"
                :key="option.value"
                :value="option.value"
                data-i18n-ignore
              >
                {{ option.label }}
              </option>
            </select>
            <ChevronDown class="pointer-events-none absolute right-3 top-2.5 h-4 w-4 text-[color:var(--muted-text)]" aria-hidden="true" />
          </div>
        </div>

        <label
          data-settings-image2-row
          class="flex min-h-[58px] flex-col items-stretch justify-between gap-3 border-t border-[color:var(--main-line-soft)] px-3 py-3 sm:flex-row sm:items-center sm:gap-4"
        >
          <span data-device-name-copy class="grid min-w-0 flex-1 gap-1">
            <span class="text-[15px] font-bold text-white">设备名称</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">用于局域网内识别这台设备</span>
          </span>
          <input
            v-model="draft.deviceName"
            data-device-name-field
            data-settings-image2-field
            class="h-8 w-full min-w-0 shrink-0 rounded-md border-0 bg-[color:var(--field-bg)] px-3 text-[13px] text-white sm:w-[180px]"
            :disabled="configMutationSaving"
            @blur="saveDeviceName"
            @keydown.enter="saveDeviceName"
          >
        </label>

        <label
          data-settings-image2-row
          class="flex min-h-[50px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="text-[15px] font-bold text-white">监听端口</span>
          <input
            v-model.number="draft.port"
            data-settings-image2-field
            class="h-8 w-[112px] rounded-md border-0 bg-[color:var(--field-bg)] px-3 text-center text-[13px] text-white"
            type="number"
            min="1"
            max="65535"
            :disabled="configMutationSaving"
            @change="savePort"
            @blur="savePort"
            @keydown.enter="savePort"
          >
        </label>

        <div
          data-settings-image2-row
          class="flex min-h-[58px] items-start justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="pt-1 text-[15px] font-bold text-white">主题外观</span>
          <div data-settings-image2-select class="relative w-full shrink-0 sm:w-[220px]">
            <select
              aria-label="主题外观"
              class="h-9 w-full appearance-none rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] pl-3 pr-9 text-[13px] font-semibold text-[color:var(--clipboard-card-text)] outline-none transition hover:border-[color:var(--main-line)] focus-visible:border-[color:var(--accent-line)] focus-visible:ring-2 focus-visible:ring-[color:var(--accent-soft)]"
              :value="draft.theme"
              :disabled="configMutationSaving"
              @change="saveTheme(($event.target as HTMLSelectElement).value as AppTheme)"
            >
              <option
                v-for="option in themeOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
            <ChevronDown class="pointer-events-none absolute right-3 top-2.5 h-4 w-4 text-[color:var(--muted-text)]" aria-hidden="true" />
          </div>
        </div>

        <div
          data-close-action-setting
          data-settings-image2-row
          class="flex min-h-[58px] items-start justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="pt-1 text-[15px] font-bold text-white">关闭按钮行为</span>
          <div data-settings-image2-select class="relative w-full shrink-0 sm:w-[220px]">
            <select
              aria-label="关闭按钮行为"
              class="h-9 w-full appearance-none rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] pl-3 pr-9 text-[13px] font-semibold text-[color:var(--clipboard-card-text)] outline-none transition hover:border-[color:var(--main-line)] focus-visible:border-[color:var(--accent-line)] focus-visible:ring-2 focus-visible:ring-[color:var(--accent-soft)]"
              :value="draft.closeAction"
              :disabled="configMutationSaving"
              @change="saveCloseAction(($event.target as HTMLSelectElement).value as CloseAction)"
            >
              <option
                v-for="option in closeActionOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
            <ChevronDown class="pointer-events-none absolute right-3 top-2.5 h-4 w-4 text-[color:var(--muted-text)]" aria-hidden="true" />
          </div>
        </div>
      </div>
    </section>

    <section data-storage-settings class="grid gap-2">
      <p class="text-[13px] font-bold text-[color:var(--subtle-text)]">存储</p>
      <div
        data-download-location-setting
        data-settings-image2-card
        class="overflow-hidden rounded-[10px] border border-[color:var(--main-line)] bg-[color:var(--panel-bg)]"
      >
        <div data-settings-image2-row class="flex min-h-[58px] items-center justify-between gap-4 px-3 py-3">
          <div class="grid min-w-0 flex-1 gap-2">
            <span class="text-[15px] font-bold text-white">下载位置</span>
            <span
              data-settings-image2-field
              data-i18n-ignore
              class="h-8 min-w-0 truncate rounded-md bg-[color:var(--field-bg)] px-3 font-mono text-[13px] leading-8 text-slate-300"
              :title="displayedTransferSaveDir"
            >
              {{ displayedTransferSaveDir }}
            </span>
            <span class="text-[13px] text-[color:var(--muted-text)]">接收文件或复制远端文件时保存到这里</span>
          </div>
          <div class="flex flex-wrap justify-end gap-2">
            <Button
              size="sm"
              variant="secondary"
              :disabled="configMutationSaving"
              @click="chooseDownloadLocation"
            >
              更改位置
            </Button>
            <Button
              size="sm"
              variant="secondary"
              :disabled="configMutationSaving"
              @click="openDownloadLocation"
            >
              打开文件夹
            </Button>
            <Button
              size="sm"
              variant="ghost"
              :disabled="configMutationSaving || !draft.fileSaveDir"
              @click="resetDownloadLocation"
            >
              恢复默认
            </Button>
          </div>
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[50px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">文件保存后操作</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">接收文件保存完成后自动打开文件夹</span>
          </span>
          <Switch
            control-only
            :model-value="draft.autoOpenFolderAfterSave"
            label="自动打开文件夹"
            :disabled="configMutationSaving"
            @update:model-value="saveAutoOpenFolderAfterSave"
          />
        </div>
        <div
          data-cache-management-settings
          data-settings-image2-row
          class="flex min-h-[68px] flex-col gap-3 border-t border-[color:var(--main-line-soft)] px-3 py-3 lg:flex-row lg:items-center lg:justify-between"
        >
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">缓存管理</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">
              包含图片历史、图片缩略图、视频缩略图等本地缓存
            </span>
          </span>
          <div class="flex flex-wrap items-center justify-end gap-2">
            <span
              class="h-8 min-w-[104px] rounded-md bg-[color:var(--field-bg)] px-3 text-center font-mono text-[13px] font-bold leading-8 text-slate-200"
            >
              {{ cacheSizeLabel }}
            </span>
            <Button
              size="sm"
              variant="secondary"
              :disabled="cacheSizeLoading || cacheClearing"
              @click="loadCacheSize"
            >
              刷新大小
            </Button>
            <Button
              size="sm"
              variant="ghost"
              :disabled="cacheClearing || cacheSizeLoading || cacheSizeBytes === 0"
              @click="clearLocalCache"
            >
              清除缓存
            </Button>
          </div>
        </div>
      </div>
    </section>

    <section class="grid gap-2">
      <p class="text-[13px] font-bold text-[color:var(--subtle-text)]">同步内容</p>
      <div
        data-settings-image2-card
        class="overflow-hidden rounded-[10px] border border-[color:var(--main-line)] bg-[color:var(--panel-bg)]"
      >
        <div
          data-sync-direction-setting
          data-settings-image2-row
          class="flex min-h-[58px] flex-col items-stretch justify-between gap-3 px-3 py-3 sm:flex-row sm:items-center sm:gap-4"
        >
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">同步方向</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">选择本机剪贴板内容的发送和接收方式</span>
          </span>
          <div data-settings-image2-select class="relative w-full shrink-0 sm:w-[220px]">
            <select
              aria-label="同步方向"
              class="h-9 w-full appearance-none rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] pl-3 pr-9 text-[13px] font-semibold text-[color:var(--clipboard-card-text)] outline-none transition hover:border-[color:var(--main-line)] focus-visible:border-[color:var(--accent-line)] focus-visible:ring-2 focus-visible:ring-[color:var(--accent-soft)]"
              :value="draft.syncDirection"
              :disabled="configMutationSaving"
              @change="saveSyncDirection(($event.target as HTMLSelectElement).value as SyncDirection)"
            >
              <option v-for="option in syncDirectionOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
            <ChevronDown class="pointer-events-none absolute right-3 top-2.5 h-4 w-4 text-[color:var(--muted-text)]" aria-hidden="true" />
          </div>
        </div>
        <div data-settings-image2-row class="flex min-h-[54px] items-center justify-between gap-4 px-3 py-3">
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">同步文本</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">复制文本后同步到其他设备；关闭不影响本机复制</span>
          </span>
          <Switch control-only :model-value="draft.syncText" label="同步文本" :disabled="configMutationSaving" @update:model-value="saveSyncText" />
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[54px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">同步图片</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">复制截图或图片后同步到其他设备；关闭不影响本机复制</span>
          </span>
          <Switch
            control-only
            :model-value="draft.syncImage"
            label="同步图片"
            :disabled="configMutationSaving"
            @update:model-value="saveSyncImage"
          />
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[54px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">同步文件</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">复制文件后同步到其他设备；关闭不影响本机复制</span>
          </span>
          <Switch
            control-only
            :model-value="draft.syncFiles"
            label="同步文件"
            :disabled="configMutationSaving"
            @update:model-value="saveSyncFiles"
          />
        </div>
        <div
          data-file-transfer-limit-summary
          class="flex min-h-[58px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">文件传输上限</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">
              无论一个还是多个文件，每次发送或接收最多 10 GiB
            </span>
          </span>
          <span class="shrink-0 rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-2.5 py-1 font-mono text-[12px] font-bold text-slate-200">
            最高 10 GiB
          </span>
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[50px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">去重同步内容</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">开启后，相同内容在本次运行期间最多同步一次</span>
          </span>
          <Switch
            control-only
            :model-value="draft.deduplicateSyncContent"
            label="去重同步内容"
            :disabled="configMutationSaving"
            @update:model-value="saveDeduplicateSyncContent"
          />
        </div>
      </div>
    </section>

    <section data-translation-settings class="grid gap-2">
      <p class="text-[13px] font-bold text-[color:var(--subtle-text)]">翻译</p>
      <div
        data-settings-image2-card
        class="overflow-hidden rounded-[10px] border border-[color:var(--main-line)] bg-[color:var(--panel-bg)]"
      >
        <div
          data-settings-image2-row
          class="translation-engine-section"
        >
          <span class="translation-engine-heading">
            <span class="text-[15px] font-bold text-white">翻译方式</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">选择默认使用的翻译服务</span>
          </span>
          <div
            data-translation-engine-picker
            role="group"
            aria-label="翻译方式"
            class="translation-engine-picker"
          >
            <button
              v-for="option in translationEngineOptions"
              :key="option.value"
              type="button"
              class="translation-engine-option"
              :class="{ 'is-selected': draft.translationEngine === option.value }"
              :title="option.hint"
              :aria-pressed="draft.translationEngine === option.value"
              :disabled="configMutationSaving"
              @click="saveTranslationEngine(option.value)"
            >
              <span class="translation-engine-icon"><component :is="option.icon" class="h-4 w-4" /></span>
              <span class="translation-engine-copy">
                <strong>{{ option.label }}</strong>
                <small>{{ option.hint }}</small>
              </span>
              <CheckCircle2 v-if="draft.translationEngine === option.value" class="translation-engine-check" aria-hidden="true" />
            </button>
          </div>
        </div>

        <div
          v-if="draft.translationEngine === 'google'"
          data-translation-google-ready
          class="translation-ready-row"
        >
          <CheckCircle2 class="h-4 w-4 shrink-0 text-[color:var(--accent-text)]" aria-hidden="true" />
          <span class="translation-ready-copy">
            <strong>Google 翻译已启用</strong>
            <span>无需 API Key 或额外设置</span>
          </span>
        </div>

        <div
          v-else
          data-translation-ai-settings
          class="border-t border-[color:var(--main-line-soft)]"
        >
          <div class="flex items-center gap-3 bg-[color:var(--main-bg-muted)] px-3 py-3">
            <span
              class="grid h-8 w-8 shrink-0 place-items-center rounded-md border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]"
            >
              <Sparkles class="h-4 w-4" />
            </span>
            <span class="grid min-w-0 gap-0.5">
              <span class="text-[14px] font-bold text-white">AI 服务配置</span>
              <span class="text-[13px] text-[color:var(--muted-text)]">使用 OpenAI 兼容服务，需要填写以下信息</span>
            </span>
          </div>

          <label
            data-settings-image2-row
            class="flex min-h-[64px] flex-col items-stretch justify-between gap-2 border-t border-[color:var(--main-line-soft)] px-3 py-3 lg:flex-row lg:items-center lg:gap-4"
          >
            <span class="grid min-w-0 flex-1 gap-1">
              <span class="text-[14px] font-bold text-white">服务地址（必填）</span>
              <span class="text-[13px] text-[color:var(--muted-text)]">OpenAI 兼容接口地址</span>
            </span>
            <input
              v-model="draft.translationApiUrl"
              data-settings-image2-field
              class="h-9 w-full rounded-md border-0 bg-[color:var(--field-bg)] px-3 text-[13px] text-white outline-none ring-1 ring-transparent transition focus:ring-[color:var(--accent-line)] lg:w-[min(430px,50vw)]"
              placeholder="https://api.openai.com"
              inputmode="url"
              spellcheck="false"
              :disabled="configMutationSaving"
              @blur="saveTranslationApiUrl"
              @keydown.enter.prevent="saveTranslationApiUrl"
            >
          </label>

          <label
            data-settings-image2-row
            class="flex min-h-[64px] flex-col items-stretch justify-between gap-2 border-t border-[color:var(--main-line-soft)] px-3 py-3 lg:flex-row lg:items-center lg:gap-4"
          >
            <span class="grid min-w-0 flex-1 gap-1">
              <span class="text-[14px] font-bold text-white">API 密钥（必填）</span>
              <span class="text-[13px] text-[color:var(--muted-text)]">仅保存在本机配置文件中</span>
            </span>
            <input
              v-model="draft.translationApiKey"
              data-settings-image2-field
              class="h-9 w-full rounded-md border-0 bg-[color:var(--field-bg)] px-3 text-[13px] text-white outline-none ring-1 ring-transparent transition focus:ring-[color:var(--accent-line)] lg:w-[min(430px,50vw)]"
              type="password"
              placeholder="sk-..."
              autocomplete="off"
              :disabled="configMutationSaving"
              @blur="saveTranslationApiKey"
              @keydown.enter.prevent="saveTranslationApiKey"
            >
          </label>

          <label
            data-settings-image2-row
            class="flex min-h-[64px] flex-col items-stretch justify-between gap-2 border-t border-[color:var(--main-line-soft)] px-3 py-3 lg:flex-row lg:items-center lg:gap-4"
          >
            <span class="grid min-w-0 flex-1 gap-1">
              <span class="text-[14px] font-bold text-white">模型名称</span>
              <span class="text-[13px] text-[color:var(--muted-text)]">使用服务支持的模型名称</span>
            </span>
            <input
              v-model="draft.translationModel"
              data-settings-image2-field
              class="h-9 w-full rounded-md border-0 bg-[color:var(--field-bg)] px-3 text-[13px] text-white outline-none ring-1 ring-transparent transition focus:ring-[color:var(--accent-line)] lg:w-[min(300px,40vw)]"
              placeholder="gpt-4o-mini"
              spellcheck="false"
              :disabled="configMutationSaving"
              @blur="saveTranslationModel"
              @keydown.enter.prevent="saveTranslationModel"
            >
          </label>
        </div>
      </div>
    </section>

    <section data-global-shortcut-settings class="grid gap-2">
      <p class="text-[13px] font-bold text-[color:var(--subtle-text)]">快捷键</p>
      <button
        data-shortcut-settings-entry
        data-settings-image2-card
        type="button"
        class="group flex min-h-[64px] w-full items-center justify-between gap-4 rounded-[10px] border border-[color:var(--main-line)] bg-[color:var(--panel-bg)] px-3 py-3 text-left transition hover:border-[color:var(--accent-line)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-[color:var(--accent-line)]"
        @click="shortcutDialogOpen = true"
      >
        <span class="flex min-w-0 items-center gap-3">
          <span class="grid h-9 w-9 shrink-0 place-items-center rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-muted)] text-[color:var(--accent-text)]">
            <Keyboard class="h-4 w-4" />
          </span>
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">快捷键设置</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">
              已启用 {{ shortcutEnabledCount }} 个，点击统一管理
            </span>
          </span>
        </span>
        <ChevronRight class="h-4 w-4 shrink-0 text-slate-500 transition group-hover:translate-x-0.5 group-hover:text-[color:var(--accent-text)]" />
      </button>
    </section>

    <section data-desktop-notification-settings class="grid gap-2">
      <p class="text-[13px] font-bold text-[color:var(--subtle-text)]">桌面通知</p>
      <div
        data-settings-image2-card
        class="overflow-hidden rounded-[10px] border border-[color:var(--main-line)] bg-[color:var(--panel-bg)]"
      >
        <div data-settings-image2-row class="flex min-h-[54px] items-center justify-between gap-4 px-3 py-3">
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">启用桌面通知</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">剪贴板、信任确认、设备状态与异常提醒</span>
          </span>
          <Switch
            control-only
            :model-value="draft.desktopNotifications"
            label="启用桌面通知"
            :disabled="configMutationSaving"
            @update:model-value="saveDesktopNotifications"
          />
        </div>
        <Transition name="notification-options">
          <div
            v-if="draft.desktopNotifications"
            data-notification-options
            class="notification-options-grid"
          >
            <div data-notification-options-content class="min-h-0 overflow-hidden">
        <div
          data-settings-image2-row
          class="flex min-h-[50px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="text-[15px] font-bold text-white">剪贴板内容提醒</span>
          <Switch
            control-only
            :model-value="draft.notifyClipboard"
            label="剪贴板内容提醒"
            :disabled="configMutationSaving || !draft.desktopNotifications"
            @update:model-value="saveNotifyClipboard"
          />
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[50px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="text-[15px] font-bold text-white">信任确认提醒</span>
          <Switch
            control-only
            :model-value="draft.notifyTrustRequired"
            label="信任确认提醒"
            :disabled="configMutationSaving || !draft.desktopNotifications"
            @update:model-value="saveNotifyTrustRequired"
          />
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[50px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">设备上线/离线提醒</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">发现设备上线或离线时提醒</span>
          </span>
          <Switch
            control-only
            :model-value="draft.notifyDeviceStatus"
            label="设备上线/离线提醒"
            :disabled="configMutationSaving || !draft.desktopNotifications"
            @update:model-value="saveNotifyDeviceStatus"
          />
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[50px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="text-[15px] font-bold text-white">同步异常提醒</span>
          <Switch
            control-only
            :model-value="draft.notifySyncError"
            label="同步异常提醒"
            :disabled="configMutationSaving || !draft.desktopNotifications"
            @update:model-value="saveNotifySyncError"
          />
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[50px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="text-[15px] font-bold text-white">通知中显示剪贴板预览</span>
          <Switch
            control-only
            :model-value="draft.notificationClipboardPreview"
            label="通知中显示剪贴板预览"
            :disabled="configMutationSaving || !draft.desktopNotifications || !draft.notifyClipboard"
            @update:model-value="saveNotificationClipboardPreview"
          />
        </div>
            </div>
          </div>
        </Transition>
      </div>
    </section>

    <p v-if="configStore.error" class="rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-[13px] text-red-100">
      {{ configStore.error }}
    </p>

    <ShortcutSettingsDialog
      :open="shortcutDialogOpen"
      @close="shortcutDialogOpen = false"
    />
  </div>
</template>

<style scoped>
.translation-engine-section {
  display: grid;
  gap: 14px;
  padding: 16px;
}

.translation-engine-heading,
.translation-engine-copy {
  display: grid;
  min-width: 0;
  gap: 3px;
}

.translation-engine-picker {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.translation-engine-option {
  position: relative;
  display: flex;
  min-width: 0;
  min-height: 68px;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  border: 1px solid var(--main-line-soft);
  border-radius: 10px;
  background: var(--field-bg);
  color: var(--clipboard-card-text);
  text-align: left;
  transition: border-color 160ms ease, background 160ms ease;
}

.translation-engine-option:hover:not(:disabled) {
  border-color: var(--main-line);
  background: var(--main-bg-muted);
}

.translation-engine-option:focus-visible {
  outline: 2px solid var(--accent-text);
  outline-offset: 2px;
}

.translation-engine-option:disabled { cursor: not-allowed; opacity: 0.6; }

.translation-engine-option.is-selected {
  border-color: var(--accent-line);
  background: linear-gradient(105deg, var(--accent-soft), var(--field-bg) 70%);
}

.translation-engine-icon {
  display: grid;
  width: 34px;
  height: 34px;
  flex: 0 0 auto;
  place-items: center;
  border: 1px solid var(--main-line-soft);
  border-radius: 9px;
  background: var(--main-bg-soft);
  color: var(--muted-text);
}

.is-selected .translation-engine-icon {
  border-color: var(--accent-line);
  background: var(--accent-soft);
  color: var(--accent-text);
}

.translation-engine-copy strong { font-size: 13px; line-height: 1.3; }
.translation-engine-copy small { color: var(--muted-text); font-size: 12px; line-height: 1.35; }
.translation-engine-check { width: 17px; height: 17px; flex: 0 0 auto; margin-left: auto; color: var(--accent-text); }

.translation-ready-row {
  display: flex;
  min-height: 44px;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
  border-top: 1px solid var(--main-line-soft);
}

.translation-ready-copy { display: flex; min-width: 0; flex-wrap: wrap; align-items: baseline; gap: 2px 12px; }
.translation-ready-copy strong { color: var(--clipboard-card-text); font-size: 13px; }
.translation-ready-copy span { color: var(--muted-text); font-size: 12px; }

@media (max-width: 560px) {
  .translation-engine-picker { grid-template-columns: 1fr; }
  .translation-engine-option { min-height: 60px; }
}

.notification-options-grid {
  display: grid;
  grid-template-rows: 1fr;
}

.notification-options-enter-active,
.notification-options-leave-active {
  overflow: hidden;
  transition:
    grid-template-rows 340ms cubic-bezier(0.22, 1, 0.36, 1),
    opacity 220ms ease,
    transform 340ms cubic-bezier(0.22, 1, 0.36, 1);
}

.notification-options-enter-from,
.notification-options-leave-to {
  grid-template-rows: 0fr;
  opacity: 0;
  transform: translateY(-6px);
}

.notification-options-enter-to,
.notification-options-leave-from {
  grid-template-rows: 1fr;
  opacity: 1;
  transform: translateY(0);
}

@media (prefers-reduced-motion: reduce) {
  .notification-options-enter-active,
  .notification-options-leave-active {
    transition: none;
  }
}
</style>
