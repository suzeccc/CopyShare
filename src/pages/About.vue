<script setup lang="ts">
import ExternalLink from "lucide-vue-next/dist/esm/icons/external-link.js";
import Github from "lucide-vue-next/dist/esm/icons/github.js";
import RefreshCw from "lucide-vue-next/dist/esm/icons/refresh-cw.js";
import UserRound from "lucide-vue-next/dist/esm/icons/user-round.js";
import Download from "lucide-vue-next/dist/esm/icons/download.js";

import appIconUrl from "../../src-tauri/icons/icon.ico?url";

import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import {
  APP_VERSION,
  AUTHOR_NAME,
  GITHUB_REPOSITORY_URL,
  UPDATE_URL,
} from "@/lib/about";
import { openExternalUrl } from "@/lib/tauri";
import { useUpdaterStore } from "@/stores/updater";

const repositoryName = "suzeccc/CopyShare";
const updater = useUpdaterStore();

async function openExternalLink(url: string) {
  try {
    await openExternalUrl(url);
  } catch (error) {
    updater.error = error instanceof Error ? error.message : "打开链接失败";
  }
}

async function openRepository() {
  await openExternalLink(GITHUB_REPOSITORY_URL);
}

async function openLatestRelease() {
  await openExternalLink(UPDATE_URL);
}

async function checkForUpdate() {
  await updater.checkForUpdate();
}
</script>

<template>
  <Card class="!p-0 overflow-hidden">
    <div class="px-5 py-6 sm:px-6">
      <div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
        <div class="flex min-w-0 items-start gap-4">
          <img :src="appIconUrl" alt="" class="h-12 w-12 shrink-0 object-contain" draggable="false">
          <div class="min-w-0">
            <h1 class="text-xl font-semibold tracking-tight text-white">关于 CopyShare</h1>
            <p class="mt-2 max-w-2xl text-[13px] leading-6 text-[color:var(--muted-text)]">
              局域网剪贴板同步工具，用于在已信任设备之间同步文本剪贴板内容
            </p>
          </div>
        </div>
        <Button
          class="shrink-0 self-start"
          variant="primary"
          size="sm"
          :disabled="updater.busy || updater.phase === 'ready' || updater.phase === 'installed'"
          :aria-busy="updater.phase === 'checking'"
          @click="checkForUpdate"
        >
          <RefreshCw class="h-4 w-4 motion-reduce:animate-none" :class="{ 'animate-spin': updater.phase === 'checking' }" aria-hidden="true" />
          {{ updater.phase === "checking" ? "检查中" : "检查更新" }}
        </Button>
      </div>

      <dl class="mt-5 flex flex-wrap items-center gap-x-8 gap-y-3 border-t border-[color:var(--main-line-soft)] pt-4 text-[13px]">
        <div class="flex items-center gap-3">
          <dt class="text-[color:var(--muted-text)]">版本信息</dt>
          <dd class="font-mono text-sm font-semibold text-white">v{{ APP_VERSION }}</dd>
        </div>
        <div class="flex items-center gap-3">
          <dt class="text-[color:var(--muted-text)]">作者</dt>
          <dd class="flex items-center gap-2 text-sm font-semibold text-white">
            <UserRound class="h-4 w-4 text-[color:var(--subtle-text)]" aria-hidden="true" />
            <span data-i18n-ignore>{{ AUTHOR_NAME }}</span>
          </dd>
        </div>
      </dl>

      <div
        v-if="updater.message || updater.error"
        role="status"
        aria-live="polite"
        class="mt-4 flex flex-wrap items-center justify-between gap-3 rounded-md border px-3 py-2"
        :class="{
          'border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]': !updater.error && updater.phase !== 'ready' && updater.phase !== 'idle',
          'border-emerald-400/40 bg-emerald-400/10 text-emerald-100': !updater.error && (updater.phase === 'ready' || updater.phase === 'idle'),
          'border-red-500/40 bg-red-500/10 text-red-100': !!updater.error,
        }"
      >
        <p class="min-w-0 flex-1 break-words text-[13px] leading-6">{{ updater.error || updater.message }}</p>
        <Button
          v-if="updater.phase === 'available'"
          data-update-download-button
          class="shrink-0"
          variant="primary"
          size="sm"
          @click="updater.downloadUpdate"
        >
          <Download class="h-4 w-4" aria-hidden="true" />
          {{ updater.error ? "重试下载" : "下载更新" }}
        </Button>
        <Button
          v-if="updater.phase === 'ready'"
          data-update-install-button
          class="shrink-0"
          variant="primary"
          size="sm"
          @click="updater.installUpdate"
        >
          安装并重启
        </Button>
        <Button v-if="updater.phase === 'installed'" size="sm" @click="updater.restartApp">
          重新启动
        </Button>
      </div>
      <div v-if="updater.phase === 'downloading'" class="mt-3 grid gap-2" data-update-progress>
        <progress :value="updater.progress ?? undefined" max="100" aria-label="下载更新" class="h-2 w-full [accent-color:var(--accent-text)]" />
        <p class="text-xs text-[color:var(--muted-text)]" aria-live="polite">
          {{ updater.progress === null ? "正在下载更新..." : `${updater.progress}%` }}
          · {{ (updater.downloadedBytes / 1048576).toFixed(1) }} MiB
          <span v-if="updater.totalBytes"> / {{ (updater.totalBytes / 1048576).toFixed(1) }} MiB</span>
        </p>
      </div>
      <div v-if="updater.notes" class="mt-4" data-update-notes>
        <p class="text-xs font-medium text-[color:var(--muted-text)]">更新说明</p>
        <p data-i18n-ignore class="mt-2 max-h-32 overflow-auto whitespace-pre-wrap break-words text-[13px] leading-6 text-[color:var(--muted-text)]">{{ updater.notes }}</p>
      </div>
    </div>

    <div class="border-t border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] px-5 py-5 sm:px-6">
      <p class="text-xs font-medium text-[color:var(--muted-text)]">GitHub 仓库</p>
      <div class="mt-3 flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
        <div class="flex min-w-0 items-start gap-3">
          <Github class="mt-0.5 h-5 w-5 shrink-0 text-[color:var(--subtle-text)]" aria-hidden="true" />
          <div class="min-w-0">
            <p class="break-words text-sm font-semibold text-white" data-i18n-ignore>{{ repositoryName }}</p>
            <p class="mt-1 break-all font-mono text-xs leading-5 text-[color:var(--muted-text)]">
              <span data-i18n-ignore>{{ GITHUB_REPOSITORY_URL }}</span>
            </p>
          </div>
        </div>
        <div class="flex shrink-0 flex-wrap gap-2">
          <Button variant="secondary" size="sm" @click="openRepository">
            <ExternalLink class="h-4 w-4" aria-hidden="true" />
            打开仓库
          </Button>
          <Button variant="primary" size="sm" @click="openLatestRelease">
            <RefreshCw class="h-4 w-4" aria-hidden="true" />
            查看最新版本
          </Button>
        </div>
      </div>
      <p data-github-star-hint class="mt-4 text-xs leading-5 text-[color:var(--muted-text)]">
        如果这个项目帮到了你，欢迎在 GitHub 仓库点一颗 Star
      </p>
    </div>
  </Card>
</template>
