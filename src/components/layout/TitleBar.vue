<script setup lang="ts">
import { Minus, RotateCw } from "lucide-vue-next";

import ConnectionBadge from "@/components/status/ConnectionBadge.vue";
import Button from "@/components/ui/Button.vue";
import { hideMainWindow } from "@/lib/tauri";
import { useStatusStore } from "@/stores/status";

const statusStore = useStatusStore();
</script>

<template>
  <header class="flex h-16 items-center justify-between border-b border-slate-800 bg-slate-950/55 px-6">
    <div class="min-w-0">
      <p class="text-xs text-slate-500">桌面同步控制台</p>
      <p class="truncate text-sm font-medium text-slate-200">
        {{ statusStore.status.deviceName }} · {{ statusStore.status.localIp || "等待网络地址" }}
      </p>
    </div>
    <div class="flex items-center gap-3">
      <ConnectionBadge :state="statusStore.status.state" :label="statusStore.statusLabel" />
      <Button variant="ghost" size="sm" @click="statusStore.refresh()">
        <RotateCw class="h-4 w-4" />
        刷新
      </Button>
      <Button variant="ghost" size="sm" @click="hideMainWindow()">
        <Minus class="h-4 w-4" />
        隐藏
      </Button>
    </div>
  </header>
</template>
