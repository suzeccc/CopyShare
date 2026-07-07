<script setup lang="ts">
import { computed } from "vue";
import { Download, X } from "lucide-vue-next";

import Button from "@/components/ui/Button.vue";
import { formatTransferSize } from "@/lib/fileTransfer";
import { useFileTransferStore } from "@/stores/fileTransfer";

const fileTransferStore = useFileTransferStore();
const offer = computed(() => fileTransferStore.pendingOffer);
const totalSize = computed(() => offer.value?.totalSize ?? 0);

async function acceptOffer() {
  if (!offer.value) {
    return;
  }
  await fileTransferStore.acceptOffer(offer.value.transferId);
}

async function rejectOffer() {
  if (!offer.value) {
    return;
  }
  await fileTransferStore.rejectOffer(offer.value.transferId);
}
</script>

<template>
  <Transition name="trust-prompt">
    <div
      v-if="offer"
      data-file-transfer-offer-dialog
      class="absolute inset-0 z-[60] flex items-center justify-center bg-[color:var(--dialog-overlay-bg)] px-6 backdrop-blur-sm"
    >
      <section
        class="w-full max-w-[480px] rounded-lg border border-[color:var(--main-line)] bg-[color:var(--dialog-bg)] p-5 shadow-[0_20px_70px_rgba(0,0,0,0.48)]"
      >
        <div class="flex items-start gap-3">
          <div
            class="grid h-11 w-11 shrink-0 place-items-center rounded-lg border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]"
          >
            <Download class="h-5 w-5" />
          </div>
          <div class="min-w-0 flex-1">
            <p class="text-base font-semibold text-white">接收文件？</p>
            <p class="mt-1 text-sm leading-6 text-slate-300">
              {{ offer.peerDeviceName }} 想向你发送
              {{ offer.files.length }} 个文件，共 {{ formatTransferSize(totalSize) }}。
            </p>
          </div>
          <button
            type="button"
            class="grid h-8 w-8 shrink-0 place-items-center rounded-md text-slate-400 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white"
            aria-label="拒绝文件"
            @click="rejectOffer"
          >
            <X class="h-4 w-4" />
          </button>
        </div>

        <div
          class="mt-4 rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-soft)] px-3 py-3"
        >
          <p class="text-xs font-medium uppercase tracking-wide text-slate-500">
            文件列表
          </p>
          <ul class="mt-2 grid max-h-40 gap-1 overflow-auto">
            <li
              v-for="file in offer.files.slice(0, 8)"
              :key="file.id"
              class="flex justify-between gap-3 text-sm"
            >
              <span class="min-w-0 truncate text-white">{{ file.name }}</span>
              <span class="shrink-0 text-xs text-slate-400">
                {{ formatTransferSize(file.size) }}
              </span>
            </li>
            <li v-if="offer.files.length > 8" class="text-xs text-slate-400">
              还有 {{ offer.files.length - 8 }} 个文件
            </li>
          </ul>
        </div>

        <div class="mt-5 flex justify-end gap-3">
          <Button variant="danger" @click="rejectOffer">拒绝</Button>
          <Button variant="primary" @click="acceptOffer">接收</Button>
        </div>
      </section>
    </div>
  </Transition>
</template>
