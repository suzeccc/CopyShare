<script setup lang="ts">
import CircleAlert from "lucide-vue-next/dist/esm/icons/circle-alert.js";
import { computed, ref } from "vue";

import DeviceCard from "@/components/devices/DeviceCard.vue";
import ManualConnectForm from "@/components/devices/ManualConnectForm.vue";
import MobileDeviceCard from "@/components/devices/MobileDeviceCard.vue";
import MobileConnectDialog from "@/components/mobile/MobileConnectDialog.vue";
import NetworkDiagnosticsDialog from "@/components/settings/NetworkDiagnosticsDialog.vue";
import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import RefreshButton from "@/components/ui/RefreshButton.vue";
import {
  getNetworkDiagnostics,
  openWindowsNetworkSettings,
  repairWindowsFirewall,
} from "@/lib/tauri";
import { useDevicesStore } from "@/stores/devices";
import { useHistoryStore } from "@/stores/history";
import { useMobileStore } from "@/stores/mobile";
import { useStatusStore } from "@/stores/status";
import { useToastStore } from "@/stores/toasts";
import type { MobileSessionPhase } from "@/types/mobile";
import {
  isOperationalNetworkDiagnostic,
  type NetworkDiagnosticReport,
} from "@/types/networkDiagnostics";

const devicesStore = useDevicesStore();
const historyStore = useHistoryStore();
const mobileStore = useMobileStore();
const statusStore = useStatusStore();
const toastStore = useToastStore();
const lanDiscoveryScanning = ref(false);
const showMobileConnectDialog = ref(false);
const networkDiagnosticsDialogOpen = ref(false);
const networkDiagnostics = ref<NetworkDiagnosticReport | null>(null);
const networkDiagnosticsLoading = ref(false);
const networkDiagnosticsRepairing = ref(false);
const networkDiagnosticsError = ref("");
const recentIps = computed(() =>
  Array.from(
    new Set(
      devicesStore.history
        .map((device) => device.ip.trim())
        .filter(Boolean),
    ),
  ).slice(0, 8),
);
const mobileHistory = computed(() => {
  const lastSeenAt = [
    mobileStore.history[0]?.lastSeenAt,
    ...historyStore.items
      .filter((item) => item.sourceDevice === "移动端")
      .map((item) => item.createdAt),
  ]
    .filter((value): value is string => Boolean(value))
    .sort((left, right) => Date.parse(right) - Date.parse(left))[0];

  return lastSeenAt
    ? [{
        id: "mobile" as const,
        name: "移动端" as const,
        lastSeenAt,
        status: mobileDeviceStatus(mobileStore.session?.phase),
      }]
    : [];
});

function mobileDeviceStatus(phase: MobileSessionPhase | undefined): "connected" | "waiting" | "offline" {
  if (phase === "waiting") return "waiting";
  if (phase && !["expired", "closed"].includes(phase)) return "connected";
  return "offline";
}
const connectionError = computed(() => devicesStore.error || statusStore.error);
const networkDiagnosticSummary = computed(() => {
  if (networkDiagnosticsLoading.value) return "正在检查网络环境...";
  if (networkDiagnosticsError.value) return "诊断未完成";
  if (!networkDiagnostics.value) return "尚未检测";

  const operationalChecks = networkDiagnostics.value.checks.filter((item) =>
    isOperationalNetworkDiagnostic(item.id),
  );
  const errorCount = operationalChecks.filter(
    (item) => item.status === "error",
  ).length;
  const warningCount = operationalChecks.filter(
    (item) => item.status === "warning" || item.status === "unknown",
  ).length;
  if (errorCount > 0) return `发现 ${errorCount} 项需要处理`;
  if (warningCount > 0) return `${warningCount} 项需要确认`;
  return "局域网入口检查正常";
});

async function scanLanDevices() {
  if (lanDiscoveryScanning.value) {
    return;
  }

  lanDiscoveryScanning.value = true;
  toastStore.info("正在扫描局域网设备...");

  try {
    const { total, newCount } = await devicesStore.scanLanDevices();

    if (newCount > 0) {
      toastStore.success(`发现 ${newCount} 台新设备`);
      return;
    }

    if (total > 0) {
      toastStore.success(`已发现 ${total} 台局域网设备`);
      return;
    }

    toastStore.info("未发现局域网设备，请确认对方已启动 CopyShare 并允许防火墙访问");
  } catch (error) {
    toastStore.error(error instanceof Error ? error.message : "扫描局域网设备失败");
  } finally {
    lanDiscoveryScanning.value = false;
  }
}

function openNetworkDiagnosticsDialog() {
  networkDiagnosticsDialogOpen.value = true;
  if (!networkDiagnostics.value && !networkDiagnosticsLoading.value) {
    void loadNetworkDiagnostics();
  }
}

async function loadNetworkDiagnostics(showSuccess = false) {
  if (networkDiagnosticsLoading.value || networkDiagnosticsRepairing.value) return;

  networkDiagnosticsLoading.value = true;
  networkDiagnosticsError.value = "";
  try {
    networkDiagnostics.value = await getNetworkDiagnostics();
    if (showSuccess) toastStore.success("网络诊断已刷新");
  } catch (error) {
    networkDiagnosticsError.value = String(error);
  } finally {
    networkDiagnosticsLoading.value = false;
  }
}

async function repairFirewall() {
  if (networkDiagnosticsLoading.value || networkDiagnosticsRepairing.value) return;

  networkDiagnosticsRepairing.value = true;
  networkDiagnosticsError.value = "";
  try {
    networkDiagnostics.value = await repairWindowsFirewall();
    toastStore.success("CopyShare 专用网络防火墙规则已修复");
  } catch (error) {
    networkDiagnosticsError.value = String(error);
    toastStore.error(`防火墙修复失败：${String(error)}`);
  } finally {
    networkDiagnosticsRepairing.value = false;
  }
}

async function openSystemNetworkSettings() {
  try {
    await openWindowsNetworkSettings();
  } catch (error) {
    toastStore.error(`无法打开 Windows 网络设置：${String(error)}`);
  }
}
</script>

<template>
  <div class="flex min-h-full flex-col gap-6">
    <section
      class="grid gap-5"
      :class="[
        devicesStore.connected.length ? 'xl:grid-cols-[0.85fr_1.15fr]' : '',
      ]"
    >
      <Card>
        <p class="text-sm font-semibold text-white">快速配置</p>
        <p class="mt-2 text-sm leading-6 text-[color:var(--muted-text)]">
          CopyShare 会自动发现同一局域网内正在运行的设备；也可以手动输入对方 IPv4 地址和端口连接。要双向同步，两台电脑都需要在设备列表里信任对方
        </p>
        <div data-device-action-grid class="mt-5 grid gap-3 lg:grid-cols-2">
          <div
            data-lan-discovery-card
            class="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] p-4"
          >
            <div class="min-w-0 flex-1">
              <p class="text-sm font-semibold text-white">局域网自动发现</p>
              <p class="mt-1 text-xs leading-5 text-[color:var(--muted-text)]">扫描同网段 CopyShare 电脑设备</p>
            </div>
            <Button
              data-lan-discovery-scan-button
              class="shrink-0"
              variant="secondary"
              :disabled="devicesStore.loading || lanDiscoveryScanning"
              @click="scanLanDevices"
            >
              {{ lanDiscoveryScanning ? "正在扫描..." : "扫描局域网设备" }}
            </Button>
          </div>
          <div
            data-mobile-connect-card
            class="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] p-4"
          >
            <div class="min-w-0 flex-1">
              <p class="text-sm font-semibold text-white">手机连接</p>
              <p class="mt-1 text-xs leading-5 text-[color:var(--muted-text)]">手机扫码临时传输剪贴板，无需安装 App</p>
            </div>
            <Button
              data-mobile-connect-dialog-button
              class="shrink-0"
              variant="secondary"
              @click="showMobileConnectDialog = true"
            >
              打开二维码连接
            </Button>
          </div>
        </div>
        <div class="mt-4 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] p-4">
          <ManualConnectForm
            :ip="devicesStore.connectDraft.ip"
            :port="devicesStore.connectDraft.port"
            :recent-ips="recentIps"
            :loading="devicesStore.loading"
            @update:ip="devicesStore.setConnectDraftIp"
            @update:port="devicesStore.setConnectDraftPort"
            @connect="devicesStore.connect"
          />
        </div>
        <div
          v-if="connectionError"
          data-device-connection-error
          class="mt-4 flex items-start gap-3 rounded-lg border border-red-500/35 bg-red-500/10 px-3 py-3 text-red-100"
        >
          <CircleAlert class="mt-0.5 h-5 w-5 shrink-0 text-red-300" />
          <div class="grid min-w-0 flex-1 gap-1">
            <p class="text-[13px] font-bold">连接没有成功</p>
            <p class="text-[12px] leading-5 text-red-100/80">{{ connectionError }}</p>
          </div>
          <Button
            data-device-network-diagnostics-button
            class="shrink-0"
            size="sm"
            variant="secondary"
            @click="openNetworkDiagnosticsDialog"
          >
            开始诊断
          </Button>
        </div>
      </Card>

      <Card v-if="devicesStore.connected.length">
        <div class="flex items-start justify-between gap-4">
          <div>
            <p class="text-sm font-semibold text-white">已连接设备</p>
          <p class="mt-2 text-sm text-[color:var(--muted-text)]">已信任并保持连接的设备，只保留断开操作</p>
          </div>
          <RefreshButton :refresh="() => devicesStore.refresh()" :failed="() => Boolean(devicesStore.error)" />
        </div>
        <div class="mt-5 grid gap-3 md:grid-cols-2">
          <DeviceCard
            v-for="device in devicesStore.connected"
            :key="device.id"
            :device="device"
            mode="connected"
            @disconnect="devicesStore.disconnect"
            @reject="devicesStore.reject"
            @trust="devicesStore.trust"
          />
        </div>
      </Card>
    </section>

    <Card class="flex min-h-[220px] flex-1 flex-col">
      <div class="mb-4 flex items-center justify-between">
        <div>
          <p class="text-sm font-semibold text-white">历史连接设备列表</p>
            <p class="mt-1 text-xs text-[color:var(--muted-text)]">连接成功、等待确认和已断开的设备都会保留在这里，手机扫码使用后也会显示在这里</p>
        </div>
      </div>
      <div v-if="devicesStore.history.length || mobileHistory.length" class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        <DeviceCard
          v-for="device in devicesStore.history"
          :key="device.id"
          :device="device"
          mode="status"
          @disconnect="devicesStore.disconnect"
          @reconnect="devicesStore.connect"
          @reject="devicesStore.reject"
          @trust="devicesStore.trust"
        />
        <MobileDeviceCard
          v-for="device in mobileHistory"
          :key="device.id"
          :device="device"
          @reconnect="showMobileConnectDialog = true"
        />
      </div>
      <div v-else class="flex min-h-0 flex-1 items-center justify-center rounded-lg border border-dashed border-[color:var(--main-line-soft)] px-4 py-10 text-center text-sm text-[color:var(--subtle-text)]">
        连接设备成功后，设备连接记录会显示在这里。设备暂时离线时，也可以查看历史状态并重新发起连接
      </div>
    </Card>

    <MobileConnectDialog v-model="showMobileConnectDialog" />
    <NetworkDiagnosticsDialog
      :open="networkDiagnosticsDialogOpen"
      :report="networkDiagnostics"
      :loading="networkDiagnosticsLoading"
      :repairing="networkDiagnosticsRepairing"
      :error="networkDiagnosticsError"
      :summary="networkDiagnosticSummary"
      @close="networkDiagnosticsDialogOpen = false"
      @refresh="loadNetworkDiagnostics(true)"
      @repair="repairFirewall"
      @open-network-settings="openSystemNetworkSettings"
    />
  </div>
</template>
