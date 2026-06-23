<script setup lang="ts">
import { computed, onBeforeUnmount, reactive, watch } from "vue";

import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import Switch from "@/components/ui/Switch.vue";
import { clampPort } from "@/lib/format";
import { useConfigStore } from "@/stores/config";
import type { AppTheme } from "@/types/config";

const configStore = useConfigStore();

const draft = reactive({ ...configStore.config });
const themeOptions: Array<{ value: AppTheme; label: string; hint: string }> = [
  { value: "win11Dark", label: "Win11 深色", hint: "深灰卡片与系统设置风格" },
  { value: "copyBlue", label: "经典蓝", hint: "当前深蓝控制台风格" },
];

function applyThemePreview(theme: AppTheme) {
  document.documentElement.dataset.appTheme = theme;
  document.body.dataset.appTheme = theme;
}

watch(
  () => configStore.config,
  (next) => {
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

const canSave = computed(() => draft.deviceName.trim().length > 0);

function save() {
  configStore.save({
    ...draft,
    deviceName: draft.deviceName.trim(),
    port: clampPort(draft.port),
    syncText: true,
    syncFiles: false,
  });
}
</script>

<template>
  <div class="grid gap-6 xl:grid-cols-[1fr_0.85fr]">
    <Card>
      <p class="text-sm font-semibold text-white">基础设置</p>
      <div class="mt-5 grid gap-4">
        <label>
          <span class="mb-2 block text-xs font-medium text-slate-400">设备名称</span>
          <input
            v-model="draft.deviceName"
            class="h-10 w-full rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 text-sm text-white"
          />
        </label>
        <label>
          <span class="mb-2 block text-xs font-medium text-slate-400">监听端口</span>
          <input
            v-model.number="draft.port"
            class="h-10 w-full rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] px-3 text-sm text-white"
            type="number"
            min="1"
            max="65535"
          />
        </label>
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
        <Switch v-model="draft.autoStart" label="开机自启" hint="系统登录后自动启动 Copy-Sharer。" />
        <Switch v-model="draft.autoSync" label="启动后自动同步" hint="启动应用后自动开始监听剪贴板。" />
        <Switch v-model="draft.saveHistory" label="保存同步摘要" hint="只保存摘要，不保存完整敏感剪贴板内容。" />
      </div>
      <p v-if="configStore.error" class="mt-4 rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-sm text-red-100">
        {{ configStore.error }}
      </p>
      <div class="mt-5">
        <Button variant="primary" :disabled="!canSave || configStore.saving" @click="save">
          保存设置
        </Button>
      </div>
    </Card>

    <Card>
      <p class="text-sm font-semibold text-white">同步内容</p>
      <div class="mt-5 grid gap-3">
        <Switch v-model="draft.syncText" label="同步文本" hint="MVP 默认开启，只同步文本剪贴板。" disabled />
        <Switch v-model="draft.syncImage" label="同步图片" hint="支持截图和图片复制。" />
        <Switch v-model="draft.syncFiles" label="同步文件" hint="后续支持" disabled />
      </div>
      <div class="mt-6 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] p-4">
        <p class="text-xs font-medium text-slate-400">已信任设备</p>
        <p class="mt-2 text-sm text-slate-300">
          {{ draft.trustedDevices.length ? `${draft.trustedDevices.length} 台` : "暂无" }}
        </p>
      </div>
    </Card>
  </div>
</template>
