<script setup lang="ts">
import {
  Clipboard,
  History,
  Home,
  Info,
  Languages,
  MonitorSmartphone,
  ScanText,
  Settings,
} from "lucide-vue-next";

const primaryItems = [
  { label: "总览", path: "/", icon: Home },
  { label: "剪切板", path: "/clipboard", icon: Clipboard },
  { label: "设备连接", path: "/devices", icon: MonitorSmartphone },
  { label: "图片转文字", path: "/ocr", icon: ScanText },
  { label: "翻译", path: "/translate", icon: Languages },
  { label: "连接与剪贴日志", path: "/logs", icon: History },
];

const footerItems = [
  { label: "设置", path: "/settings", icon: Settings },
  { label: "关于", path: "/about", icon: Info },
];
</script>

<template>
  <aside class="flex h-full w-48 shrink-0 flex-col border-r border-[color:var(--main-line)] bg-[color:var(--main-bg)] px-2 py-5">
    <div class="mb-6">
      <p class="text-xs font-medium uppercase text-slate-500">CopyShare</p>
      <h1 class="mt-2 text-xl font-semibold text-white">剪贴板共享</h1>
    </div>

    <nav class="space-y-1">
      <RouterLink
        v-for="item in primaryItems"
        :key="item.path"
        :to="item.path"
        class="flex h-10 items-center gap-2.5 rounded-md px-2 text-sm font-medium text-slate-400 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white"
        active-class="bg-[color:var(--main-bg-soft)] text-[color:var(--accent-text)] ring-1 ring-[color:var(--main-line)]"
      >
        <component :is="item.icon" class="h-4 w-4" />
        {{ item.label }}
      </RouterLink>
    </nav>

    <nav data-sidebar-footer-nav class="mt-auto space-y-1">
      <RouterLink
        v-for="item in footerItems"
        :key="item.path"
        :to="item.path"
        class="flex h-10 items-center gap-2.5 rounded-md px-2 text-sm font-medium text-slate-400 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white"
        active-class="bg-[color:var(--main-bg-soft)] text-white ring-1 ring-[color:var(--main-line)]"
      >
        <component :is="item.icon" class="h-4 w-4" />
        {{ item.label }}
      </RouterLink>
    </nav>
  </aside>
</template>
