<script setup lang="ts">
import { FileText, FolderOpen, Image as ImageIcon, Monitor, Network, Settings, X } from "lucide-vue-next";
import { computed, ref } from "vue";
import { RouterLink } from "vue-router";

import SyncSwitch from "@/components/status/SyncSwitch.vue";
import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import CopyTextButton from "@/components/ui/CopyTextButton.vue";
import { formatTime } from "@/lib/format";
import { CLIPBOARD_PREVIEW_LIMIT, getRecentClipboardItems } from "@/lib/historyPreview";
import { useConfigStore } from "@/stores/config";
import { useHistoryStore } from "@/stores/history";
import { useStatusStore } from "@/stores/status";

const statusStore = useStatusStore();
const configStore = useConfigStore();
const historyStore = useHistoryStore();
const showClipboardHistoryModal = ref(false);

const address = computed(() => {
  const ip = statusStore.status.localIp;
  return ip ? `${ip}:${statusStore.status.port}` : "等待网络地址";
});

const syncContentItems = computed(() => [
  {
    label: "文本剪贴板",
    hint: "复制文本后自动广播给已信任设备",
    state: configStore.config.syncText ? "已启用" : "已关闭",
    enabled: configStore.config.syncText,
    icon: FileText,
  },
  {
    label: "图片",
    hint: "后续版本支持图片剪贴板",
    state: "暂未开放",
    enabled: false,
    icon: ImageIcon,
  },
  {
    label: "文件",
    hint: "后续版本支持文件列表同步",
    state: "暂未开放",
    enabled: false,
    icon: FolderOpen,
  },
]);

const recentSyncItems = computed(() => getRecentClipboardItems(historyStore.items));
const allClipboardItems = computed(() =>
  getRecentClipboardItems(historyStore.items, historyStore.items.length),
);
</script>

<template>
  <div class="grid gap-4">
    <section class="grid gap-4">
      <Card>
        <div class="flex h-full flex-col justify-between gap-5">
          <div class="flex flex-wrap items-start justify-between gap-4">
            <div>
              <p class="text-xs font-medium text-[color:var(--accent-text)]">局域网剪贴板同步</p>
              <h2 class="mt-2 text-2xl font-semibold text-white">Copy-Sharer</h2>
              <p class="mt-3 max-w-2xl text-sm leading-6 text-slate-400">
                监听本机文本剪贴板，通过 WebSocket 同步给已信任的局域网设备。
              </p>
            </div>
            <SyncSwitch
              :running="statusStore.status.running"
              :loading="statusStore.loading"
              @start="statusStore.start()"
              @stop="statusStore.stop()"
            />
          </div>

          <div class="grid gap-3 lg:grid-cols-3">
            <div class="rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--stat-bg)] px-4 py-3">
              <p class="text-xs text-slate-500">同步状态</p>
              <p class="mt-1.5 text-xl font-semibold text-white">{{ statusStore.statusLabel }}</p>
            </div>
            <div class="rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--stat-bg)] px-4 py-3">
              <p class="text-xs text-slate-500">已连接设备</p>
              <p class="mt-1.5 text-xl font-semibold text-white">{{ statusStore.status.connectedCount }} 台</p>
            </div>
            <div class="rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--stat-bg)] px-4 py-3">
              <p class="text-xs text-slate-500">最近同步</p>
              <p class="mt-1.5 truncate text-xl font-semibold text-white">{{ formatTime(statusStore.status.lastSyncAt) }}</p>
            </div>
          </div>

          <p
            v-if="statusStore.status.message"
            class="rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-4 py-3 text-sm leading-6 text-slate-200"
          >
            {{ statusStore.status.message }}
          </p>
        </div>

        <p v-if="statusStore.error" class="mt-4 rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-sm text-red-100">
          {{ statusStore.error }}
        </p>
      </Card>
    </section>

    <section class="grid gap-3 md:grid-cols-[1fr_1fr]">
      <Card compact>
        <div class="flex h-full flex-col justify-between gap-4">
          <div class="flex items-center justify-between gap-3">
            <p class="text-base font-semibold text-white">快速操作</p>
            <p class="truncate text-xs text-slate-500">常用入口</p>
          </div>
          <div class="grid gap-3 sm:grid-cols-3">
            <RouterLink
              to="/devices"
              class="group flex min-h-16 items-center justify-center gap-3 rounded-lg border border-[color:var(--main-line)] bg-[color:var(--main-bg-muted)] px-3 py-3 text-sm font-semibold text-white transition hover:border-[color:var(--accent-line)] hover:bg-[color:var(--stat-bg)]"
            >
              <span class="grid h-9 w-9 shrink-0 place-items-center rounded-md bg-[color:var(--accent-soft)] text-[color:var(--accent-text)] ring-1 ring-[color:var(--accent-line)] transition group-hover:bg-[color:var(--accent-soft)]">
                <Network class="h-5 w-5" />
              </span>
              <span class="whitespace-nowrap">连接设备</span>
            </RouterLink>
            <RouterLink
              to="/logs"
              class="group flex min-h-16 items-center justify-center gap-3 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--stat-bg)] px-3 py-3 text-sm font-semibold text-slate-200 transition hover:border-[color:var(--accent-line)] hover:bg-[color:var(--main-bg-muted)] hover:text-white"
            >
              <span class="grid h-9 w-9 shrink-0 place-items-center rounded-md bg-white/[0.06] text-slate-100 ring-1 ring-white/10 transition group-hover:bg-[color:var(--accent-soft)] group-hover:text-[color:var(--accent-text)]">
                <Monitor class="h-5 w-5" />
              </span>
              <span class="whitespace-nowrap">日志</span>
            </RouterLink>
            <RouterLink
              to="/settings"
              class="group flex min-h-16 items-center justify-center gap-3 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--stat-bg)] px-3 py-3 text-sm font-semibold text-slate-200 transition hover:border-[color:var(--accent-line)] hover:bg-[color:var(--main-bg-muted)] hover:text-white"
            >
              <span class="grid h-9 w-9 shrink-0 place-items-center rounded-md bg-white/[0.06] text-slate-100 ring-1 ring-white/10 transition group-hover:bg-[color:var(--accent-soft)] group-hover:text-[color:var(--accent-text)]">
                <Settings class="h-5 w-5" />
              </span>
              <span class="whitespace-nowrap">设置</span>
            </RouterLink>
          </div>
        </div>
      </Card>

      <Card compact>
        <div class="grid h-full gap-3">
          <div class="flex items-start gap-3">
            <Monitor class="mt-0.5 h-5 w-5 text-[color:var(--accent-text)]" />
            <div class="flex min-w-0 flex-1 flex-wrap items-center justify-between gap-3">
              <div class="min-w-0">
                <p class="truncate text-sm font-medium text-white">{{ statusStore.status.deviceName }}</p>
                <p class="mt-1 truncate font-mono text-xs text-slate-500">{{ statusStore.status.deviceId }}</p>
              </div>
              <span class="rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--stat-bg)] px-3 py-1 text-sm font-semibold text-slate-200">
                {{ statusStore.status.running ? "运行中" : "等待启动" }}
              </span>
            </div>
          </div>
          <div data-home-device-fields class="grid gap-2 text-sm text-slate-300 sm:grid-cols-[0.62fr_1.38fr]">
            <p data-home-port-block class="flex min-w-0 items-center justify-between gap-3 rounded-md bg-[color:var(--field-bg)] px-3 py-2">
              <span class="shrink-0 whitespace-nowrap text-slate-500">监听端口</span>
              <span class="font-mono">{{ statusStore.status.port }}</span>
            </p>
            <p data-home-address-block class="flex min-w-0 items-center justify-between gap-2 rounded-md bg-[color:var(--field-bg)] px-3 py-2">
              <span class="shrink-0 whitespace-nowrap text-slate-500">本机地址</span>
              <span data-home-address-value class="whitespace-nowrap text-right font-mono text-slate-100" :title="address">{{ address }}</span>
            </p>
          </div>
        </div>
      </Card>
    </section>

    <section class="grid gap-4">
      <Card>
        <div class="mb-5 flex flex-wrap items-start justify-between gap-4">
          <div>
            <p class="text-sm font-semibold text-white">同步内容</p>
            <p class="mt-2 text-sm leading-6 text-slate-400">
              当前版本明确展示可同步内容，避免只在设置里隐藏开关。
            </p>
          </div>
          <RouterLink to="/settings">
            <Button size="sm" variant="ghost">
              <Settings class="h-4 w-4" />
              内容设置
            </Button>
          </RouterLink>
        </div>

        <div class="grid gap-3 md:grid-cols-3">
          <article
            v-for="item in syncContentItems"
            :key="item.label"
            class="rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--stat-bg)] p-4"
          >
            <div class="mb-4 flex items-center justify-between gap-3">
              <div class="grid h-9 w-9 place-items-center rounded-md border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)]">
                <component :is="item.icon" class="h-4 w-4 text-[color:var(--accent-text)]" />
              </div>
              <span
                class="rounded-md px-2 py-1 text-xs font-medium"
                :class="item.enabled ? 'bg-emerald-500/[0.14] text-emerald-200' : 'bg-[color:var(--field-bg)] text-slate-400'"
              >
                {{ item.state }}
              </span>
            </div>
            <p class="text-sm font-semibold text-white">{{ item.label }}</p>
            <p class="mt-2 text-xs leading-5 text-slate-500">{{ item.hint }}</p>
          </article>
        </div>

        <div class="mt-4 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--stat-bg)] p-4">
          <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
            <p class="text-sm font-semibold text-white">最近同步内容</p>
            <div class="flex items-center gap-2">
              <p class="text-xs text-slate-500">最近 {{ CLIPBOARD_PREVIEW_LIMIT }} 条历史记录</p>
              <Button
                data-more-clipboard-button
                size="sm"
                variant="ghost"
                @click="showClipboardHistoryModal = true"
              >
                更多
              </Button>
            </div>
          </div>

          <div v-if="recentSyncItems.length" class="grid gap-2">
            <div
              v-for="item in recentSyncItems"
              :key="item.id"
              class="flex items-start gap-3 rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 py-2"
            >
              <p class="line-clamp-2 min-w-0 flex-1 break-words text-sm leading-5 text-slate-300">
                {{ item.text }}
              </p>
              <CopyTextButton :text="item.text" icon-only label="复制内容" />
            </div>
          </div>
          <p v-else class="rounded-md border border-dashed border-[color:var(--main-line-soft)] px-3 py-4 text-sm text-slate-500">
            暂无同步内容，启动同步并复制文本后会显示在这里。
          </p>
        </div>
      </Card>
    </section>

    <Transition name="trust-prompt">
      <div
        v-if="showClipboardHistoryModal"
        data-clipboard-history-modal
        class="fixed inset-0 z-50 flex items-center justify-center bg-[color:var(--dialog-overlay-bg)] px-6 py-8 backdrop-blur-sm"
        @click.self="showClipboardHistoryModal = false"
      >
        <section
          class="flex max-h-full w-full max-w-3xl flex-col rounded-lg border border-[color:var(--main-line)] bg-[color:var(--dialog-bg)] shadow-[0_24px_80px_rgba(0,0,0,0.5)]"
          role="dialog"
          aria-modal="true"
          aria-label="全部剪贴内容"
        >
          <div class="flex items-start justify-between gap-4 border-b border-[color:var(--main-line-soft)] px-5 py-4">
            <div>
              <p class="text-base font-semibold text-white">全部剪贴内容</p>
              <p class="mt-1 text-xs text-[color:var(--muted-text)]">共 {{ allClipboardItems.length }} 条历史记录</p>
            </div>
            <button
              class="grid h-8 w-8 shrink-0 place-items-center rounded-md text-slate-300 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white"
              type="button"
              aria-label="关闭"
              title="关闭"
              @click="showClipboardHistoryModal = false"
            >
              <X class="h-4 w-4" />
            </button>
          </div>

          <div v-if="allClipboardItems.length" class="min-h-0 overflow-x-hidden overflow-y-auto p-5">
            <div class="grid gap-2">
              <div
                v-for="item in allClipboardItems"
                :key="item.id"
                data-clipboard-history-row
                class="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-3 rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 py-2.5"
              >
                <p data-clipboard-history-text class="min-w-0 whitespace-pre-wrap break-all text-sm leading-6 text-slate-200">
                  {{ item.text }}
                </p>
                <div data-clipboard-history-copy class="flex shrink-0 justify-end">
                  <CopyTextButton :text="item.text" icon-only label="复制内容" />
                </div>
              </div>
            </div>
          </div>
          <p v-else class="m-5 rounded-md border border-dashed border-[color:var(--main-line-soft)] px-3 py-8 text-center text-sm text-[color:var(--subtle-text)]">
            暂无剪贴内容。
          </p>
        </section>
      </div>
    </Transition>
  </div>
</template>
