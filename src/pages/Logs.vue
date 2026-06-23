<script setup lang="ts">
import { Trash2 } from "lucide-vue-next";

import DeviceCard from "@/components/devices/DeviceCard.vue";
import HistoryItem from "@/components/history/HistoryItem.vue";
import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import RefreshButton from "@/components/ui/RefreshButton.vue";
import { useDevicesStore } from "@/stores/devices";
import { useHistoryStore } from "@/stores/history";

const devicesStore = useDevicesStore();
const historyStore = useHistoryStore();
</script>

<template>
  <div class="grid gap-5">
    <section class="grid gap-5 xl:grid-cols-[0.85fr_1.15fr]">
      <Card>
        <div class="flex flex-wrap items-start justify-between gap-4">
          <div>
            <p class="text-sm font-semibold text-white">连接状态</p>
            <p class="mt-2 text-sm leading-6 text-slate-400">
              集中查看当前会话里的连接设备，连接失败和断开状态会同步到设备列表。
            </p>
          </div>
          <RefreshButton :refresh="() => devicesStore.refresh()" />
        </div>

        <p v-if="devicesStore.error" class="mt-4 rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-sm text-red-100">
          {{ devicesStore.error }}
        </p>

        <div v-if="devicesStore.devices.length" class="mt-5 grid gap-3">
          <DeviceCard
            v-for="device in devicesStore.devices"
            :key="device.id"
            :device="device"
            @connect="devicesStore.connect"
            @disconnect="devicesStore.disconnect"
            @trust="devicesStore.trust"
          />
        </div>
        <div v-else class="mt-5 rounded-lg border border-dashed border-[color:var(--main-line-soft)] px-4 py-10 text-center text-sm text-slate-500">
          暂无连接设备。去“设备连接”输入对方 IP 后，这里会显示连接状态。
        </div>
      </Card>

      <Card>
        <div class="flex flex-wrap items-start justify-between gap-4">
          <div>
            <p class="text-sm font-semibold text-white">剪贴日志</p>
            <p class="mt-2 text-sm leading-6 text-slate-400">
              只保存内容摘要，用于确认同步方向、来源设备和同步时间。
            </p>
          </div>
          <Button variant="danger" :disabled="!historyStore.items.length || historyStore.loading" @click="historyStore.clear()">
            <Trash2 class="h-4 w-4" />
            清空日志
          </Button>
        </div>

        <p v-if="historyStore.error" class="mt-4 rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-sm text-red-100">
          {{ historyStore.error }}
        </p>

        <div v-if="historyStore.items.length" class="mt-5 grid gap-3">
          <HistoryItem v-for="item in historyStore.items" :key="item.id" :item="item" />
        </div>
        <div v-else class="mt-5 rounded-lg border border-dashed border-[color:var(--main-line-soft)] px-4 py-12 text-center text-sm text-slate-500">
          暂无剪贴日志。启动同步并复制文本后，这里会显示摘要。
        </div>
      </Card>
    </section>
  </div>
</template>
