<script setup lang="ts">
import { computed } from "vue";
import { ShieldQuestion, ShieldX, Unplug } from "lucide-vue-next";

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

    if (props.device.trusted) {
      return {
        label: "已连接",
        detail: "连接正常，剪贴板状态会实时更新。",
        badgeClass: "border-emerald-400/45 bg-emerald-400/10 text-emerald-100",
        dotClass: "bg-emerald-300 shadow-[0_0_14px_rgba(110,231,183,0.65)]",
        cardClass: "border-emerald-400/30 bg-[rgba(20,54,72,0.58)]",
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
      label: "已信任同步中",
      detail: "本机已信任此设备；若仍只能单向同步，请在对方电脑也信任本机。",
      badgeClass: "border-emerald-400/45 bg-emerald-400/10 text-emerald-100",
      dotClass: "bg-emerald-300 shadow-[0_0_14px_rgba(110,231,183,0.65)]",
      cardClass: "border-emerald-400/30 bg-[rgba(20,54,72,0.58)]",
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

    <div v-if="showActions" class="mt-4 flex flex-wrap gap-2">
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
      <Button v-else size="sm" variant="ghost" @click="$emit('disconnect', device.id)">
        <Unplug class="h-4 w-4" />
        断开
      </Button>
    </div>
  </article>
</template>
