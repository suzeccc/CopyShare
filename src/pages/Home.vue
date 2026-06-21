<script setup lang="ts">
import { Clock, Monitor, Network, Settings } from "lucide-vue-next";
import { computed } from "vue";
import { RouterLink } from "vue-router";

import StatusCard from "@/components/status/StatusCard.vue";
import SyncSwitch from "@/components/status/SyncSwitch.vue";
import Card from "@/components/ui/Card.vue";
import Button from "@/components/ui/Button.vue";
import HistoryItem from "@/components/history/HistoryItem.vue";
import { formatTime } from "@/lib/format";
import { useHistoryStore } from "@/stores/history";
import { useStatusStore } from "@/stores/status";

const statusStore = useStatusStore();
const historyStore = useHistoryStore();

const address = computed(() => {
  const ip = statusStore.status.localIp || "未获取";
  return `${ip}:${statusStore.status.port}`;
});

const latestItems = computed(() => historyStore.items.slice(0, 3));
</script>

<template>
  <div class="grid gap-6">
    <section class="grid gap-5 lg:grid-cols-[1.25fr_0.75fr]">
      <Card>
        <div class="flex flex-wrap items-start justify-between gap-5">
          <div>
            <p class="text-xs font-medium text-blue-200">局域网剪贴板同步</p>
            <h2 class="mt-2 text-2xl font-semibold text-white">Copy-Sharer</h2>
            <p class="mt-3 max-w-2xl text-sm leading-6 text-slate-400">
              监听本机文本剪贴板，通过 WebSocket 同步给已信任的局域网设备。
              MVP 阶段默认只开放文本同步，图片和文件保留为后续能力。
            </p>
          </div>
          <SyncSwitch
            :running="statusStore.status.running"
            :loading="statusStore.loading"
            @start="statusStore.start()"
            @stop="statusStore.stop()"
          />
        </div>
        <p v-if="statusStore.error" class="mt-4 rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-sm text-red-100">
          {{ statusStore.error }}
        </p>
      </Card>

      <Card>
        <p class="text-sm font-medium text-white">快速操作</p>
        <div class="mt-4 grid gap-2">
          <RouterLink to="/devices">
            <Button class="w-full justify-start" variant="secondary">
              <Network class="h-4 w-4" />
              手动连接设备
            </Button>
          </RouterLink>
          <RouterLink to="/settings">
            <Button class="w-full justify-start" variant="ghost">
              <Settings class="h-4 w-4" />
              打开设置
            </Button>
          </RouterLink>
        </div>
      </Card>
    </section>

    <section class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      <StatusCard label="同步状态" :value="statusStore.statusLabel" :hint="statusStore.status.message || undefined" />
      <StatusCard label="已连接设备" :value="`${statusStore.status.connectedCount} 台`" hint="仅统计当前会话连接" />
      <StatusCard label="本机地址" :value="address" hint="同一局域网设备可连接此地址" />
      <StatusCard label="最近同步" :value="formatTime(statusStore.status.lastSyncAt)" hint="历史只保存内容摘要" />
    </section>

    <section class="grid gap-5 lg:grid-cols-[0.8fr_1.2fr]">
      <Card>
        <div class="flex items-center gap-3">
          <Monitor class="h-5 w-5 text-blue-300" />
          <div>
            <p class="text-sm font-medium text-white">{{ statusStore.status.deviceName }}</p>
            <p class="mt-1 font-mono text-xs text-slate-500">{{ statusStore.status.deviceId }}</p>
          </div>
        </div>
        <div class="mt-5 grid gap-3 text-sm text-slate-300">
          <p class="flex justify-between gap-4">
            <span class="text-slate-500">监听端口</span>
            <span class="font-mono">{{ statusStore.status.port }}</span>
          </p>
          <p class="flex justify-between gap-4">
            <span class="text-slate-500">运行状态</span>
            <span>{{ statusStore.status.running ? "后台任务运行中" : "等待启动" }}</span>
          </p>
        </div>
      </Card>

      <Card>
        <div class="mb-4 flex items-center justify-between gap-4">
          <div>
            <p class="text-sm font-medium text-white">连接和剪贴日志</p>
            <p class="mt-1 text-xs text-slate-500">只显示摘要，不保存完整敏感内容。</p>
          </div>
          <Clock class="h-5 w-5 text-slate-500" />
        </div>
        <div v-if="latestItems.length" class="grid gap-3">
          <HistoryItem v-for="item in latestItems" :key="item.id" :item="item" />
        </div>
        <div v-else class="rounded-lg border border-dashed border-slate-700 px-4 py-8 text-center text-sm text-slate-500">
          启动同步后，连接和剪贴记录会出现在这里。
        </div>
      </Card>
    </section>
  </div>
</template>
