<script setup lang="ts">
import DeviceCard from "@/components/devices/DeviceCard.vue";
import ManualConnectForm from "@/components/devices/ManualConnectForm.vue";
import Card from "@/components/ui/Card.vue";
import RefreshButton from "@/components/ui/RefreshButton.vue";
import { useConfigStore } from "@/stores/config";
import { useDevicesStore } from "@/stores/devices";

const devicesStore = useDevicesStore();
const configStore = useConfigStore();
</script>

<template>
  <div class="grid gap-6">
    <section class="grid gap-5 xl:grid-cols-[0.85fr_1.15fr]">
      <Card>
        <p class="text-sm font-semibold text-white">快速配对</p>
        <p class="mt-2 text-sm leading-6 text-slate-400">
          在另一台电脑启动 Copy-Sharer，输入它的局域网 IPv4 地址和端口。
          首次连接会进入待信任状态，确认后才会参与同步。
        </p>
        <div class="mt-5 rounded-lg border border-[color:var(--main-line-soft)] bg-[rgba(19,34,63,0.58)] p-4">
          <ManualConnectForm
            :loading="devicesStore.loading"
            :default-port="configStore.config.port"
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
            <p class="mt-2 text-sm text-slate-400">当前会话里保持 WebSocket 连接的设备。</p>
          </div>
          <RefreshButton :refresh="() => devicesStore.refresh()" />
        </div>
        <div v-if="devicesStore.connected.length" class="mt-5 grid gap-3 md:grid-cols-2">
          <DeviceCard
            v-for="device in devicesStore.connected"
            :key="device.id"
            :device="device"
            @disconnect="devicesStore.disconnect"
            @trust="devicesStore.trust"
          />
        </div>
        <div v-else class="mt-5 rounded-lg border border-dashed border-[color:var(--main-line-soft)] px-4 py-8 text-center text-sm text-slate-500">
          还没有连接设备。输入对方 IP 后点击“连接”。
        </div>
      </Card>
    </section>

    <Card>
      <div class="mb-4 flex items-center justify-between">
        <div>
          <p class="text-sm font-semibold text-white">设备列表</p>
          <p class="mt-1 text-xs text-slate-500">包含已发现、已连接和已信任设备。</p>
        </div>
      </div>
      <div v-if="devicesStore.devices.length" class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        <DeviceCard
          v-for="device in devicesStore.devices"
          :key="device.id"
          :device="device"
          @disconnect="devicesStore.disconnect"
          @trust="devicesStore.trust"
        />
      </div>
      <div v-else class="rounded-lg border border-dashed border-[color:var(--main-line-soft)] px-4 py-10 text-center text-sm text-slate-500">
        输入对方 IP 后点击“连接”，设备会出现在这里。
      </div>
    </Card>
  </div>
</template>
