<script setup lang="ts">
import QRCode from "qrcode";
import {
  Copy,
  Monitor,
  QrCode,
  RefreshCw,
  Smartphone,
  X,
} from "lucide-vue-next";
import { computed, onBeforeUnmount, watch, ref } from "vue";

import Button from "@/components/ui/Button.vue";
import { useMobileStore } from "@/stores/mobile";
import { useToastStore } from "@/stores/toasts";
import type { MobileSessionPhase } from "@/types/mobile";

const visible = defineModel<boolean>({ default: false });
const mobileStore = useMobileStore();
const toastStore = useToastStore();
const qr = ref("");
let pollTimer: number | undefined;

const phaseText = computed(() => getPhaseText(mobileStore.session?.phase));
const contentItems = computed(() => mobileStore.session?.contentItems ?? []);
const submittedItems = computed(() => mobileStore.session?.submittedItems ?? []);
const canUseSession = computed(
  () =>
    Boolean(mobileStore.session) &&
    mobileStore.session?.phase !== "closed" &&
    mobileStore.session?.phase !== "expired",
);

watch(
  () => visible.value,
  (isVisible) => {
    if (!isVisible) {
      return;
    }
    if (mobileStore.hasActiveSession) {
      ensurePolling();
      void mobileStore.refreshSession();
      return;
    }
    void generateQr();
  },
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

onBeforeUnmount(() => {
  stopPolling();
});

async function renderQr(url: string | undefined) {
  qr.value = url
    ? await QRCode.toDataURL(url, {
        margin: 1,
        width: 232,
        color: { dark: "#020617", light: "#ffffff" },
      })
    : "";
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

function closeDialog() {
  visible.value = false;
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
      return "已扫码";
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
      return "正在生成";
  }
}

function phaseClass(phase: MobileSessionPhase | undefined) {
  if (phase === "written") {
    return "border-emerald-300/50 bg-emerald-400/14 text-emerald-50";
  }
  if (phase === "submitted") {
    return "border-orange-300/50 bg-orange-400/14 text-orange-50";
  }
  if (phase === "opened" || phase === "copied") {
    return "border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]";
  }
  if (phase === "expired" || phase === "closed") {
    return "border-red-300/50 bg-red-500/12 text-red-50";
  }
  return "border-white/10 bg-white/[0.05] text-slate-300";
}

function errorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}
</script>

<template>
  <Transition name="trust-prompt">
    <div
      v-if="visible"
      data-mobile-connect-dialog
      class="fixed inset-0 z-50 flex items-center justify-center bg-[color:var(--dialog-overlay-bg)] px-6 py-8 backdrop-blur-sm"
      @click.self="closeDialog"
    >
      <section
        class="grid max-h-full w-full max-w-3xl gap-5 overflow-hidden rounded-2xl border border-[color:var(--main-line)] bg-[color:var(--dialog-bg)] p-5 shadow-[0_24px_80px_rgba(0,0,0,0.52)]"
        role="dialog"
        aria-modal="true"
        aria-label="手机连接"
      >
        <div class="flex min-w-0 items-start justify-between gap-4">
          <div class="min-w-0">
            <p class="text-xs font-semibold text-[color:var(--accent-text)]">局域网临时传输</p>
            <h2 class="mt-1 text-xl font-semibold text-white">手机连接</h2>
            <p class="mt-1 text-sm leading-6 text-[color:var(--muted-text)]">
              用手机扫描二维码，临时传输电脑与手机剪贴板内容，无需安装 App。
            </p>
          </div>
          <button
            class="grid h-8 w-8 shrink-0 place-items-center rounded-md text-slate-300 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white"
            type="button"
            aria-label="关闭手机连接"
            title="关闭"
            @click="closeDialog"
          >
            <X class="h-4 w-4" />
          </button>
        </div>

        <div class="grid min-h-0 gap-5 lg:grid-cols-[minmax(260px,0.48fr)_minmax(0,0.52fr)]">
          <aside class="grid content-start gap-4 rounded-[22px] border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] p-4">
            <div class="flex items-start justify-between gap-3">
              <div>
                <p class="text-xs font-medium text-[color:var(--subtle-text)]">扫码连接</p>
                <p class="mt-1 text-base font-semibold text-white">CopyShare Mobile</p>
              </div>
              <span class="shrink-0 rounded-full border px-3 py-1 text-xs font-medium" :class="phaseClass(mobileStore.session?.phase)">
                {{ phaseText }}
              </span>
            </div>

            <div
              data-mobile-connect-qr-zone
              class="mx-auto grid aspect-square w-full max-w-[300px] place-items-center rounded-[20px] border border-[color:var(--accent-line)] bg-[color:var(--panel-bg-soft)] p-3 shadow-[0_18px_46px_rgba(0,0,0,0.28),0_0_0_1px_var(--accent-soft)_inset]"
            >
              <img v-if="qr" :src="qr" alt="手机连接二维码" class="h-[232px] w-[232px] rounded-[14px]" />
              <div v-else class="grid place-items-center gap-3 text-center text-[color:var(--accent-text)]">
                <QrCode class="h-14 w-14" />
                <p class="text-sm font-medium text-[color:var(--muted-text)]">正在生成二维码</p>
              </div>
            </div>

            <div class="grid grid-cols-1 gap-2 sm:grid-cols-3 lg:grid-cols-1">
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

          <div class="grid min-w-0 content-start gap-3 overflow-auto pr-1">
            <section class="rounded-[20px] border border-[color:var(--main-line-soft)] bg-white/[0.045] p-4">
              <div class="flex items-start gap-3">
                <span class="grid h-10 w-10 shrink-0 place-items-center rounded-full border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]">
                  <Smartphone class="h-4 w-4" />
                </span>
                <div>
                  <p class="text-sm font-semibold text-white">保持到手动结束</p>
                  <p class="mt-1 text-sm leading-6 text-[color:var(--muted-text)]">
                    关闭 CopyShare 或点击结束会话后，手机页面会停止同步。
                  </p>
                </div>
              </div>
            </section>

            <section class="rounded-[20px] border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] p-4">
              <p class="flex items-center gap-2 text-sm font-semibold text-white">
                <Monitor class="h-4 w-4 text-[color:var(--accent-text)]" />
                电脑剪贴板
              </p>
              <div v-if="contentItems.length" class="mt-3 grid max-h-32 gap-2 overflow-auto pr-1">
                <article
                  v-for="item in contentItems"
                  :key="item.id"
                  class="rounded-2xl border border-[color:var(--main-line-soft)] bg-black/15 p-3"
                >
                  <p class="line-clamp-3 break-all text-sm leading-6 text-slate-100">{{ item.text }}</p>
                </article>
              </div>
              <p v-else class="mt-3 text-sm leading-6 text-[color:var(--muted-text)]">等待手机扫码后读取。</p>
            </section>

            <section class="rounded-[20px] border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] p-4">
              <p class="flex items-center gap-2 text-sm font-semibold text-white">
                <Smartphone class="h-4 w-4 text-[color:var(--accent-text)]" />
                手机提交
              </p>
              <div v-if="submittedItems.length" class="mt-3 grid max-h-32 gap-2 overflow-auto pr-1">
                <article
                  v-for="item in submittedItems"
                  :key="item.id"
                  class="rounded-2xl border border-emerald-300/15 bg-emerald-400/[0.06] p-3"
                >
                  <p class="line-clamp-3 break-all text-sm leading-6 text-slate-100">{{ item.text }}</p>
                </article>
              </div>
              <p v-else class="mt-3 text-sm leading-6 text-[color:var(--muted-text)]">手机发送后会显示在这里。</p>
            </section>
          </div>
        </div>
      </section>
    </div>
  </Transition>
</template>
