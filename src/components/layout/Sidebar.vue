<script setup lang="ts">
import { History, Home, MonitorSmartphone, Settings } from "lucide-vue-next";

const items = [
  { label: "总览", path: "/", icon: Home },
  { label: "设备连接", path: "/devices", icon: MonitorSmartphone },
  { label: "同步历史", path: "/history", icon: History },
  { label: "设置", path: "/settings", icon: Settings },
];
</script>

<template>
  <aside class="flex h-full w-64 shrink-0 flex-col border-r border-slate-800 bg-slate-950/72 px-4 py-5">
    <div class="mb-7">
      <p class="text-xs font-medium uppercase text-slate-500">Copy-Sharer</p>
      <h1 class="mt-2 text-xl font-semibold text-white">剪贴板共享</h1>
    </div>

    <nav class="space-y-1">
      <RouterLink
        v-for="item in items"
        :key="item.path"
        :to="item.path"
        class="flex h-10 items-center gap-3 rounded-md px-3 text-sm font-medium text-slate-400 transition hover:bg-slate-900 hover:text-white"
        active-class="bg-blue-600/16 text-blue-100 ring-1 ring-blue-500/35"
      >
        <component :is="item.icon" class="h-4 w-4" />
        {{ item.label }}
      </RouterLink>
    </nav>

    <div class="mt-auto rounded-lg border border-slate-800 bg-slate-900/70 p-3 text-xs text-slate-400">
      <p class="font-medium text-slate-200">局域网直连</p>
      <p class="mt-1 leading-5">默认端口 8765，只同步已信任设备的文本剪贴板。</p>
    </div>
  </aside>
</template>
