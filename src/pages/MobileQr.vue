<script setup lang="ts">
import Copy from "lucide-vue-next/dist/esm/icons/copy.js";
import QrCode from "lucide-vue-next/dist/esm/icons/qr-code.js";
import RefreshCw from "lucide-vue-next/dist/esm/icons/refresh-cw.js";
import Smartphone from "lucide-vue-next/dist/esm/icons/smartphone.js";
import X from "lucide-vue-next/dist/esm/icons/x.js";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import { createMobileQrCodeDataUrl } from "@/lib/qrCode";
import { useMobileStore } from "@/stores/mobile";
import { useToastStore } from "@/stores/toasts";
import type { MobileSessionPhase } from "@/types/mobile";

const mobileStore = useMobileStore();
const toastStore = useToastStore();
const qr = ref("");
let pollTimer: number | undefined;

const phaseText = computed(() => getPhaseText(mobileStore.session?.phase));
const submittedItems = computed(() => mobileStore.session?.submittedItems ?? []);
const canUseSession = computed(
  () =>
    Boolean(mobileStore.session) &&
    mobileStore.session?.phase !== "closed" &&
    mobileStore.session?.phase !== "expired",
);

watch(
  () => [mobileStore.session?.url, mobileStore.session?.phase] as const,
  ([url, phase]) => {
    if (phase === "closed" || phase === "expired") {
      stopPolling();
      void renderQr(undefined);
      return;
    }
    void renderQr(url);
  },
  { immediate: true },
);

onMounted(() => {
  if (mobileStore.hasActiveSession) {
    ensurePolling();
    void mobileStore.refreshSession();
  }
});

onBeforeUnmount(() => {
  stopPolling();
});

async function renderQr(url: string | undefined) {
  qr.value = await createMobileQrCodeDataUrl(url);
}

function ensurePolling() {
  if (pollTimer !== undefined) {
    return;
  }

  pollTimer = window.setInterval(() => {
    void mobileStore.refreshSession();
  }, 1200);
}

function stopPolling() {
  window.clearInterval(pollTimer);
  pollTimer = undefined;
}

async function generateQr() {
  try {
    await mobileStore.createSession();
    toastStore.success("已生成手机连接二维码");
    ensurePolling();
  } catch (error) {
    toastStore.error(errorMessage(error, "生成二维码失败"));
  }
}

async function closeSession() {
  if (!mobileStore.session) {
    return;
  }

  try {
    await mobileStore.closeSession();
    stopPolling();
    toastStore.success("已结束手机连接会话");
  } catch (error) {
    toastStore.error(errorMessage(error, "结束会话失败"));
  }
}

async function copyLink() {
  if (!mobileStore.session || !canUseSession.value) {
    return;
  }

  try {
    await navigator.clipboard.writeText(mobileStore.session.url);
    toastStore.success("二维码链接已复制");
  } catch {
    toastStore.error("复制链接失败");
  }
}

function getPhaseText(phase: MobileSessionPhase | undefined) {
  switch (phase) {
    case "waiting":
      return "等待扫码";
    case "opened":
      return "已连接";
    case "copied":
      return "已复制电脑内容";
    case "submitted":
      return "正在写入剪贴板";
    case "written":
      return "已写入剪贴板";
    case "expired":
      return "已过期";
    case "closed":
      return "已结束";
    default:
      return "等待生成";
  }
}

function phaseClass(phase: MobileSessionPhase | undefined) {
  if (phase === "written") {
    return "border-emerald-300/50 bg-emerald-400/14 text-emerald-50 shadow-[0_0_24px_rgba(52,211,153,0.14)]";
  }
  if (phase === "submitted") {
    return "border-orange-300/50 bg-orange-400/14 text-orange-50 shadow-[0_0_24px_rgba(251,146,60,0.14)]";
  }
  if (phase === "opened" || phase === "copied") {
    return "border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)] shadow-[0_0_24px_rgba(45,212,191,0.12)]";
  }
  if (phase === "expired" || phase === "closed") {
    return "border-red-300/50 bg-red-500/12 text-red-50 shadow-[0_0_24px_rgba(248,113,113,0.12)]";
  }
  return "border-white/10 bg-white/[0.05] text-slate-300";
}

function errorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}
</script>

<template>
  <div class="grid w-full min-w-0 max-w-full gap-4 overflow-hidden">
    <section class="flex flex-wrap items-start justify-between gap-4">
      <div class="min-w-0 flex-1">
        <p class="text-xs font-semibold text-[color:var(--accent-text)]">局域网临时传输</p>
        <h2 class="mt-2 text-2xl font-semibold text-white">手机连接</h2>
        <p data-mobile-intro-copy class="mt-2 max-w-none whitespace-nowrap text-sm leading-6 text-[color:var(--muted-text)]">
          用相机扫描二维码，临时传输电脑与手机剪贴板内容，无需安装 App
        </p>
      </div>
      <div class="flex flex-wrap justify-end gap-2">
        <span class="rounded-full border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-4 py-2 text-xs font-medium text-[color:var(--muted-text)] shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]">
          最近 5 条
        </span>
        <span class="rounded-full border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-4 py-2 text-xs font-medium text-[color:var(--muted-text)] shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]">
          单条 100KB
        </span>
        <span class="rounded-full border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-4 py-2 text-xs font-medium text-[color:var(--muted-text)] shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]">
          本次运行期有效
        </span>
      </div>
    </section>

    <Card data-mobile-session-card>
      <div data-mobile-layout="qr-left-content-right" class="grid w-full max-w-full overflow-hidden gap-5 lg:grid-cols-[minmax(320px,0.46fr)_minmax(0,0.54fr)]">
        <aside
          data-mobile-qr-rail
          class="relative grid w-full min-w-0 max-w-full content-start gap-4 overflow-hidden rounded-[24px] border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] p-5 shadow-[0_18px_54px_rgba(0,0,0,0.28)]"
        >
          <div class="flex w-full items-start justify-between gap-3">
            <div class="min-w-0">
              <p class="text-xs font-medium text-[color:var(--subtle-text)]">扫码连接</p>
              <p class="mt-1 truncate text-base font-semibold text-white">CopyShare Mobile</p>
            </div>
            <span class="shrink-0 rounded-full border px-3 py-1 text-xs font-medium" :class="phaseClass(mobileStore.session?.phase)">
              {{ phaseText }}
            </span>
          </div>

          <div data-mobile-qr-zone class="mx-auto grid aspect-square w-full max-w-[320px] place-items-center rounded-[22px] border border-[color:var(--accent-line)] bg-[color:var(--panel-bg-soft)] p-3 shadow-[0_18px_46px_rgba(0,0,0,0.28),0_0_0_1px_var(--accent-soft)_inset]">
            <img v-if="qr" :src="qr" alt="手机连接二维码" class="h-[232px] w-[232px] rounded-[14px]" />
            <div v-else data-mobile-qr-placeholder class="grid place-items-center gap-3 text-center text-[color:var(--accent-text)]">
              <QrCode class="h-14 w-14" />
              <p class="text-sm font-medium text-[color:var(--muted-text)]">等待生成二维码</p>
            </div>
          </div>

          <div class="mx-auto grid w-full max-w-[320px] gap-2">
            <Button class="rounded-full" variant="primary" :disabled="mobileStore.loading" @click="generateQr">
              <RefreshCw class="h-4 w-4" :class="mobileStore.loading ? 'animate-spin' : ''" />
              生成二维码
            </Button>
            <Button class="rounded-full" variant="secondary" :disabled="!canUseSession" @click="copyLink">
              <Copy class="h-4 w-4" />
              复制链接
            </Button>
            <Button class="rounded-full" variant="danger" :disabled="!canUseSession || mobileStore.loading" @click="closeSession">
              <X class="h-4 w-4" />
              结束会话
            </Button>
          </div>

        </aside>

        <div data-mobile-content-panel class="grid w-full max-w-full min-w-0 overflow-hidden content-start gap-4">
          <section class="min-w-0 max-w-full overflow-hidden rounded-[24px] border border-[color:var(--main-line-soft)] bg-white/[0.045] p-4 shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]">
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div class="flex min-w-0 flex-1 items-start gap-3">
                <span class="grid h-11 w-11 shrink-0 place-items-center rounded-full border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]">
                  <Smartphone class="h-5 w-5" />
                </span>
                <div class="min-w-0">
                  <p class="text-base font-semibold text-white">本次临时传输</p>
                  <p class="mt-1 text-sm leading-6 text-[color:var(--muted-text)]">
                    生成二维码后，用手机扫描二维码打开局域网页，无需安装 App
                  </p>
                </div>
              </div>
              <div class="rounded-full border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 py-1.5 text-xs font-medium text-[color:var(--muted-text)]">
                {{ mobileStore.session ? '会话已创建' : '未创建会话' }}
              </div>
            </div>

            <div
              data-mobile-session-lifetime-notice
              class="mt-4 inline-flex rounded-full border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-4 py-2"
            >
              <div class="flex min-w-0 items-center">
                <span class="text-sm font-semibold text-slate-100">保持到手动结束</span>
                <span class="hidden">
                  关闭 CopyShare 或点击结束会话后，手机页面会停止同步
                </span>
              </div>
            </div>
          </section>

          <div data-mobile-summary-grid class="grid min-w-0 max-w-full gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
            <section data-mobile-submit-card class="min-w-0 overflow-hidden rounded-[22px] border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] p-4 lg:col-span-2">
              <div class="flex items-center justify-between gap-3">
                <p class="flex items-center gap-2 text-sm font-semibold text-white">
                  <Smartphone class="h-4 w-4 text-[color:var(--accent-text)]" />
                  手机提交
                </p>
              </div>
              <div v-if="submittedItems.length" class="mt-3 grid max-h-[13rem] gap-2 overflow-auto pr-1">
                <article
                  v-for="(item, index) in submittedItems"
                  :key="item.id"
                  class="rounded-2xl border border-emerald-300/15 bg-emerald-400/[0.06] p-3"
                >
                  <p class="mb-2 text-[11px] font-medium text-emerald-200/80">手机提交 {{ index + 1 }}</p>
                  <p data-i18n-ignore class="line-clamp-4 select-text break-all text-sm leading-6 text-slate-100">{{ item.text }}</p>
                </article>
              </div>
              <p v-else class="mt-3 min-h-[7.5rem] text-sm leading-6 text-[color:var(--muted-text)]">
                待连接
              </p>
              <p class="hidden">
                手机在同一个页面可连续粘贴并发送多条内容，这里会按顺序显示摘要
              </p>
            </section>
          </div>

        </div>
      </div>
    </Card>
  </div>
</template>
