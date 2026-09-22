<script setup lang="ts">
import ArrowLeft from "lucide-vue-next/dist/esm/icons/arrow-left.js";
import ArrowRight from "lucide-vue-next/dist/esm/icons/arrow-right.js";
import CheckCircle2 from "lucide-vue-next/dist/esm/icons/circle-check.js";
import ChevronDown from "lucide-vue-next/dist/esm/icons/chevron-down.js";
import FolderDown from "lucide-vue-next/dist/esm/icons/folder-down.js";
import Laptop from "lucide-vue-next/dist/esm/icons/laptop.js";
import Radio from "lucide-vue-next/dist/esm/icons/radio.js";
import X from "lucide-vue-next/dist/esm/icons/x.js";
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";

import ManualConnectForm from "@/components/devices/ManualConnectForm.vue";
import Button from "@/components/ui/Button.vue";
import Switch from "@/components/ui/Switch.vue";
import { getEffectiveLocale, setUiLanguage } from "@/i18n";
import { getTransferSaveDir, selectTransferSaveDir } from "@/lib/tauri";
import router from "@/router";
import { useConfigStore } from "@/stores/config";
import { useDevicesStore } from "@/stores/devices";
import { useToastStore } from "@/stores/toasts";
import type { UiLanguage } from "@/types/config";

type WizardStep = 1 | 2 | 3;

const props = defineProps<{ review?: boolean }>();
const emit = defineEmits<{ close: [] }>();
const configStore = useConfigStore();
const devicesStore = useDevicesStore();
const toastStore = useToastStore();
const step = ref<WizardStep>(1);
const selectingDirectory = ref(false);
const finishing = ref(false);
const saveError = ref("");
const scanNotice = ref("");
const lanDiscoveryScanning = ref(false);
const manualConnectAvailable = ref(false);
const showManualConnectDialog = ref(false);
const defaultTransferSaveDir = ref("");
const dialogRef = ref<HTMLElement | null>(null);
const previousFocus = document.activeElement as HTMLElement | null;
const draft = reactive({
  deviceName: configStore.config.deviceName,
  fileSaveDir: configStore.config.fileSaveDir,
  autoStart: configStore.config.autoStart,
  autoSync: configStore.config.autoSync,
  uiLanguage: configStore.config.uiLanguage === "system" ? getEffectiveLocale() : configStore.config.uiLanguage,
});
const languageOptions: Array<{ value: UiLanguage; label: string }> = [
  { value: "zh-CN", label: "简体中文" },
  { value: "zh-TW", label: "繁體中文" },
  { value: "en-US", label: "English" },
  { value: "ja-JP", label: "日本語" },
];

const deviceNameValid = computed(() => draft.deviceName.trim().length > 0);
const canContinue = computed(() => step.value !== 1 || deviceNameValid.value);
const busy = computed(() => selectingDirectory.value || finishing.value || configStore.saving);
const displayedTransferSaveDir = computed(() =>
  draft.fileSaveDir || defaultTransferSaveDir.value || "系统默认下载目录",
);
const recentIps = computed(() =>
  [...new Set(devicesStore.history.map((device) => device.ip.trim()).filter(Boolean))].slice(0, 8),
);
const visibleDevices = computed(() =>
  devicesStore.history.filter((device) => device.status === "online" || device.connected).slice(0, 6),
);

onMounted(async () => {
  dialogRef.value?.querySelector<HTMLElement>("[data-onboarding-device-step] input:not(:disabled)")?.focus();
  try {
    defaultTransferSaveDir.value = await getTransferSaveDir();
  } catch {
    // Keep the localized fallback when the native path cannot be resolved.
  }
});

onBeforeUnmount(() => {
  if (draft.uiLanguage !== configStore.config.uiLanguage) setUiLanguage(configStore.config.uiLanguage);
  if (previousFocus?.isConnected) previousFocus.focus();
});

watch(showManualConnectDialog, async (open) => {
  await nextTick();
  dialogRef.value?.querySelector<HTMLElement>(
    open ? "[data-onboarding-ip-dialog] input" : "[data-onboarding-scan]",
  )?.focus();
});

watch(step, () => {
  const scroll = dialogRef.value?.querySelector<HTMLElement>(".onboarding-scroll");
  if (scroll) scroll.scrollTop = 0;
});

function trapFocus(event: KeyboardEvent) {
  const focusRoot = showManualConnectDialog.value
    ? dialogRef.value?.querySelector<HTMLElement>("[data-onboarding-ip-dialog]")
    : dialogRef.value;
  const controls = Array.from(focusRoot?.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled), select:not(:disabled), a[href], [tabindex='0']") ?? [])
    .filter((control) => control.offsetParent !== null);
  const target = event.shiftKey ? controls.at(-1) : controls[0];
  const boundary = event.shiftKey ? controls[0] : controls.at(-1);
  if (target && (document.activeElement === boundary || !controls.includes(document.activeElement as HTMLElement))) {
    event.preventDefault();
    target.focus();
  }
}

const steps = [
  { id: 1, label: "这台电脑", hint: "确认局域网中的名称", icon: Laptop },
  { id: 2, label: "接收方式", hint: "保存位置与启动设置", icon: FolderDown },
  { id: 3, label: "连接设备", hint: "搜索局域网设备", icon: Radio },
] as const;

function previousStep() {
  if (busy.value) return;
  if (step.value > 1) {
    step.value = (step.value - 1) as WizardStep;
  }
}

async function saveDraftConfig(completed: boolean) {
  const deviceName = draft.deviceName.trim();
  if (!deviceName) throw new Error("请输入设备名称");
  await configStore.save({
    ...configStore.config,
    deviceName,
    fileSaveDir: draft.fileSaveDir,
    autoStart: draft.autoStart,
    autoSync: draft.autoSync,
    uiLanguage: draft.uiLanguage,
    onboardingCompleted: completed || configStore.config.onboardingCompleted,
  });
  if (configStore.error) throw new Error(configStore.error);
}

async function nextStep() {
  if (busy.value) return;
  saveError.value = "";
  if (!canContinue.value) {
    saveError.value = "请输入设备名称";
    return;
  }
  if (step.value === 2) {
    finishing.value = true;
    try {
      await saveDraftConfig(false);
      step.value = 3;
    } catch (error) {
      saveError.value = error instanceof Error ? error.message : String(error);
    } finally {
      finishing.value = false;
    }
  } else if (step.value < 3) step.value = (step.value + 1) as WizardStep;
}

async function chooseDirectory() {
  if (busy.value) return;
  selectingDirectory.value = true;
  saveError.value = "";
  try {
    const nextConfig = await selectTransferSaveDir(false);
    if (nextConfig) {
      draft.fileSaveDir = nextConfig.fileSaveDir;
    }
  } catch (error) {
    saveError.value = error instanceof Error ? error.message : String(error);
  } finally {
    selectingDirectory.value = false;
  }
}

async function scanLanDevices() {
  if (lanDiscoveryScanning.value) return;
  lanDiscoveryScanning.value = true;
  manualConnectAvailable.value = false;
  showManualConnectDialog.value = false;
  scanNotice.value = "正在扫描局域网设备...";
  try {
    const { total, newCount } = await devicesStore.scanLanDevices();
    scanNotice.value = newCount > 0
      ? `发现 ${newCount} 台新设备`
      : total > 0 ? `已发现 ${total} 台局域网设备` : "未发现局域网设备，请确认对方已启动 CopyShare";
    if (step.value === 3 && total === 0 && visibleDevices.value.length === 0) {
      manualConnectAvailable.value = true;
      showManualConnectDialog.value = true;
    }
  } catch (error) {
    scanNotice.value = error instanceof Error ? error.message : String(error);
    manualConnectAvailable.value = true;
  } finally {
    lanDiscoveryScanning.value = false;
  }
}

async function connectManual(ip: string, port: number) {
  await devicesStore.connect(ip, port);
  if (!devicesStore.error) showManualConnectDialog.value = false;
}

async function finish(skip = false) {
  if (busy.value) return;
  const deviceName = draft.deviceName.trim();
  if (!skip && !deviceName) {
    step.value = 1;
    saveError.value = "请输入设备名称";
    return;
  }

  finishing.value = true;
  saveError.value = "";
  try {
    if (props.review && skip) {
      emit("close");
      return;
    }
    if (skip) {
      await configStore.save({
        ...configStore.config,
        onboardingCompleted: true,
      });
      if (configStore.error) throw new Error(configStore.error);
    } else await saveDraftConfig(true);
    if (!skip) await router.push("/devices");
    emit("close");
  } catch (error) {
    saveError.value = error instanceof Error ? error.message : String(error);
    toastStore.error("设置保存或打开设备页失败，请重试");
  } finally {
    finishing.value = false;
  }
}
</script>

<template>
  <div
    ref="dialogRef"
    data-onboarding-wizard
    :data-guide-review="props.review || undefined"
    class="onboarding-mask"
    role="dialog"
    aria-modal="true"
    aria-labelledby="onboarding-title"
    @keydown.tab="trapFocus"
    @keydown.esc.stop.prevent="showManualConnectDialog ? showManualConnectDialog = false : finish(true)"
  >
    <div class="onboarding-drag-strip" data-tauri-drag-region></div>
    <section class="onboarding-panel" :aria-hidden="showManualConnectDialog || undefined">
      <aside class="onboarding-route" aria-label="首次设置进度">
        <div class="onboarding-brand">
          <span class="onboarding-brand-mark" aria-hidden="true">
            <span></span>
          </span>
          <span>
            <strong>CopyShare</strong>
            <small>{{ props.review ? "新手指南" : "首次连接" }}</small>
          </span>
        </div>

        <ol>
          <li
            v-for="item in steps"
            :key="item.id"
            :class="{ active: step === item.id, complete: step > item.id }"
          >
            <span class="onboarding-node">
              <Transition name="onboarding-icon" mode="out-in">
                <CheckCircle2 v-if="step > item.id" :key="`done-${item.id}`" class="h-4 w-4" />
                <component :is="item.icon" v-else :key="`step-${item.id}`" class="h-4 w-4" />
              </Transition>
            </span>
            <span class="onboarding-step-copy">
              <strong>{{ item.label }}</strong>
              <small>{{ item.hint }}</small>
            </span>
            <span v-if="item.id < 3" class="onboarding-line" aria-hidden="true"></span>
          </li>
        </ol>

      </aside>

      <main class="onboarding-content">
        <div class="onboarding-actions">
          <button
            type="button"
            data-onboarding-dismiss
            :disabled="busy"
            :title="step === 3 ? '关闭指南，已应用的设置会保留' : '关闭指南，不保存本次修改'"
            @click="finish(true)"
          >{{ props.review ? "关闭指南" : "稍后设置" }}</button>
        </div>
        <div class="onboarding-scroll">
        <Transition name="onboarding-page" mode="out-in" appear>
        <div :key="step" class="onboarding-page" :data-step="step">
        <header>
          <p class="onboarding-eyebrow"><span>设置进度</span><strong>{{ step }}/3</strong></p>
          <template v-if="step === 1">
            <h1 id="onboarding-title">先确认这台电脑</h1>
            <p>这个名称会显示在同一局域网内的其他 CopyShare 设备上。</p>
          </template>
          <template v-else-if="step === 2">
            <h1 id="onboarding-title">决定文件怎么接收</h1>
            <p>选择保存位置和启动行为，继续后应用设置并开始连接设备。</p>
          </template>
          <template v-else>
            <h1 id="onboarding-title">准备连接第一台设备</h1>
            <p>先搜索同一局域网内的设备，找不到时再手动连接。</p>
          </template>
        </header>

        <section v-if="step === 1" data-onboarding-device-step class="onboarding-step-body">
          <label class="onboarding-field">
            <span>设备名称</span>
            <input
              v-model="draft.deviceName"
              :disabled="busy"
              autofocus
              maxlength="32"
              autocomplete="off"
              placeholder="例如：书房电脑"
              @keydown.enter="nextStep"
            />
            <small>建议使用你能一眼认出的名称，最多 32 个字符。</small>
          </label>

          <div class="onboarding-device-preview">
            <span class="onboarding-preview-icon"><Laptop class="h-5 w-5" /></span>
            <span>
              <small>其他设备将看到</small>
              <strong>{{ draft.deviceName.trim() || "未命名设备" }}</strong>
            </span>
            <i>本机</i>
          </div>
        </section>

        <section v-else-if="step === 2" data-onboarding-storage-step class="onboarding-step-body">
          <button
            type="button"
            class="onboarding-folder"
            :disabled="busy"
            @click="chooseDirectory"
          >
            <span class="onboarding-folder-icon"><FolderDown class="h-5 w-5" /></span>
            <span>
              <small>接收文件保存到</small>
              <strong :data-i18n-ignore="Boolean(draft.fileSaveDir || defaultTransferSaveDir) || undefined" :title="displayedTransferSaveDir">
                {{ displayedTransferSaveDir }}
              </strong>
            </span>
            <i>{{ selectingDirectory ? "正在选择" : "更改" }}</i>
          </button>

          <div class="onboarding-setting-row">
            <span class="onboarding-setting-copy">
              <strong>启动后自动同步</strong>
              <small>打开 CopyShare 后立即等待其他设备连接</small>
            </span>
            <span class="onboarding-recommendation">建议开启</span>
            <Switch v-model="draft.autoSync" control-only :disabled="busy" label="启动后自动同步" />
          </div>
          <div class="onboarding-setting-row">
            <span class="onboarding-setting-copy">
              <strong>开机时启动 CopyShare</strong>
              <small>无需手动打开，设备上线后即可同步</small>
            </span>
            <span class="onboarding-recommendation subtle">按需开启</span>
            <Switch v-model="draft.autoStart" control-only :disabled="busy" label="开机时启动 CopyShare" />
          </div>
          <label class="onboarding-setting-row onboarding-language-row">
            <span class="onboarding-setting-copy"><strong>界面语言</strong></span>
            <span class="onboarding-language-picker">
              <select v-model="draft.uiLanguage" data-onboarding-language :disabled="busy" @change="setUiLanguage(draft.uiLanguage)">
                <option v-for="option in languageOptions" :key="option.value" :value="option.value" data-i18n-ignore>{{ option.label }}</option>
              </select>
              <ChevronDown class="h-4 w-4" aria-hidden="true" />
            </span>
          </label>
        </section>

        <section v-else data-onboarding-connect-step class="onboarding-step-body">
          <div class="onboarding-connect-option onboarding-lan-primary" data-onboarding-lan-primary>
            <span class="onboarding-connect-icon"><Radio class="h-4 w-4" /></span>
            <span class="onboarding-connect-copy">
              <strong>局域网自动发现</strong>
              <small>扫描同网段的 CopyShare 电脑</small>
            </span>
            <Button data-onboarding-scan variant="primary" :disabled="busy || lanDiscoveryScanning" @click="scanLanDevices">
              {{ lanDiscoveryScanning ? "正在扫描..." : "扫描局域网设备" }}
            </Button>
          </div>
          <p v-if="scanNotice" data-onboarding-scan-result class="onboarding-connect-message" role="status">{{ scanNotice }}</p>
          <p v-else class="onboarding-connect-message">请先在另一台电脑打开程序，再扫描局域网。</p>

          <div v-if="visibleDevices.length" data-onboarding-discovered-devices class="onboarding-device-results">
            <strong>发现的设备</strong>
            <div v-for="device in visibleDevices" :key="device.id" class="onboarding-device-row">
              <span class="min-w-0">
                <strong data-i18n-ignore class="block truncate">{{ device.name }}</strong>
                <small class="block truncate">{{ device.ip }}:{{ device.port }}</small>
              </span>
              <span v-if="device.connected && device.trusted && device.remoteTrusted" class="onboarding-device-state">已连接</span>
              <Button v-else-if="device.connected && !device.trusted" size="sm" variant="secondary" @click="devicesStore.trust(device.id)">信任设备</Button>
              <span v-else-if="device.connected" class="onboarding-device-state">等待对方信任</span>
              <Button v-else size="sm" variant="secondary" :disabled="devicesStore.loading" @click="devicesStore.connect(device.ip, device.port)">连接</Button>
            </div>
          </div>

          <button
            v-if="manualConnectAvailable"
            data-onboarding-manual-fallback
            type="button"
            class="onboarding-manual-fallback"
            @click="showManualConnectDialog = true"
          >手动连接设备 <ArrowRight class="h-3.5 w-3.5" /></button>
          <p v-if="devicesStore.error" class="onboarding-error" role="alert">{{ devicesStore.error }}</p>
        </section>
        </div>
        </Transition>

        <p v-if="saveError" class="onboarding-error" role="alert">{{ saveError }}</p>
        </div>

        <footer>
          <button
            v-if="step > 1"
            type="button"
            class="onboarding-back"
            :disabled="busy"
            @click="previousStep"
          >
            <ArrowLeft class="h-4 w-4" />
            上一步
          </button>
          <span v-else></span>

          <button
            v-if="step < 3"
            type="button"
            data-onboarding-next
            class="onboarding-primary"
            :disabled="busy || !canContinue"
            @click="nextStep"
          >
            继续
            <ArrowRight class="h-4 w-4" />
          </button>
          <button
            v-else
            type="button"
            data-onboarding-finish
            class="onboarding-primary"
            :disabled="busy"
            @click="finish()"
          >
            {{ finishing ? "正在保存" : "完成" }}
            <ArrowRight class="h-4 w-4" />
          </button>
        </footer>
      </main>
    </section>
    <div
      v-if="showManualConnectDialog"
      data-onboarding-ip-dialog
      class="onboarding-ip-overlay"
      role="dialog"
      aria-modal="true"
      aria-label="手动连接设备"
      @click.self="showManualConnectDialog = false"
    >
      <div class="onboarding-ip-panel">
        <button type="button" class="onboarding-ip-close" aria-label="关闭" @click="showManualConnectDialog = false"><X class="h-4 w-4" /></button>
        <h2>手动连接设备</h2>
        <p>局域网未找到设备时，可以输入对方 IP 地址和端口。</p>
        <ManualConnectForm
          :ip="devicesStore.connectDraft.ip"
          :port="devicesStore.connectDraft.port"
          :recent-ips="recentIps"
          :loading="devicesStore.loading"
          @update:ip="devicesStore.setConnectDraftIp"
          @update:port="devicesStore.setConnectDraftPort"
          @connect="connectManual"
        />
        <p v-if="devicesStore.error" class="onboarding-error" role="alert">{{ devicesStore.error }}</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.onboarding-mask {
  position: fixed;
  inset: 0;
  z-index: 150;
  display: grid;
  place-items: center;
  padding: 18px;
  color: var(--clipboard-card-text);
  background: transparent;
}

.onboarding-drag-strip {
  position: absolute;
  inset: 0 0 auto;
  height: 38px;
}

.onboarding-panel {
  display: grid;
  grid-template-columns: 232px minmax(0, 1fr);
  width: min(840px, 100%);
  height: min(640px, calc(100vh - 36px));
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--main-line);
  border-radius: 20px;
  background: var(--dialog-bg);
  box-shadow: 0 30px 100px rgba(0, 0, 0, 0.46);
}

.onboarding-route {
  display: flex;
  flex-direction: column;
  padding: 24px 18px 18px;
  border-right: 1px solid var(--main-line-soft);
  background:
    linear-gradient(150deg, var(--accent-soft), transparent 42%),
    var(--panel-bg-soft);
}

.onboarding-brand {
  display: flex;
  align-items: center;
  gap: 12px;
}

.onboarding-brand > span:last-child,
.onboarding-step-copy {
  display: flex;
  min-width: 0;
  flex-direction: column;
}

.onboarding-brand strong {
  font-size: 16px;
  letter-spacing: 0.02em;
}

.onboarding-brand small { font-size: 12px; }

.onboarding-brand small,
.onboarding-step-copy small,
.onboarding-content header p,
.onboarding-field small,
.onboarding-preview-icon + span small,
.onboarding-folder small {
  color: var(--subtle-text);
}

.onboarding-brand-mark {
  position: relative;
  width: 40px;
  height: 40px;
  border: 1px solid var(--accent-line);
  border-radius: 11px;
  background: var(--accent-soft);
}

.onboarding-brand-mark::before,
.onboarding-brand-mark::after,
.onboarding-brand-mark span {
  content: "";
  position: absolute;
  top: 18px;
  width: 7px;
  height: 7px;
  border: 2px solid var(--accent-text);
  border-radius: 50%;
}

.onboarding-brand-mark::before { left: 9px; }
.onboarding-brand-mark::after { right: 9px; }
.onboarding-brand-mark span {
  left: 16px;
  width: 8px;
  height: 2px;
  border: 0;
  border-radius: 0;
  background: var(--accent-text);
}

.onboarding-route ol {
  display: grid;
  gap: 4px;
  margin: 30px 0 0;
}

.onboarding-route li {
  position: relative;
  display: grid;
  grid-template-columns: 40px minmax(0, 1fr);
  gap: 12px;
  min-height: 88px;
  align-items: start;
  padding: 9px 8px;
  border-radius: 12px;
  color: var(--subtle-text);
  transition: color 180ms ease, background-color 180ms ease;
}

.onboarding-route li.active,
.onboarding-route li.complete {
  color: var(--clipboard-card-text);
}

.onboarding-route li.active { background: var(--accent-soft); }

.onboarding-node {
  z-index: 1;
  display: grid;
  place-items: center;
  width: 40px;
  height: 40px;
  border: 1px solid var(--main-line);
  border-radius: 50%;
  background: var(--main-bg-soft);
  transition: border-color 180ms ease, background-color 180ms ease, color 180ms ease, transform 180ms ease, box-shadow 180ms ease;
}

.active .onboarding-node,
.complete .onboarding-node {
  border-color: var(--accent-line);
  color: var(--accent-text);
  background: var(--accent-soft);
}

.active .onboarding-node {
  box-shadow: 0 0 0 2px var(--accent-line);
}

.onboarding-step-copy {
  gap: 5px;
  padding-top: 3px;
}

.onboarding-icon-enter-active,
.onboarding-icon-leave-active { transition: opacity 100ms ease, transform 100ms ease; }
.onboarding-icon-enter-from,
.onboarding-icon-leave-to { opacity: 0; transform: scale(0.75); }

.onboarding-step-copy strong {
  font-size: 15px;
  font-weight: 650;
}

.onboarding-step-copy small {
  font-size: 12px;
  line-height: 1.5;
}

.onboarding-line {
  position: absolute;
  top: 49px;
  bottom: -13px;
  left: 27px;
  width: 1px;
  overflow: hidden;
  background: var(--main-line-soft);
}

.complete .onboarding-line { background: var(--accent-line); }

.onboarding-content {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  padding: 24px 28px 22px;
}

.onboarding-actions {
  display: flex;
  flex-shrink: 0;
  justify-content: flex-end;
  margin-bottom: 8px;
}

.onboarding-actions button {
  color: var(--subtle-text);
  font-size: 12px;
}

.onboarding-scroll {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px 4px 2px;
  scrollbar-gutter: stable;
}

.onboarding-page {
  display: flex;
  min-height: 100%;
  flex-direction: column;
  justify-content: center;
  padding: 22px 0 28px;
}

.onboarding-page[data-step="3"] {
  justify-content: flex-start;
  padding-top: 38px;
}

.onboarding-eyebrow {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
  color: var(--accent-text) !important;
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.12em;
}

.onboarding-eyebrow strong {
  font-variant-numeric: tabular-nums;
}

.onboarding-content h1 {
  font-size: 27px;
  font-weight: 680;
  letter-spacing: -0.025em;
}

.onboarding-content header > p:last-child {
  max-width: 510px;
  margin-top: 10px;
  font-size: 14px;
  line-height: 1.7;
}

.onboarding-step-body {
  display: grid;
  gap: 12px;
  margin-top: 24px;
}

.onboarding-page[data-step="1"] .onboarding-field {
  padding: 20px;
  border: 1px solid var(--main-line-soft);
  border-radius: 14px;
  background: var(--panel-bg-soft);
}

.onboarding-page[data-step="1"] .onboarding-device-preview {
  min-height: 76px;
  border-color: var(--accent-line);
  background: var(--accent-soft);
}

.onboarding-page-enter-active { transition: opacity 240ms ease-out; }
.onboarding-page-leave-active { transition: opacity 90ms ease-in; }
.onboarding-page-enter-from,
.onboarding-page-leave-to { opacity: 0; }

.onboarding-setting-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto 44px;
  align-items: center;
  gap: 12px;
  min-height: 66px;
  padding: 12px 14px;
  border: 1px solid var(--main-line-soft);
  border-radius: 11px;
  background: var(--panel-bg-soft);
}

.onboarding-setting-copy {
  display: grid;
  min-width: 0;
  gap: 3px;
  font-size: 13px;
}

.onboarding-setting-copy strong { font-weight: 600; }
.onboarding-setting-copy small { color: var(--subtle-text); font-size: 12px; line-height: 1.45; }

.onboarding-recommendation {
  border: 1px solid var(--accent-line);
  border-radius: 6px;
  padding: 3px 6px;
  color: var(--accent-text);
  background: var(--accent-soft);
  font-size: 11px;
  white-space: nowrap;
}

.onboarding-recommendation.subtle {
  border-color: var(--main-line);
  color: var(--subtle-text);
  background: transparent;
}

.onboarding-language-row { grid-template-columns: minmax(0, 1fr) 174px; }
.onboarding-language-picker { position: relative; min-width: 0; }
.onboarding-language-picker select {
  width: 100%;
  height: 36px;
  appearance: none;
  border: 1px solid var(--main-line);
  border-radius: 8px;
  padding: 0 30px 0 11px;
  color: var(--clipboard-card-text);
  background: var(--field-bg);
  cursor: pointer;
  font-size: 12px;
}

.onboarding-language-picker select:focus-visible { outline: 2px solid var(--accent-text); outline-offset: 2px; }
.onboarding-language-picker select:disabled { cursor: not-allowed; opacity: 0.5; }
.onboarding-language-picker svg { position: absolute; top: 10px; right: 9px; pointer-events: none; color: var(--subtle-text); }

.onboarding-connect-option,
.onboarding-device-row {
  border: 1px solid var(--main-line-soft);
  border-radius: 11px;
  background: var(--panel-bg-soft);
}

.onboarding-connect-option {
  display: grid;
  grid-template-columns: 26px minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
  padding: 12px;
}

.onboarding-connect-copy { display: grid; min-width: 0; gap: 4px; }
.onboarding-connect-option strong,
.onboarding-device-results > strong { font-size: 12px; }
.onboarding-connect-option small,
.onboarding-device-row small,
.onboarding-connect-message { color: var(--muted-text); font-size: 11px; }
.onboarding-connect-option button { white-space: nowrap; }

.onboarding-lan-primary {
  grid-template-columns: 42px minmax(0, 1fr) auto;
  min-height: 96px;
  padding: 18px;
  border-color: var(--accent-line);
  background: linear-gradient(110deg, var(--accent-soft), var(--panel-bg-soft) 75%);
}

.onboarding-lan-primary strong { font-size: 16px; }
.onboarding-lan-primary small { font-size: 12px; }
.onboarding-lan-primary .onboarding-connect-icon { width: 42px; height: 42px; }

.onboarding-connect-icon {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  margin-bottom: 3px;
  border: 1px solid var(--accent-line);
  border-radius: 7px;
  color: var(--accent-text);
  background: var(--accent-soft);
}

.onboarding-connect-message { line-height: 1.5; }
.onboarding-manual-fallback {
  display: inline-flex;
  width: max-content;
  align-items: center;
  gap: 6px;
  color: var(--accent-text);
  font-size: 12px;
  font-weight: 600;
}
.onboarding-manual-fallback:hover { text-decoration: underline; }
.onboarding-manual-fallback:focus-visible { outline: 2px solid var(--accent-line); outline-offset: 3px; }
.onboarding-device-results { display: grid; gap: 8px; }
.onboarding-device-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 9px 12px;
}
.onboarding-device-row strong { font-size: 12px; }
.onboarding-device-state { flex-shrink: 0; color: var(--accent-text); font-size: 11px; }

.onboarding-field {
  display: grid;
  gap: 9px;
}

.onboarding-field > span {
  font-size: 13px;
  font-weight: 600;
}

.onboarding-field input {
  width: 100%;
  height: 50px;
  border: 1px solid var(--main-line);
  border-radius: 10px;
  outline: none;
  padding: 0 14px;
  color: var(--clipboard-card-text);
  background: var(--field-bg);
  transition: border-color 150ms ease, box-shadow 150ms ease;
}

.onboarding-field input:focus {
  border-color: var(--accent-line);
  box-shadow: 0 0 0 3px var(--accent-soft);
}

.onboarding-device-preview,
.onboarding-folder {
  display: flex;
  align-items: center;
  gap: 12px;
  border: 1px solid var(--main-line-soft);
  border-radius: 12px;
  padding: 15px 16px;
  background: var(--panel-bg-soft);
}

.onboarding-preview-icon,
.onboarding-folder-icon {
  display: grid;
  place-items: center;
  width: 42px;
  height: 42px;
  flex: 0 0 auto;
  border-radius: 11px;
  color: var(--accent-text);
  background: var(--accent-soft);
}

.onboarding-device-preview > span:nth-child(2),
.onboarding-folder > span:nth-child(2) {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  gap: 3px;
  text-align: left;
}

.onboarding-device-preview small,
.onboarding-folder small {
  font-size: 12px;
}

.onboarding-device-preview strong,
.onboarding-folder strong { font-size: 14px; }

.onboarding-device-preview i,
.onboarding-folder i {
  flex: 0 0 auto;
  color: var(--accent-text);
  font-size: 11px;
  font-style: normal;
}

.onboarding-folder {
  width: 100%;
  color: inherit;
  cursor: pointer;
}

.onboarding-folder:hover { border-color: var(--accent-line); }
.onboarding-folder:focus-visible { outline: 2px solid var(--accent-line); outline-offset: 2px; }

.onboarding-folder strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.onboarding-error {
  margin-top: 14px;
  color: #fca5a5;
  font-size: 12px;
}

.onboarding-ip-overlay {
  position: absolute;
  inset: 0;
  z-index: 5;
  display: grid;
  place-items: center;
  padding: 18px;
  background: var(--dialog-overlay-bg);
  backdrop-filter: blur(8px);
}

.onboarding-ip-panel {
  position: relative;
  display: grid;
  width: min(520px, 100%);
  gap: 14px;
  border: 1px solid var(--main-line);
  border-radius: 16px;
  padding: 22px;
  background: var(--dialog-bg);
  box-shadow: 0 22px 70px rgb(0 0 0 / 0.35);
}
.onboarding-ip-panel h2 { font-size: 17px; font-weight: 700; }
.onboarding-ip-panel > p { color: var(--muted-text); font-size: 12px; line-height: 1.5; }
.onboarding-ip-close { position: absolute; top: 18px; right: 18px; color: var(--muted-text); }
.onboarding-ip-close:hover { color: var(--clipboard-card-text); }
.onboarding-ip-close:focus-visible { outline: 2px solid var(--accent-line); outline-offset: 3px; }

.onboarding-content footer {
  display: flex;
  flex-shrink: 0;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 12px;
  margin-top: auto;
  padding-top: 16px;
}

.onboarding-back,
.onboarding-primary {
  display: inline-flex;
  height: 44px;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border-radius: 9px;
  padding: 0 16px;
  font-size: 14px;
  font-weight: 600;
  transition: transform 150ms ease, background-color 150ms ease, border-color 150ms ease;
}

.onboarding-back {
  border: 1px solid var(--main-line-soft);
  color: var(--muted-text);
  background: transparent;
}

.onboarding-primary {
  margin-left: auto;
  max-width: 100%;
  height: auto;
  min-height: 44px;
  padding: 8px 16px;
  border: 1px solid var(--button-primary-line);
  color: var(--button-primary-text);
  background: var(--button-primary-bg);
}

.onboarding-primary:hover { background: var(--button-primary-bg-hover); }
.onboarding-back:hover { border-color: var(--main-line); background: var(--main-bg-muted); }
.onboarding-back:active,
.onboarding-primary:active { transform: scale(0.98); }
.onboarding-back:disabled,
.onboarding-primary:disabled,
.onboarding-folder:disabled { cursor: not-allowed; opacity: 0.5; }

@media (max-width: 720px) {
  .onboarding-mask { padding: 12px; }
  .onboarding-panel { grid-template-columns: 1fr; grid-template-rows: auto minmax(0, 1fr); height: min(640px, calc(100vh - 24px)); min-height: 0; }
  .onboarding-route { padding: 12px 16px; border-right: 0; border-bottom: 1px solid var(--main-line-soft); }
  .onboarding-route ol { grid-template-columns: repeat(3, 1fr); gap: 8px; margin-top: 12px; }
  .onboarding-route li { grid-template-columns: 30px minmax(0, 1fr); min-height: auto; gap: 7px; padding: 6px 4px; }
  .onboarding-node { width: 30px; height: 30px; }
  .onboarding-line,
  .onboarding-step-copy small { display: none; }
  .onboarding-step-copy strong { font-size: 12px; }
  .onboarding-content { padding: 12px 16px; }
  .onboarding-page { padding: 12px 0 20px; }
  .onboarding-content header > p:last-child { margin-top: 6px; line-height: 1.5; }
  .onboarding-step-body { gap: 8px; margin-top: 16px; }
  .onboarding-page[data-step="1"] .onboarding-field { padding: 12px; }
  .onboarding-folder { padding: 10px; }
  .onboarding-setting-row { min-height: 56px; gap: 8px; padding: 8px 10px; }
  .onboarding-content footer { padding-top: 8px; }
}

@media (max-width: 520px) {
  .onboarding-connect-option,
  .onboarding-lan-primary { grid-template-columns: 42px minmax(0, 1fr); }
  .onboarding-connect-option button { grid-column: 2; justify-self: start; }
  .onboarding-setting-row { grid-template-columns: minmax(0, 1fr) auto; }
  .onboarding-setting-row .onboarding-recommendation { grid-column: 1; grid-row: 2; justify-self: start; }
  .onboarding-setting-row .onboarding-setting-copy { grid-column: 1; }
  .onboarding-setting-row .onboarding-language-picker,
  .onboarding-setting-row > label { grid-column: 2; grid-row: 1; }
  .onboarding-language-row { grid-template-columns: minmax(0, 1fr) 154px; }
}

@media (prefers-reduced-motion: reduce) {
  .onboarding-route li,
  .onboarding-node,
  .onboarding-icon-enter-active,
  .onboarding-icon-leave-active,
  .onboarding-back,
  .onboarding-primary,
  .onboarding-field input { transition: none; }
}

@media (max-width: 720px) and (max-height: 480px) {
  .onboarding-brand { display: none; }
  .onboarding-route { padding: 10px 14px; }
  .onboarding-route ol { margin-top: 0; }
  .onboarding-route li { grid-template-columns: 18px minmax(0, 1fr); gap: 6px; }
  .onboarding-node { width: 18px; height: 18px; }
  .onboarding-step-copy strong { font-size: 11px; }
  .onboarding-content { padding: 10px 14px; }
  .onboarding-actions { margin-bottom: 6px; }
  .onboarding-content h1 { font-size: 18px; }
  .onboarding-step-body { margin-top: 16px; }
}
</style>
