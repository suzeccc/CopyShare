<script setup lang="ts">
import { Minus, Square, X } from "lucide-vue-next";
import { ref } from "vue";

import Button from "@/components/ui/Button.vue";
import appIconUrl from "../../../src-tauri/icons/icon.ico?url";
import {
  closeWindow,
  hideMainWindow,
  minimizeWindow,
  startWindowDrag,
  toggleMaximizeWindow,
} from "@/lib/tauri";
import { startWindowDragFromMouseEvent } from "@/lib/windowDrag";
import { useConfigStore } from "@/stores/config";
import type { CloseAction } from "@/types/config";

type SavedCloseAction = Exclude<CloseAction, "ask">;

const configStore = useConfigStore();
const showCloseActionDialog = ref(false);
const rememberCloseAction = ref(false);
const closeActionSaving = ref(false);

function handleWindowDrag(event: MouseEvent) {
  startWindowDragFromMouseEvent(event, startWindowDrag);
}

async function runCloseAction(action: SavedCloseAction) {
  if (action === "minimize") {
    await hideMainWindow();
    return;
  }

  await closeWindow();
}

async function saveCloseActionPreference(action: SavedCloseAction) {
  closeActionSaving.value = true;
  try {
    await configStore.save({
      ...configStore.config,
      closeAction: action,
    });
  } finally {
    closeActionSaving.value = false;
  }
}

async function chooseCloseAction(action: SavedCloseAction) {
  if (closeActionSaving.value) {
    return;
  }

  if (rememberCloseAction.value) {
    await saveCloseActionPreference(action);
  }

  showCloseActionDialog.value = false;
  await runCloseAction(action);
}

async function handleCloseWindow() {
  const closeAction = configStore.config.closeAction ?? "ask";

  if (closeAction === "ask") {
    rememberCloseAction.value = false;
    showCloseActionDialog.value = true;
    return;
  }

  await runCloseAction(closeAction);
}
</script>

<template>
  <header
    class="flex h-10 shrink-0 select-none items-center border-b border-[color:var(--main-line)] bg-[color:var(--main-bg-deep)] text-slate-100"
    data-window-drag-region
    @dblclick="toggleMaximizeWindow()"
    @mousedown.capture="handleWindowDrag"
  >
    <div
      class="flex min-w-0 items-center gap-2 px-4"
      data-window-drag-region
    >
      <img
        :src="appIconUrl"
        alt=""
        class="h-5 w-5 shrink-0 rounded-sm"
        draggable="false"
        data-window-drag-region
      >
      <span
        class="truncate text-sm font-medium text-white"
        data-window-drag-region
      >
        CopyShare
      </span>
    </div>

    <div
      class="h-full flex-1"
      aria-hidden="true"
      data-window-drag-region
    />

    <div class="flex h-full items-center" data-window-control @dblclick.stop>
      <button
        class="grid h-full w-12 place-items-center text-slate-300 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white"
        type="button"
        aria-label="最小化"
        title="最小化"
        data-window-control
        @click="minimizeWindow()"
      >
        <Minus class="h-4 w-4" />
      </button>
      <button
        class="grid h-full w-12 place-items-center text-slate-300 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white"
        type="button"
        aria-label="最大化"
        title="最大化"
        data-window-control
        @click="toggleMaximizeWindow()"
      >
        <Square class="h-3.5 w-3.5" />
      </button>
      <button
        class="grid h-full w-12 place-items-center text-slate-300 transition hover:bg-red-500 hover:text-white"
        type="button"
        aria-label="关闭"
        title="关闭"
        data-window-control
        @click="handleCloseWindow"
      >
        <X class="h-4 w-4" />
      </button>
    </div>
  </header>

  <Teleport to="body">
    <Transition name="trust-prompt">
      <div
        v-if="showCloseActionDialog"
        data-close-action-dialog
        class="fixed inset-0 z-[80] flex items-center justify-center bg-[color:var(--dialog-overlay-bg)] px-6 backdrop-blur-sm"
      >
        <section class="w-full max-w-[430px] rounded-lg border border-[color:var(--main-line)] bg-[color:var(--dialog-bg)] p-5 text-slate-100 shadow-[0_20px_70px_rgba(0,0,0,0.52)]">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-base font-semibold text-white">关闭 CopyShare？</p>
              <p class="mt-2 text-sm leading-6 text-slate-300">
                可以最小化到托盘继续同步，也可以直接退出应用。
              </p>
            </div>
            <button
              class="grid h-8 w-8 shrink-0 place-items-center rounded-md text-slate-300 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white"
              type="button"
              aria-label="关闭提示"
              title="关闭提示"
              @click="showCloseActionDialog = false"
            >
              <X class="h-4 w-4" />
            </button>
          </div>

          <label class="mt-4 flex items-center gap-2 rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] px-3 py-2.5 text-sm text-slate-300">
            <input
              v-model="rememberCloseAction"
              data-close-action-remember
              type="checkbox"
              class="h-4 w-4 rounded border-[color:var(--main-line)] bg-[color:var(--field-bg)] accent-[color:var(--theme-accent)]"
            >
            <span>记住我的选择</span>
          </label>

          <div class="mt-5 grid gap-3 sm:grid-cols-2">
            <Button
              data-close-action-minimize
              variant="primary"
              :disabled="closeActionSaving"
              @click="chooseCloseAction('minimize')"
            >
              最小化到托盘
            </Button>
            <Button
              data-close-action-exit
              variant="danger"
              :disabled="closeActionSaving"
              @click="chooseCloseAction('exit')"
            >
              直接退出
            </Button>
          </div>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>
