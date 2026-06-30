<script setup lang="ts">
import { computed } from "vue";
import { RefreshCw, ShieldQuestion, ShieldX, Unplug } from "lucide-vue-next";

import Button from "@/components/ui/Button.vue";
import { deviceAddress, formatTime } from "@/lib/format";
import type { DeviceInfo } from "@/types/device";

const props = withDefaults(
  defineProps<{
    device: DeviceInfo;
    mode?: "pending" | "connected" | "status";
    showActions?: boolean;
  }>(),
  {
    mode: "pending",
    showActions: true,
  },
);

defineEmits<{
  disconnect: [deviceId: string];
  reject: [deviceId: string];
  reconnect: [ip: string, port: number];
  trust: [deviceId: string];
}>();

const status = computed(() => {
  if (props.mode === "status") {
    if (!props.device.connected) {
      return {
        label: "已离线",
        detail: "设备已断开连接，等待重新连接。",
        badgeClass: "border-white/35 bg-white/[0.08] text-white",
        dotClass: "bg-white shadow-[0_0_12px_rgba(255,255,255,0.45)]",
        cardClass: "border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)]",
      };
    }

    if (props.device.trusted && props.device.remoteTrusted) {
      return {
        label: "已连接",
        detail: "连接正常，剪贴板状态会实时更新。",
        badgeClass: "border-emerald-400/45 bg-emerald-400/10 text-emerald-100",
        dotClass: "bg-emerald-300 shadow-[0_0_14px_rgba(110,231,183,0.65)]",
        cardClass: "border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)]",
      };
    }

    if (props.device.trusted) {
      return {
        label: "等待对方信任",
        detail: "本机已信任此设备，等待对方也信任本机后开始双向同步。",
        badgeClass: "border-amber-300/35 bg-amber-400/10 text-amber-100",
        dotClass: "bg-amber-300 shadow-[0_0_14px_rgba(252,211,77,0.45)]",
        cardClass: "border-amber-300/25 bg-[color:var(--panel-bg-soft)]",
      };
    }

    if (props.device.remoteTrusted) {
      return {
        label: "对方已信任，等待本机确认",
        detail: "对方已经信任本机，点击信任设备后即可双向同步。",
        badgeClass: "border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]",
        dotClass: "bg-[color:var(--theme-accent)] shadow-[0_0_14px_var(--accent-glow)]",
        cardClass: "border-[color:var(--accent-line)] bg-[color:var(--panel-bg-soft)]",
      };
    }

    return {
      label: "等待确认",
      detail: "连接已建立，等待同步权限确认。",
      badgeClass: "border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]",
      dotClass: "bg-[color:var(--theme-accent)] shadow-[0_0_14px_var(--accent-glow)]",
      cardClass: "border-[color:var(--accent-line)] bg-[color:var(--panel-bg-soft)]",
    };
  }

  if (props.mode === "connected") {
    return {
      label: "已连接",
      detail: "双方设备已互相信任，剪贴板状态会实时更新。",
      badgeClass: "border-white/15 bg-white/[0.07] text-slate-100",
      dotClass: "bg-[#8fd6a8] shadow-[0_0_10px_rgba(143,214,168,0.34)]",
      cardClass: "border-[color:var(--main-line-soft)] bg-[#2a2a2a] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]",
    };
  }

  return {
    label: "等待确认",
    detail: "连接已建立。要双向同步，两台电脑都需要信任对方。",
    badgeClass: "border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]",
    dotClass: "bg-[color:var(--theme-accent)] shadow-[0_0_14px_var(--accent-glow)]",
    cardClass: "border-[color:var(--accent-line)] bg-[color:var(--panel-bg-soft)]",
  };
});

const showActionButtons = computed(() => {
  if (!props.showActions) return false;
  if (props.mode === "pending") return true;
  if (props.mode === "connected") return true;
  return props.mode === "status" && !props.device.connected;
});
</script>

<template>
  <article
    class="rounded-lg border p-4 transition duration-200 hover:border-[color:var(--main-line)]"
    :class="status.cardClass"
  >
    <div class="flex items-start justify-between gap-4">
      <div class="min-w-0">
        <h3 class="truncate text-sm font-semibold text-white">{{ device.name }}</h3>
        <p class="mt-1 font-mono text-xs text-slate-400">
          {{ deviceAddress(device.ip, device.port) }}
        </p>
      </div>
      <span
        class="inline-flex h-8 shrink-0 items-center gap-2 rounded-md border px-2.5 text-xs font-medium"
        :class="status.badgeClass"
      >
        <span class="h-2 w-2 rounded-full" :class="status.dotClass" />
        {{ status.label }}
      </span>
    </div>

    <div class="mt-4 grid gap-2 text-xs leading-5 text-slate-400">
      <p class="text-slate-300">{{ status.detail }}</p>
      <p>最后在线：{{ formatTime(device.lastSeenAt) }}</p>
    </div>

    <div v-if="showActionButtons" class="mt-4 flex flex-wrap gap-2">
      <template v-if="mode === 'pending'">
        <Button size="sm" variant="secondary" @click="$emit('trust', device.id)">
          <ShieldQuestion class="h-4 w-4" />
          信任设备
        </Button>
        <Button size="sm" variant="danger" @click="$emit('reject', device.id)">
          <ShieldX class="h-4 w-4" />
          不信任设备
        </Button>
      </template>
      <Button v-else-if="mode === 'status' && !device.connected" size="sm" variant="secondary" @click="$emit('reconnect', device.ip, device.port)">
        <RefreshCw class="h-4 w-4" />
        重新连接
      </Button>
      <Button v-else-if="mode === 'connected'" size="sm" variant="ghost" @click="$emit('disconnect', device.id)">
        <Unplug class="h-4 w-4" />
        断开
      </Button>
    </div>
  </article>
</template>
