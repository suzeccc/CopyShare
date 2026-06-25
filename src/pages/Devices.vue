<script setup lang="ts">
import { computed } from "vue";

import DeviceCard from "@/components/devices/DeviceCard.vue";
import ManualConnectForm from "@/components/devices/ManualConnectForm.vue";
import Card from "@/components/ui/Card.vue";
import RefreshButton from "@/components/ui/RefreshButton.vue";
import { useDevicesStore } from "@/stores/devices";

const devicesStore = useDevicesStore();
const recentIps = computed(() =>
  Array.from(
    new Set(
      devicesStore.history
        .map((device) => device.ip.trim())
        .filter(Boolean),
    ),
  ).slice(0, 8),
);
</script>

<template>
  <div class="grid gap-6">
    <section class="grid gap-5 xl:grid-cols-[0.85fr_1.15fr]">
      <Card>
        <p class="text-sm font-semibold text-white">快速配置</p>
        <p class="mt-2 text-sm leading-6 text-[color:var(--muted-text)]">
          在另一台电脑启动 CopyShare 并开启同步，输入它的局域网 IPv4 地址和端口。要双向同步，两台电脑都需要在设备列表里信任对方。
        </p>
        <div class="mt-5 rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] p-4">
          <ManualConnectForm
            :ip="devicesStore.connectDraft.ip"
            :port="devicesStore.connectDraft.port"
            :recent-ips="recentIps"
            :loading="devicesStore.loading"
            @update:ip="devicesStore.setConnectDraftIp"
            @update:port="devicesStore.setConnectDraftPort"
            @connect="devicesStore.connect"
          />
        </div>
        <p v-if="devicesStore.error" class="mt-4 rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-sm text-red-100">
          {{ devicesStore.error }}
        </p>
      </Card>

      <Card>
        <div class="flex items-start justify-between gap-4">
          <div>
            <p class="text-sm font-semibold text-white">已连接设备</p>
            <p class="mt-2 text-sm text-[color:var(--muted-text)]">已信任并保持连接的设备，只保留断开操作。</p>
          </div>
          <RefreshButton :refresh="() => devicesStore.refresh()" :failed="() => Boolean(devicesStore.error)" />
        </div>
        <div v-if="devicesStore.connected.length" class="mt-5 grid gap-3 md:grid-cols-2">
          <DeviceCard
            v-for="device in devicesStore.connected"
            :key="device.id"
            :device="device"
            mode="connected"
            @disconnect="devicesStore.disconnect"
            @reject="devicesStore.reject"
            @trust="devicesStore.trust"
          />
        </div>
        <div v-else class="mt-5 rounded-lg border border-dashed border-[color:var(--main-line-soft)] px-4 py-8 text-center text-sm text-[color:var(--subtle-text)]">
          还没有已信任的连接设备。先在设备列表确认信任。
        </div>
      </Card>
    </section>

    <Card>
      <div class="mb-4 flex items-center justify-between">
        <div>
          <p class="text-sm font-semibold text-white">历史连接设备列表</p>
          <p class="mt-1 text-xs text-[color:var(--muted-text)]">连接成功、等待确认和已断开的设备都会保留在这里。</p>
        </div>
      </div>
      <div v-if="devicesStore.history.length" class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        <DeviceCard
          v-for="device in devicesStore.history"
          :key="device.id"
          :device="device"
          mode="status"
          @disconnect="devicesStore.disconnect"
          @reconnect="devicesStore.connect"
          @reject="devicesStore.reject"
          @trust="devicesStore.trust"
        />
      </div>
      <div v-else class="rounded-lg border border-dashed border-[color:var(--main-line-soft)] px-4 py-10 text-center text-sm text-[color:var(--subtle-text)]">
        输入对方 IP 后点击“连接”，设备会作为历史记录显示在这里。
      </div>
    </Card>
  </div>
</template>
