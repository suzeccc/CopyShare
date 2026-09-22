<script setup lang="ts">
import CircleAlert from "lucide-vue-next/dist/esm/icons/circle-alert.js";
import CircleCheck from "lucide-vue-next/dist/esm/icons/circle-check.js";
import Info from "lucide-vue-next/dist/esm/icons/info.js";
import { computed } from "vue";

import { useToastStore } from "@/stores/toasts";
import type { ToastKind } from "@/lib/toasts";
import { translateSource } from "@/i18n";

const toastStore = useToastStore();
const toast = computed(() => toastStore.items[0]);
const toastMessage = computed(() => toast.value ? translateSource(toast.value.message) : "");

function iconFor(kind: ToastKind) {
  if (kind === "success") return CircleCheck;
  if (kind === "error") return CircleAlert;
  return Info;
}

function iconClass(kind: ToastKind) {
  if (kind === "success") return "text-emerald-400";
  if (kind === "error") return "text-red-300";
  return "text-sky-300";
}
</script>

<template>
  <Teleport to="body">
    <div data-toast-stack class="pointer-events-none fixed left-1/2 top-14 z-[120] -translate-x-1/2">
      <Transition name="toast" mode="out-in">
      <div
        v-if="toast"
        :key="toast.id"
        data-toast-item
        class="pointer-events-auto inline-flex min-h-12 max-w-[min(420px,calc(100vw-3rem))] items-center gap-2.5 rounded-[14px] border border-white/10 bg-[rgba(72,75,82,0.92)] px-5 py-3 text-[15px] font-medium text-white shadow-[0_18px_42px_rgba(0,0,0,0.38)] backdrop-blur-md"
      >
        <component :is="iconFor(toast.kind)" class="h-5 w-5 shrink-0" :class="iconClass(toast.kind)" />
        <span class="min-w-0 truncate">{{ toastMessage }}</span>
      </div>
      </Transition>
    </div>
  </Teleport>
</template>
