<script setup lang="ts">
import { Check, CircleAlert, LoaderCircle } from "lucide-vue-next";
import { computed, onBeforeUnmount, reactive, ref, watch } from "vue";

import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import Switch from "@/components/ui/Switch.vue";
import { clampPort } from "@/lib/format";
import { getSaveFeedbackView, type SaveFeedbackState } from "@/lib/saveFeedback";
import { sendTestNotification } from "@/lib/tauri";
import { useConfigStore } from "@/stores/config";
import { useToastStore } from "@/stores/toasts";
import type { AppConfig, AppTheme, CloseAction } from "@/types/config";

const configStore = useConfigStore();
const toastStore = useToastStore();

const draft = reactive({ ...configStore.config });
const themeOptions: Array<{ value: AppTheme; label: string; hint: string }> = [
  { value: "win11Dark", label: "Win11 深色", hint: "深灰卡片与系统设置风格" },
  { value: "copyBlue", label: "茶话绿", hint: "茶话间深黑绿风格" },
];
const closeActionOptions: Array<{ value: CloseAction; label: string; hint: string }> = [
  { value: "ask", label: "每次询问", hint: "点击关闭时弹出选择提示" },
  { value: "minimize", label: "最小化到托盘", hint: "关闭窗口后继续在后台同步" },
  { value: "exit", label: "直接退出", hint: "关闭窗口时结束应用进程" },
];
const basicSettingsSaving = ref(false);
const syncContentSaving = ref(false);
const notificationSettingsSaving = ref(false);
const saveFeedbackState = ref<SaveFeedbackState>("idle");
let saveFeedbackTimer: number | null = null;
type NotificationSettingKey =
  | "desktopNotifications"
  | "notifyClipboard"
  | "notifyTrustRequired"
  | "notifyFileTransfer"
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
      draft.notifyFileTransfer = next.notifyFileTransfer;
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

const canSave = computed(() => draft.deviceName.trim().length > 0);
const saveFeedbackView = computed(() =>
  getSaveFeedbackView(basicSettingsSaving.value ? "saving" : saveFeedbackState.value),
);
const saveFeedbackIcon = computed(() => {
  if (saveFeedbackView.value.label === "保存中") return LoaderCircle;
  if (saveFeedbackView.value.label === "已保存") return Check;
  if (saveFeedbackView.value.label === "保存失败") return CircleAlert;
  return null;
});
function clearSaveFeedbackTimer() {
  if (saveFeedbackTimer !== null) {
    window.clearTimeout(saveFeedbackTimer);
    saveFeedbackTimer = null;
  }
}

function resetSaveFeedbackLater() {
  clearSaveFeedbackTimer();
  saveFeedbackTimer = window.setTimeout(() => {
    saveFeedbackState.value = "idle";
    saveFeedbackTimer = null;
  }, 1800);
}

onBeforeUnmount(() => {
  clearSaveFeedbackTimer();
  applyThemePreview(configStore.config.theme);
});

async function save() {
  if (!canSave.value || configStore.saving) return;

  clearSaveFeedbackTimer();
  saveFeedbackState.value = "saving";
  basicSettingsSaving.value = true;

  try {
    await configStore.save({
      ...draft,
      deviceName: draft.deviceName.trim(),
      port: clampPort(draft.port),
      syncText: true,
      syncFiles: false,
    });
    saveFeedbackState.value = configStore.error ? "error" : "saved";
    if (configStore.error) {
      toastStore.error("保存失败");
    } else {
      toastStore.success("保存成功");
    }
    resetSaveFeedbackLater();
  } finally {
    basicSettingsSaving.value = false;
  }
}

async function saveSyncSetting(patch: Pick<AppConfig, "syncImage">) {
  if (configStore.saving || syncContentSaving.value) return;

  syncContentSaving.value = true;
  Object.assign(draft, patch);

  try {
    await configStore.save({
      ...configStore.config,
      ...patch,
      syncText: true,
      syncFiles: false,
    });

    if (configStore.error) {
      draft.syncImage = configStore.config.syncImage;
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

async function saveNotificationSetting(patch: Partial<Pick<AppConfig, NotificationSettingKey>>) {
  if (configStore.saving || notificationSettingsSaving.value) return;

  notificationSettingsSaving.value = true;
  Object.assign(draft, patch);

  try {
    await configStore.save({
      ...configStore.config,
      ...patch,
      syncText: true,
      syncFiles: false,
    });

    if (configStore.error) {
      for (const key of Object.keys(patch) as NotificationSettingKey[]) {
        draft[key] = configStore.config[key];
      }
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

async function saveNotifyFileTransfer(notifyFileTransfer: boolean) {
  await saveNotificationSetting({ notifyFileTransfer });
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
  <div class="grid gap-6 xl:grid-cols-[1fr_0.85fr]">
    <Card>
      <p class="text-sm font-semibold text-white">基础设置</p>
      <div class="mt-5 grid gap-4">
        <div data-basic-settings-row class="grid gap-4 sm:grid-cols-[minmax(0,1fr)_160px]">
          <label class="min-w-0">
            <span class="mb-2 block text-xs font-medium text-slate-400">设备名称</span>
            <input
              v-model="draft.deviceName"
              class="h-10 w-full rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 text-sm text-white"
            >
          </label>
          <label class="min-w-[140px]">
            <span class="mb-2 block text-xs font-medium text-slate-400">监听端口</span>
            <input
              v-model.number="draft.port"
              class="h-10 w-full rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 text-sm text-white"
              type="number"
              min="1"
              max="65535"
            >
          </label>
        </div>
        <div>
          <p class="mb-2 text-xs font-medium text-slate-400">主题外观</p>
          <div class="grid gap-2 sm:grid-cols-2">
            <button
              v-for="option in themeOptions"
              :key="option.value"
              type="button"
              class="rounded-lg border px-4 py-3 text-left transition hover:border-[color:var(--main-line)] hover:bg-[color:var(--main-bg-muted)]"
              :class="draft.theme === option.value
                ? 'border-[color:var(--theme-accent)] bg-[color:var(--main-bg-muted)] text-white ring-1 ring-[color:var(--theme-accent)]'
                : 'border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] text-slate-300'"
              @click="draft.theme = option.value"
            >
              <span class="block text-sm font-semibold">{{ option.label }}</span>
              <span class="mt-1 block text-xs text-slate-400">{{ option.hint }}</span>
            </button>
          </div>
        </div>
        <div data-close-action-setting>
          <p class="mb-2 text-xs font-medium text-slate-400">关闭按钮行为</p>
          <div
            data-close-action-options
            class="grid gap-2 rounded-2xl border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] p-1 sm:grid-cols-3"
          >
            <button
              v-for="option in closeActionOptions"
              :key="option.value"
              type="button"
              class="min-h-[58px] rounded-xl px-3 py-2 text-left transition"
              :class="draft.closeAction === option.value
                ? 'bg-[color:var(--main-bg-muted)] text-white shadow-[inset_0_0_0_1px_var(--theme-accent)]'
                : 'text-slate-300 hover:bg-[color:var(--main-bg-soft)] hover:text-white'"
              @click="draft.closeAction = option.value"
            >
              <span class="text-sm font-semibold">{{ option.label }}</span>
              <span class="mt-1 block truncate text-xs leading-4 text-slate-400">{{ option.hint }}</span>
            </button>
          </div>
        </div>
        <Switch v-model="draft.autoStart" label="开机自启" hint="系统登录后自动启动 CopyShare" />
        <Switch v-model="draft.autoSync" label="启动后自动同步" hint="启动应用后自动开始监听剪贴板" />
        <Switch v-model="draft.saveHistory" label="保存同步摘要" hint="只保存摘要，不保存完整敏感剪贴板内容" />
      </div>
      <p v-if="configStore.error" class="mt-4 rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-sm text-red-100">
        {{ configStore.error }}
      </p>
      <div class="mt-5">
        <Button
          variant="primary"
          :class="saveFeedbackView.buttonClass"
          :disabled="!canSave || configStore.saving || saveFeedbackView.disabled"
          @click="save"
        >
          <component
            :is="saveFeedbackIcon"
            v-if="saveFeedbackIcon"
            class="h-4 w-4"
            :class="saveFeedbackView.iconClass"
          />
          {{ saveFeedbackView.label }}
        </Button>
      </div>
    </Card>

    <Card>
      <p class="text-sm font-semibold text-white">同步内容</p>
      <div class="mt-5 grid gap-3">
        <Switch v-model="draft.syncText" label="同步文本" hint="只同步文本剪贴板" disabled />
        <Switch
          :model-value="draft.syncImage"
          label="同步图片"
          hint="支持截图和图片复制"
          :disabled="syncContentSaving"
          @update:model-value="saveSyncImage"
        />
        <Switch v-model="draft.syncFiles" label="同步文件" hint="后续支持" disabled />
      </div>
      <div class="mt-6 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] p-4">
        <p class="text-xs font-medium text-slate-400">已信任设备</p>
        <p class="mt-2 text-sm text-slate-300">
          {{ draft.trustedDevices.length ? `${draft.trustedDevices.length} 台` : "暂无" }}
        </p>
      </div>
    </Card>

    <Card data-desktop-notification-settings>
      <p class="text-sm font-semibold text-white">桌面通知</p>
      <p class="mt-2 text-sm text-slate-400">
        在右下角提醒剪贴板、信任确认、文件传输和同步异常，点击通知会打开对应页面。
      </p>
      <div class="mt-4">
        <Button
          variant="secondary"
          :disabled="!draft.desktopNotifications"
          @click="testDesktopNotification"
        >
          发送测试通知
        </Button>
      </div>
      <div class="mt-5 grid gap-3">
        <Switch
          :model-value="draft.desktopNotifications"
          label="启用桌面通知"
          hint="关闭后不再显示系统右下角通知"
          :disabled="notificationSettingsSaving"
          @update:model-value="saveDesktopNotifications"
        />
        <Switch
          :model-value="draft.notifyClipboard"
          label="剪贴板内容提醒"
          hint="收到其他设备或手机发送的剪贴板内容时提醒"
          :disabled="notificationSettingsSaving || !draft.desktopNotifications"
          @update:model-value="saveNotifyClipboard"
        />
        <Switch
          :model-value="draft.notifyTrustRequired"
          label="信任确认提醒"
          hint="有新设备需要确认信任时提醒"
          :disabled="notificationSettingsSaving || !draft.desktopNotifications"
          @update:model-value="saveNotifyTrustRequired"
        />
        <Switch
          :model-value="draft.notifyFileTransfer"
          label="文件传输提醒"
          hint="收到文件请求、传输完成或失败时提醒"
          :disabled="notificationSettingsSaving || !draft.desktopNotifications"
          @update:model-value="saveNotifyFileTransfer"
        />
        <Switch
          :model-value="draft.notifyDeviceStatus"
          label="设备上线/离线提醒"
          hint="发现设备上线或离线时提醒"
          :disabled="notificationSettingsSaving || !draft.desktopNotifications"
          @update:model-value="saveNotifyDeviceStatus"
        />
        <Switch
          :model-value="draft.notifySyncError"
          label="同步异常提醒"
          hint="连接、监听、剪贴板写入等异常时提醒"
          :disabled="notificationSettingsSaving || !draft.desktopNotifications"
          @update:model-value="saveNotifySyncError"
        />
        <Switch
          :model-value="draft.notificationClipboardPreview"
          label="通知中显示剪贴板预览"
          hint="仅显示前 60 个字符"
          :disabled="notificationSettingsSaving || !draft.desktopNotifications || !draft.notifyClipboard"
          @update:model-value="saveNotificationClipboardPreview"
        />
      </div>
    </Card>
  </div>
</template>
