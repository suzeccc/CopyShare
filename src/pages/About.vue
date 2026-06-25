<script setup lang="ts">
import { ExternalLink, Github, RefreshCw, UserRound } from "lucide-vue-next";
import { ref } from "vue";

import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import {
  APP_VERSION,
  AUTHOR_NAME,
  GITHUB_REPOSITORY_URL,
  getLatestRelease,
  getUpdateState,
  UPDATE_URL,
} from "@/lib/about";
import { openExternalUrl } from "@/lib/tauri";

const repositoryName = "suzeccc/Copy-share";
const checkingUpdate = ref(false);
const updateMessage = ref<string | null>(null);
const updateTone = ref<"info" | "success" | "error">("info");
const updateReleaseUrl = ref<string | null>(null);

async function openExternalLink(url: string) {
  try {
    await openExternalUrl(url);
  } catch (error) {
    updateTone.value = "error";
    updateMessage.value = error instanceof Error ? error.message : "打开链接失败";
  }
}

async function openRepository() {
  await openExternalLink(GITHUB_REPOSITORY_URL);
}

async function openLatestRelease() {
  await openExternalLink(UPDATE_URL);
}

async function openUpdateRelease() {
  if (updateReleaseUrl.value) {
    await openExternalLink(updateReleaseUrl.value);
  }
}

async function checkForUpdate() {
  checkingUpdate.value = true;
  updateTone.value = "info";
  updateMessage.value = "正在检查最新版本...";
  updateReleaseUrl.value = null;

  try {
    const latestRelease = await getLatestRelease();
    const update = getUpdateState(APP_VERSION, latestRelease);

    if (!update.hasUpdate) {
      updateTone.value = "success";
      updateMessage.value = `已是最新版本 v${APP_VERSION}`;
      return;
    }

    updateTone.value = "info";
    updateMessage.value = `发现新版本 v${update.latestVersion}，请点击下方按钮打开发布页。`;
    updateReleaseUrl.value = update.updateUrl;
  } catch (error) {
    updateTone.value = "error";
    updateMessage.value = error instanceof Error ? error.message : "检查更新失败";
    updateReleaseUrl.value = null;
  } finally {
    checkingUpdate.value = false;
  }
}
</script>

<template>
  <div class="grid gap-5 xl:grid-cols-[1fr_0.85fr]">
    <Card>
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <p class="text-sm font-semibold text-white">关于 CopyShare</p>
          <p class="mt-2 max-w-2xl text-sm leading-6 text-[color:var(--muted-text)]">
            局域网剪贴板同步工具，用于在已信任设备之间同步文本剪贴板内容。
          </p>
        </div>
        <Button variant="primary" :disabled="checkingUpdate" @click="checkForUpdate">
          <RefreshCw class="h-4 w-4" />
          {{ checkingUpdate ? "检查中" : "检查更新" }}
        </Button>
      </div>

      <div class="mt-5 grid gap-3 sm:grid-cols-2">
        <div class="rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--stat-bg)] px-4 py-3">
          <p class="text-xs text-[color:var(--muted-text)]">版本信息</p>
          <p class="mt-1.5 font-mono text-xl font-semibold text-white">v{{ APP_VERSION }}</p>
        </div>
        <div class="rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--stat-bg)] px-4 py-3">
          <p class="text-xs text-[color:var(--muted-text)]">作者</p>
          <p class="mt-1.5 flex items-center gap-2 text-xl font-semibold text-white">
            <UserRound class="h-5 w-5 text-[color:var(--accent-text)]" />
            {{ AUTHOR_NAME }}
          </p>
        </div>
      </div>

      <p
        v-if="updateMessage"
        class="mt-4 rounded-md border px-3 py-2 text-sm"
        :class="{
          'border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]': updateTone === 'info',
          'border-emerald-400/40 bg-emerald-400/10 text-emerald-100': updateTone === 'success',
          'border-red-500/40 bg-red-500/10 text-red-100': updateTone === 'error',
        }"
      >
        {{ updateMessage }}
      </p>

      <Button
        v-if="updateReleaseUrl"
        data-update-release-link
        class="mt-3"
        variant="secondary"
        @click="openUpdateRelease"
      >
        <ExternalLink class="h-4 w-4" />
        打开发布页
      </Button>
    </Card>

    <Card>
      <p class="text-sm font-semibold text-white">GitHub 仓库</p>
      <div class="mt-5 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] p-4">
        <div class="flex items-start gap-3">
          <div class="grid h-10 w-10 shrink-0 place-items-center rounded-md border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]">
            <Github class="h-5 w-5" />
          </div>
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-semibold text-white">{{ repositoryName }}</p>
            <p class="mt-1 break-all font-mono text-xs text-[color:var(--muted-text)]">
              {{ GITHUB_REPOSITORY_URL }}
            </p>
          </div>
        </div>
        <div class="mt-4 flex flex-wrap items-center justify-between gap-3">
          <div class="flex flex-wrap gap-2">
            <Button variant="secondary" @click="openRepository">
              <ExternalLink class="h-4 w-4" />
              打开仓库
            </Button>
            <Button variant="ghost" @click="openLatestRelease">
              <RefreshCw class="h-4 w-4" />
              查看最新版本
            </Button>
          </div>
          <p data-github-star-hint class="max-w-sm text-right text-xs leading-5 text-[color:var(--muted-text)]">
            如果这个项目帮到了你，欢迎在 GitHub 仓库点一颗 Star。
          </p>
        </div>
      </div>
    </Card>
  </div>
</template>
