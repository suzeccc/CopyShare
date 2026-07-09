<script setup lang="ts">
import { onBeforeUnmount, reactive, ref, watch } from "vue";

import Button from "@/components/ui/Button.vue";
import Switch from "@/components/ui/Switch.vue";
import { clampPort } from "@/lib/format";
import {
  openTransferFolder,
  resetTransferSaveDir,
  selectTransferSaveDir,
  sendTestNotification,
} from "@/lib/tauri";
import { useConfigStore } from "@/stores/config";
import { useStatusStore } from "@/stores/status";
import { useToastStore } from "@/stores/toasts";
import type { AppConfig, AppTheme, CloseAction } from "@/types/config";

const configStore = useConfigStore();
const statusStore = useStatusStore();
const toastStore = useToastStore();

const draft = reactive({ ...configStore.config });
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
const basicSettingsSaving = ref(false);
const syncContentSaving = ref(false);
const notificationSettingsSaving = ref(false);
const downloadLocationSaving = ref(false);

type BasicSettingKey =
  | "deviceName"
  | "port"
  | "theme"
  | "closeAction"
  | "autoStart"
  | "autoSync"
  | "saveHistory"
  | "autoOpenFolderAfterSave";
type NotificationSettingKey =
  | "desktopNotifications"
  | "notifyClipboard"
  | "notifyTrustRequired"
  | "notifyDeviceStatus"
  | "notifySyncError"
  | "notificationClipboardPreview";

function applyThemePreview(theme: AppTheme) {
  document.documentElement.dataset.appTheme = theme;
  document.body.dataset.appTheme = theme;
}

watch(
  () => configStore.config,
  (next) => {
    if (syncContentSaving.value) {
      draft.syncText = next.syncText;
      draft.syncImage = next.syncImage;
      draft.syncFiles = next.syncFiles;
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

async function saveBasicSettings(
  patch: Partial<Pick<AppConfig, BasicSettingKey>>,
  options: {
    keepSaving?: boolean;
    silent?: boolean;
  } = {},
) {
  if (configStore.saving || (basicSettingsSaving.value && !options.keepSaving)) return;

  const previousConfig = { ...configStore.config };
  if (!options.keepSaving) {
    basicSettingsSaving.value = true;
  }

  try {
    await configStore.save({
      ...configStore.config,
      ...patch,
      deviceName: (patch.deviceName ?? configStore.config.deviceName).trim(),
      port: clampPort(patch.port ?? configStore.config.port),
      syncText: true,
    });

    if (configStore.error) {
      Object.assign(draft, previousConfig);
      applyThemePreview(previousConfig.theme);
      toastStore.error("保存失败");
    } else {
      if (!options.silent) {
        toastStore.success("保存成功");
      }
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
  if (configStore.saving || basicSettingsSaving.value) return;

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

async function saveCloseAction(closeAction: CloseAction) {
  if (closeAction === configStore.config.closeAction) return;
  draft.closeAction = closeAction;
  await saveBasicSettings({ closeAction });
}

async function saveAutoStart(autoStart: boolean) {
  draft.autoStart = autoStart;
  await saveBasicSettings({ autoStart }, { silent: true });
}

async function saveAutoSync(autoSync: boolean) {
  draft.autoSync = autoSync;
  await saveBasicSettings({ autoSync }, { silent: true });
}

async function saveHistorySetting(saveHistory: boolean) {
  draft.saveHistory = saveHistory;
  await saveBasicSettings({ saveHistory }, { silent: true });
}

async function saveAutoOpenFolderAfterSave(autoOpenFolderAfterSave: boolean) {
  draft.autoOpenFolderAfterSave = autoOpenFolderAfterSave;
  await saveBasicSettings({ autoOpenFolderAfterSave }, { silent: true });
}

async function saveSyncSetting(
  patch: Partial<Pick<AppConfig, "syncImage" | "syncFiles">>,
  options: { silent?: boolean } = { silent: true },
) {
  if (configStore.saving || syncContentSaving.value) return;

  syncContentSaving.value = true;
  Object.assign(draft, patch);

  try {
    await configStore.save({
      ...configStore.config,
      ...patch,
      syncText: true,
    });

    if (configStore.error) {
      if ("syncImage" in patch) {
        draft.syncImage = configStore.config.syncImage;
      }
      if ("syncFiles" in patch) {
        draft.syncFiles = configStore.config.syncFiles;
      }
      toastStore.error("保存失败");
    } else {
      if (!options.silent) {
        toastStore.success("保存成功");
      }
    }
  } finally {
    syncContentSaving.value = false;
  }
}

async function saveSyncImage(syncImage: boolean) {
  await saveSyncSetting({ syncImage });
}

async function saveSyncFiles(syncFiles: boolean) {
  await saveSyncSetting({ syncFiles });
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
  options: { silent?: boolean } = { silent: true },
) {
  if (configStore.saving || notificationSettingsSaving.value) return;

  notificationSettingsSaving.value = true;
  Object.assign(draft, patch);

  try {
    await configStore.save({
      ...configStore.config,
      ...patch,
      syncText: true,
    });

    if (configStore.error) {
      for (const key of Object.keys(patch) as NotificationSettingKey[]) {
        draft[key] = configStore.config[key];
      }
      toastStore.error("保存失败");
    } else {
      if (!options.silent) {
        toastStore.success("保存成功");
      }
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

async function testDesktopNotification() {
  try {
    await sendTestNotification();
    toastStore.success("测试通知已发送");
  } catch (error) {
    toastStore.error(`测试通知发送失败：${String(error)}`);
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
        <label
          data-settings-image2-row
          class="flex min-h-[58px] items-center justify-between gap-4 px-3 py-3"
        >
          <span class="grid min-w-0 flex-1 gap-2">
            <span class="text-[15px] font-bold text-white">设备名称</span>
            <input
              v-model="draft.deviceName"
              data-settings-image2-field
              class="h-8 min-w-0 rounded-md border-0 bg-[color:var(--field-bg)] px-3 text-[13px] text-white"
              :disabled="basicSettingsSaving"
              @blur="saveDeviceName"
              @keydown.enter="saveDeviceName"
            >
            <span class="text-[13px] text-[color:var(--muted-text)]">用于局域网内识别这台设备</span>
          </span>
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
            :disabled="basicSettingsSaving"
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
          <div data-settings-image2-select class="flex max-w-[620px] flex-wrap justify-end gap-2">
            <button
              v-for="option in themeOptions"
              :key="option.value"
              type="button"
              class="h-8 rounded-md border px-3 text-[13px] font-bold transition"
              :class="draft.theme === option.value
                ? 'border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]'
                : 'border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-muted)] text-slate-300 hover:border-[color:var(--main-line)] hover:text-white'"
              :title="option.hint"
              :disabled="basicSettingsSaving"
              @click="saveTheme(option.value)"
            >
              {{ option.label }}
            </button>
          </div>
        </div>

        <div
          data-close-action-setting
          data-settings-image2-row
          class="flex min-h-[58px] items-start justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="pt-1 text-[15px] font-bold text-white">关闭按钮行为</span>
          <div data-settings-image2-select class="flex max-w-[620px] flex-wrap justify-end gap-2">
            <button
              v-for="option in closeActionOptions"
              :key="option.value"
              type="button"
              class="h-8 rounded-md border px-3 text-[13px] font-bold transition"
              :class="draft.closeAction === option.value
                ? 'border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]'
                : 'border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-muted)] text-slate-300 hover:border-[color:var(--main-line)] hover:text-white'"
              :title="option.hint"
              :disabled="basicSettingsSaving"
              @click="saveCloseAction(option.value)"
            >
              {{ option.label }}
            </button>
          </div>
        </div>
      </div>
    </section>

    <section class="grid gap-2">
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
              class="h-8 min-w-0 truncate rounded-md bg-[color:var(--field-bg)] px-3 font-mono text-[13px] leading-8 text-slate-300"
              :title="draft.fileSaveDir || '默认下载目录'"
            >
              {{ draft.fileSaveDir || "默认下载目录" }}
            </span>
            <span class="text-[13px] text-[color:var(--muted-text)]">接收文件或复制远端文件时保存到这里</span>
          </div>
          <div class="flex flex-wrap justify-end gap-2">
            <Button
              size="sm"
              variant="secondary"
              :disabled="downloadLocationSaving"
              @click="chooseDownloadLocation"
            >
              更改位置
            </Button>
            <Button
              size="sm"
              variant="secondary"
              :disabled="downloadLocationSaving"
              @click="openDownloadLocation"
            >
              打开文件夹
            </Button>
            <Button
              size="sm"
              variant="ghost"
              :disabled="downloadLocationSaving || !draft.fileSaveDir"
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
            :disabled="basicSettingsSaving"
            @update:model-value="saveAutoOpenFolderAfterSave"
          />
        </div>
      </div>
    </section>

    <section class="grid gap-2">
      <p class="text-[13px] font-bold text-[color:var(--subtle-text)]">同步内容</p>
      <div
        data-settings-image2-card
        class="overflow-hidden rounded-[10px] border border-[color:var(--main-line)] bg-[color:var(--panel-bg)]"
      >
        <div data-settings-image2-row class="flex min-h-[54px] items-center justify-between gap-4 px-3 py-3">
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">同步文本</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">文本剪贴板始终同步</span>
          </span>
          <Switch control-only v-model="draft.syncText" label="同步文本" disabled />
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[54px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">同步图片</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">支持截图和图片复制</span>
          </span>
          <Switch
            control-only
            :model-value="draft.syncImage"
            label="同步图片"
            :disabled="syncContentSaving"
            @update:model-value="saveSyncImage"
          />
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[54px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">同步文件</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">复制文件后同步到对方历史</span>
          </span>
          <Switch
            control-only
            :model-value="draft.syncFiles"
            label="同步文件"
            :disabled="syncContentSaving"
            @update:model-value="saveSyncFiles"
          />
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[50px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="text-[15px] font-bold text-white">已信任设备</span>
          <span class="rounded-md bg-[color:var(--field-bg)] px-3 py-1.5 text-[13px] font-bold text-slate-300">
            {{ draft.trustedDevices.length ? `${draft.trustedDevices.length} 台` : "暂无" }}
          </span>
        </div>
      </div>
    </section>

    <section class="grid gap-2">
      <p class="text-[13px] font-bold text-[color:var(--subtle-text)]">历史记录</p>
      <div
        data-settings-image2-card
        class="overflow-hidden rounded-[10px] border border-[color:var(--main-line)] bg-[color:var(--panel-bg)]"
      >
        <div data-settings-image2-row class="flex min-h-[54px] items-center justify-between gap-4 px-3 py-3">
          <span class="grid min-w-0 gap-1">
            <span class="text-[15px] font-bold text-white">保存同步摘要</span>
            <span class="text-[13px] text-[color:var(--muted-text)]">只保存摘要，不保存完整敏感剪贴板内容</span>
          </span>
          <Switch
            control-only
            :model-value="draft.saveHistory"
            label="保存同步摘要"
            :disabled="basicSettingsSaving"
            @update:model-value="saveHistorySetting"
          />
        </div>
      </div>
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
            :disabled="notificationSettingsSaving"
            @update:model-value="saveDesktopNotifications"
          />
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[50px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="text-[15px] font-bold text-white">剪贴板内容提醒</span>
          <Switch
            control-only
            :model-value="draft.notifyClipboard"
            label="剪贴板内容提醒"
            :disabled="notificationSettingsSaving || !draft.desktopNotifications"
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
            :disabled="notificationSettingsSaving || !draft.desktopNotifications"
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
            :disabled="notificationSettingsSaving || !draft.desktopNotifications"
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
            :disabled="notificationSettingsSaving || !draft.desktopNotifications"
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
            :disabled="notificationSettingsSaving || !draft.desktopNotifications || !draft.notifyClipboard"
            @update:model-value="saveNotificationClipboardPreview"
          />
        </div>
        <div
          data-settings-image2-row
          class="flex min-h-[50px] items-center justify-between gap-4 border-t border-[color:var(--main-line-soft)] px-3 py-3"
        >
          <span class="text-[15px] font-bold text-white">发送测试通知</span>
          <Button
            variant="secondary"
            size="sm"
            :disabled="!draft.desktopNotifications"
            @click="testDesktopNotification"
          >
            测试
          </Button>
        </div>
      </div>
    </section>

    <section class="grid gap-2">
      <p class="text-[13px] font-bold text-[color:var(--subtle-text)]">开机启动</p>
      <div
        data-settings-image2-card
        class="overflow-hidden rounded-[10px] border border-[color:var(--main-line)] bg-[color:var(--panel-bg)]"
      >
        <div data-settings-image2-row class="flex min-h-[50px] items-center justify-between gap-4 px-3 py-3">
          <span class="text-[15px] font-bold text-white">开机启动</span>
          <Switch
            control-only
            :model-value="draft.autoStart"
            label="开机启动"
            :disabled="basicSettingsSaving"
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
            :disabled="basicSettingsSaving"
            @update:model-value="saveAutoSync"
          />
        </div>
      </div>
    </section>

    <p v-if="configStore.error" class="rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-[13px] text-red-100">
      {{ configStore.error }}
    </p>
  </div>
</template>
