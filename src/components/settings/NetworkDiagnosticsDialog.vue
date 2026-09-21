<script setup lang="ts">
import AlertTriangle from "lucide-vue-next/dist/esm/icons/triangle-alert.js";
import Check from "lucide-vue-next/dist/esm/icons/check.js";
import ChevronDown from "lucide-vue-next/dist/esm/icons/chevron-down.js";
import ExternalLink from "lucide-vue-next/dist/esm/icons/external-link.js";
import RefreshCw from "lucide-vue-next/dist/esm/icons/refresh-cw.js";
import ShieldCheck from "lucide-vue-next/dist/esm/icons/shield-check.js";
import Wrench from "lucide-vue-next/dist/esm/icons/wrench.js";
import X from "lucide-vue-next/dist/esm/icons/x.js";
import { computed, nextTick, ref, watch } from "vue";

import Button from "@/components/ui/Button.vue";
import type {
  DiagnosticStatus,
  NetworkDiagnosticReport,
} from "@/types/networkDiagnostics";
import {
  isNetworkDiagnosticInformational,
  isOperationalNetworkDiagnostic,
} from "@/types/networkDiagnostics";

const props = defineProps<{
  open: boolean;
  report: NetworkDiagnosticReport | null;
  loading: boolean;
  repairing: boolean;
  error: string;
  summary: string;
}>();

const emit = defineEmits<{
  close: [];
  refresh: [];
  repair: [];
  openNetworkSettings: [];
}>();

const dialogRef = ref<HTMLElement | null>(null);
const technicalDetailsOpen = ref(false);
const busy = computed(() => props.loading || props.repairing);

const statusPriority: Record<DiagnosticStatus, number> = {
  pass: 0,
  unknown: 1,
  warning: 2,
  error: 3,
};

const summaryGroups = computed(() => {
  if (!props.report) return [];

  const definitions = [
    { label: "文件传输", ids: ["sync-listener"], informational: false },
    { label: "自动发现设备", ids: ["discovery-listener"], informational: false },
    { label: "手机连接", ids: ["mobile-listener"], informational: false },
    {
      label: "Windows 网络与防火墙",
      ids: [
        "windows-network-profile",
        "windows-firewall-profile",
        "firewall-sync",
        "firewall-discovery",
        "firewall-mobile",
      ],
      informational: true,
    },
  ];

  return definitions
    .map((group) => {
      const checks = props.report?.checks.filter((item) => group.ids.includes(item.id)) ?? [];
      const status = checks.reduce<DiagnosticStatus>(
        (worst, item) => statusPriority[item.status] > statusPriority[worst] ? item.status : worst,
        "pass",
      );
      return { ...group, status };
    })
    .filter((group) => group.ids.some((id) => props.report?.checks.some((item) => item.id === id)));
});

const actionableChecks = computed(() =>
  props.report?.checks.filter(
    (item) => isOperationalNetworkDiagnostic(item.id) && item.status !== "pass",
  ) ?? [],
);

const beginnerGuidance = computed(() => {
  const seen = new Set<string>();
  return actionableChecks.value.flatMap((item) => {
    const text = beginnerRecommendation(item.id, item.recommendation);
    if (seen.has(text)) return [];
    seen.add(text);
    return [{ id: item.id, text }];
  });
});

const publicNetworkNeedsAction = computed(() =>
  actionableChecks.value.length > 0
  && props.report?.checks.some(
    (item) => item.id === "windows-network-profile" && item.status !== "pass",
  ),
);

const firewallNeedsRepair = computed(() => {
  if (!props.report?.repairSupported) return false;
  if (!actionableChecks.value.some((item) => item.status === "error")) return false;
  const firewallRuleIds = new Set(["firewall-sync", "firewall-discovery", "firewall-mobile"]);
  return props.report.checks.some(
    (item) => firewallRuleIds.has(item.id) && (item.status === "error" || item.status === "unknown"),
  );
});

const overallState = computed(() => {
  if (actionableChecks.value.some((item) => item.status === "error")) {
    return {
      title: "部分连接功能可能无法使用",
      detail: "按下面的提示处理后，再重新检测一次",
      classes: "border-red-400/25 bg-red-400/8 text-red-100",
    };
  }
  if (actionableChecks.value.length > 0) {
    return {
      title: "网络可以使用，但有设置需要确认",
      detail: "当前同步服务正常，完成下面的设置可以提高连接成功率",
      classes: "border-amber-400/25 bg-amber-400/8 text-amber-100",
    };
  }
  return {
    title: "网络连接正常",
    detail: "其他设备可以发现并连接这台电脑",
    classes: "border-emerald-400/25 bg-emerald-400/8 text-emerald-100",
  };
});

function beginnerRecommendation(id: string, fallback: string | null) {
  return {
    "local-address": "确认电脑已连接家庭或办公 Wi-Fi/网线，并暂时关闭冲突的 VPN",
    "sync-listener": "启动同步；如果仍然失败，请在基础设置中更换监听端口",
    "discovery-listener": "重新启动同步；仍无法发现时，可在设备页输入对方地址连接",
    "mobile-listener": "关闭占用手机连接服务的程序，然后重新检测",
  }[id] ?? fallback ?? "完成设置后重新检测";
}

function diagnosticStatusLabel(status: DiagnosticStatus) {
  return {
    pass: "正常",
    warning: "需确认",
    error: "需处理",
    unknown: "未知",
  }[status];
}

function diagnosticStatusClasses(status: DiagnosticStatus) {
  return {
    pass: "border-emerald-400/30 bg-emerald-400/10 text-emerald-200",
    warning: "border-amber-400/30 bg-amber-400/10 text-amber-100",
    error: "border-red-400/35 bg-red-400/10 text-red-100",
    unknown: "border-slate-500/40 bg-slate-500/10 text-slate-300",
  }[status];
}

function diagnosticStatusTextClasses(status: DiagnosticStatus) {
  return {
    pass: "text-emerald-300",
    warning: "text-amber-200",
    error: "text-red-200",
    unknown: "text-slate-300",
  }[status];
}

function closeDialog() {
  emit("close");
}

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    technicalDetailsOpen.value = false;
    await nextTick();
    dialogRef.value?.focus();
  },
);
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      data-network-diagnostics-dialog
      class="fixed inset-0 z-[90] flex items-center justify-center bg-[color:var(--dialog-overlay-bg)] px-5 py-8 backdrop-blur-sm"
      @click.self="closeDialog"
    >
      <section
        ref="dialogRef"
        class="flex max-h-full w-full max-w-[700px] flex-col overflow-hidden rounded-2xl border border-[color:var(--main-line)] bg-[color:var(--dialog-bg)] text-slate-100 shadow-[0_28px_90px_rgba(0,0,0,0.58)] outline-none"
        role="dialog"
        aria-modal="true"
        aria-labelledby="network-diagnostics-title"
        tabindex="-1"
        @keydown.esc.stop.prevent="closeDialog"
      >
        <header class="flex items-start justify-between gap-4 border-b border-[color:var(--main-line-soft)] px-5 py-4">
          <div class="flex min-w-0 items-start gap-3">
            <span class="grid h-10 w-10 shrink-0 place-items-center rounded-xl border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]">
              <ShieldCheck class="h-5 w-5" />
            </span>
            <span class="grid min-w-0 gap-1">
              <span id="network-diagnostics-title" class="text-[17px] font-bold text-white">网络诊断</span>
              <span class="text-[12px] leading-5 text-[color:var(--muted-text)]">
                {{ summary }}
                <template v-if="report?.preferredLocalIp">
                  · 当前首选 {{ report.preferredLocalIp }}
                </template>
              </span>
            </span>
          </div>
          <button
            type="button"
            class="grid h-8 w-8 shrink-0 place-items-center rounded-lg text-slate-400 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white focus-visible:outline focus-visible:outline-2 focus-visible:outline-[color:var(--accent-line)]"
            aria-label="关闭网络诊断"
            @click="closeDialog"
          >
            <X class="h-4 w-4" />
          </button>
        </header>

        <div class="min-h-0 flex-1 overflow-y-auto px-3 py-2 sm:px-5 sm:py-3">
          <div
            v-if="error"
            data-network-diagnostics-error
            class="rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-3 text-[13px] leading-6 text-red-100"
          >
            <p class="font-bold">网络诊断未完成</p>
            <p class="mt-1 break-words text-red-100/80">{{ error }}</p>
          </div>

          <div v-else-if="report" class="grid gap-3">
            <section
              data-network-diagnostics-summary
              class="rounded-xl border px-4 py-4"
              :class="overallState.classes"
            >
              <div class="flex items-start gap-3">
                <span class="mt-0.5 grid h-8 w-8 shrink-0 place-items-center rounded-full bg-current/10">
                  <Check v-if="actionableChecks.length === 0" class="h-4 w-4" />
                  <AlertTriangle v-else class="h-4 w-4" />
                </span>
                <div class="grid gap-1">
                  <h3 class="text-[15px] font-bold">{{ overallState.title }}</h3>
                  <p class="text-[12px] leading-5 opacity-75">{{ overallState.detail }}</p>
                </div>
              </div>
            </section>

            <section class="overflow-hidden rounded-xl border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg)]">
              <div
                v-for="(group, index) in summaryGroups"
                :key="group.label"
                data-network-summary-item
                class="flex items-center justify-between gap-3 px-4 py-3"
                :class="index > 0 ? 'border-t border-[color:var(--main-line-soft)]' : ''"
              >
                <span class="text-[13px] font-medium text-slate-100">{{ group.label }}</span>
                <span
                  class="flex items-center gap-1.5 text-[12px] font-bold"
                  :class="group.informational ? 'text-slate-400' : diagnosticStatusTextClasses(group.status)"
                >
                  <span class="h-1.5 w-1.5 rounded-full bg-current" />
                  {{ group.informational ? "仅供参考" : group.status === "pass" ? "正常" : group.status === "error" ? "需要处理" : "需要确认" }}
                </span>
              </div>
            </section>

            <section
              v-if="actionableChecks.length > 0"
              data-network-repair-guidance
              class="rounded-xl border border-amber-400/20 bg-amber-400/5 px-4 py-3"
            >
              <h3 class="text-[13px] font-bold text-amber-100">需要做什么</h3>
              <ul class="mt-2 grid gap-2">
                <li
                  v-for="item in beginnerGuidance"
                  :key="item.id"
                  class="flex gap-2 text-[12px] leading-5 text-[color:var(--muted-text)]"
                >
                  <span class="mt-[8px] h-1.5 w-1.5 shrink-0 rounded-full bg-amber-300" />
                  <span>{{ item.text }}</span>
                </li>
              </ul>
            </section>

            <button
              data-network-technical-toggle
              type="button"
              class="flex w-full items-center justify-between rounded-lg px-2 py-2 text-left text-[12px] font-medium text-[color:var(--muted-text)] transition hover:bg-[color:var(--main-bg-muted)] hover:text-white focus-visible:outline focus-visible:outline-2 focus-visible:outline-[color:var(--accent-line)]"
              :aria-expanded="technicalDetailsOpen"
              aria-controls="network-technical-details"
              @click="technicalDetailsOpen = !technicalDetailsOpen"
            >
              <span>{{ technicalDetailsOpen ? "收起技术详情" : "查看技术详情" }}</span>
              <ChevronDown class="h-4 w-4 transition" :class="technicalDetailsOpen ? 'rotate-180' : ''" />
            </button>

            <div
              v-show="technicalDetailsOpen"
              id="network-technical-details"
              data-network-diagnostics-results
              class="overflow-hidden rounded-xl border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg)]"
            >
              <div
                v-for="(item, index) in report.checks"
                :key="item.id"
                :data-network-diagnostic-check="item.id"
                class="grid gap-2.5 px-3 py-3.5 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-start sm:px-4"
                :class="index > 0 ? 'border-t border-[color:var(--main-line-soft)]' : ''"
              >
                <div class="grid min-w-0 gap-1.5">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-[14px] font-bold text-white">{{ item.title }}</span>
                    <span
                      class="rounded-full border px-2 py-0.5 text-[11px] font-bold"
                      :class="isNetworkDiagnosticInformational(item.id)
                        ? 'border-slate-500/40 bg-slate-500/10 text-slate-300'
                        : diagnosticStatusClasses(item.status)"
                    >
                      {{ isNetworkDiagnosticInformational(item.id) ? "参考" : diagnosticStatusLabel(item.status) }}
                    </span>
                  </div>
                  <span class="text-[12px] leading-5 text-[color:var(--muted-text)]">{{ item.detail }}</span>
                  <span
                    v-if="item.recommendation && !isNetworkDiagnosticInformational(item.id)"
                    class="text-[12px] leading-5"
                    :class="item.status === 'error' ? 'text-red-200' : 'text-amber-100/90'"
                  >
                    建议：{{ item.recommendation }}
                  </span>
                </div>
                <span
                  v-if="item.protocol && item.port"
                  class="w-fit rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-2 py-1 font-mono text-[11px] text-slate-300 sm:mt-0.5"
                >
                  {{ item.protocol }} {{ item.port }}
                </span>
              </div>
            </div>
          </div>

          <div
            v-else
            data-network-diagnostics-loading
            class="flex min-h-[180px] flex-col items-center justify-center gap-3 rounded-xl border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg)] px-4 text-center"
          >
            <RefreshCw class="h-5 w-5 animate-spin text-[color:var(--accent-text)]" />
            <span class="text-[13px] text-[color:var(--muted-text)]">正在读取网卡、监听端口和防火墙规则...</span>
          </div>
        </div>

        <footer class="flex flex-wrap items-center justify-end gap-2 border-t border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-muted)] px-5 py-3">
            <Button
              v-if="publicNetworkNeedsAction"
              data-open-network-settings-button
              size="sm"
              variant="primary"
              :disabled="busy"
              @click="emit('openNetworkSettings')"
            >
              <ExternalLink class="h-4 w-4" />
              打开 Windows 网络设置
            </Button>
            <Button
              v-if="firewallNeedsRepair"
              data-firewall-repair-button
              size="sm"
              :variant="publicNetworkNeedsAction ? 'secondary' : 'primary'"
              :disabled="busy"
              @click="emit('repair')"
            >
              <Wrench class="h-4 w-4" />
              {{ repairing ? "等待管理员授权..." : "修复防火墙" }}
            </Button>
            <Button
              data-network-diagnostics-refresh-button
              size="sm"
              variant="secondary"
              :disabled="busy"
              @click="emit('refresh')"
            >
              <RefreshCw class="h-4 w-4" :class="loading ? 'animate-spin' : ''" />
              重新检测
            </Button>
            <Button size="sm" variant="ghost" @click="closeDialog">
              完成
            </Button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>
