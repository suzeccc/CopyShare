<script setup lang="ts">
import { computed, ref } from "vue";
import appIconUrl from "../../../src-tauri/icons/icon.ico?url";
import { startWindowDrag, waitForPrimaryMouseRelease } from "@/lib/tauri";

const props = defineProps<{ running: boolean; connectedCount: number; pulse: number }>();
const emit = defineEmits<{ open: []; menu: []; dock: [] }>();
const pointer = ref<{ x: number; y: number } | null>(null);
const statusText = computed(() => props.running
  ? props.connectedCount > 0 ? `同步中，已连接 ${props.connectedCount} 台设备` : "同步中，尚未连接设备"
  : "同步已暂停");
let dragged = false;

function pointerDown(event: PointerEvent) {
  if (event.button !== 0) return;
  pointer.value = { x: event.screenX, y: event.screenY };
  dragged = false;
}

function pointerMove(event: PointerEvent) {
  if (!pointer.value || dragged) return;
  if (Math.hypot(event.screenX - pointer.value.x, event.screenY - pointer.value.y) < 5) return;
  dragged = true;
  pointer.value = null;
  void startWindowDrag().then(waitForPrimaryMouseRelease).then(() => emit("dock")).catch(() => undefined);
}

function open() {
  if (dragged) { dragged = false; return; }
  emit("open");
}

function keydown(event: KeyboardEvent) {
  if (event.key === "Enter" || event.key === " ") dragged = false;
  if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10")) {
    event.preventDefault();
    emit("menu");
  }
}
</script>

<template>
  <div data-floating-ball class="grid h-full w-full place-items-center bg-transparent p-[2px]">
    <button
      data-floating-ball-button
      type="button"
      class="floating-ball relative grid h-full w-full place-items-center rounded-full border border-[color:var(--floating-surface-line)]"
      :aria-label="`展开浮窗，${statusText}`"
      :title="`${statusText} · 单击展开，右键更多`"
      @pointerdown="pointerDown"
      @pointermove="pointerMove"
      @pointerup="pointer = null"
      @pointercancel="pointer = null"
      @keydown="keydown"
      @click="open"
      @contextmenu.prevent.stop="emit('menu')"
    >
      <span v-if="pulse" :key="pulse" data-floating-ball-pulse class="floating-ball-pulse pointer-events-none absolute inset-0 rounded-full border-2 border-[color:var(--accent-text)]" />
      <img :src="appIconUrl" alt="" draggable="false" class="pointer-events-none h-7 w-7 rounded-md object-contain" />
      <span
        data-floating-ball-status
        class="absolute bottom-1 right-1 h-2.5 w-2.5 rounded-full border-2 border-[color:var(--main-bg)]"
        :class="running ? connectedCount ? 'bg-emerald-400' : 'bg-yellow-400' : 'bg-slate-400'"
        aria-hidden="true"
      />
    </button>
  </div>
</template>

<style scoped>
.floating-ball { background: var(--floating-surface-bg); background-color: var(--main-bg); animation: floating-ball-enter 180ms cubic-bezier(.16, 1, .3, 1); transition: transform 160ms ease-out; }
@media (hover: hover) { .floating-ball:hover { transform: scale(1.04); } }
.floating-ball:active { transform: scale(.97); }
.floating-ball:focus-visible { outline: none; box-shadow: inset 0 0 0 2px var(--theme-accent); }
.floating-ball-pulse { animation: floating-ball-pulse 900ms ease-out both; }
@keyframes floating-ball-enter { from { opacity: 0; transform: scale(.72); } to { opacity: 1; transform: scale(1); } }
@keyframes floating-ball-pulse { from { opacity: .8; transform: scale(1); } to { opacity: 0; transform: scale(1.35); } }
@media (prefers-reduced-motion: reduce) {
  .floating-ball, .floating-ball-pulse { animation: none; transition: none; }
}
</style>
