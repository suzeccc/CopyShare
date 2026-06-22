<script setup lang="ts">
import { Minus, Square, X } from "lucide-vue-next";

import appIconUrl from "../../../src-tauri/icons/icon.ico?url";
import {
  closeWindow,
  minimizeWindow,
  startWindowDrag,
  toggleMaximizeWindow,
} from "@/lib/tauri";
import { shouldStartWindowDrag } from "@/lib/windowDrag";

function handleWindowDrag(event: MouseEvent) {
  if (!shouldStartWindowDrag(event)) {
    return;
  }

  void startWindowDrag();
}
</script>

<template>
  <header
    class="flex h-10 shrink-0 select-none items-center justify-between border-b border-[color:var(--main-line)] bg-[color:var(--main-bg-deep)] text-slate-100"
    data-tauri-drag-region
    @dblclick="toggleMaximizeWindow()"
    @mousedown.capture="handleWindowDrag"
  >
    <div class="flex min-w-0 items-center gap-2 px-4" data-tauri-drag-region>
      <img
        :src="appIconUrl"
        alt=""
        class="h-5 w-5 shrink-0 rounded-sm"
        draggable="false"
        data-tauri-drag-region
      >
      <span class="truncate text-sm font-medium text-white" data-tauri-drag-region>
        Copy-Sharer
      </span>
    </div>

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
        @click="closeWindow()"
      >
        <X class="h-4 w-4" />
      </button>
    </div>
  </header>
</template>
