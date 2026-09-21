<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import ChevronLeft from "lucide-vue-next/dist/esm/icons/chevron-left.js";
import ChevronRight from "lucide-vue-next/dist/esm/icons/chevron-right.js";
import ZoomOut from "lucide-vue-next/dist/esm/icons/zoom-out.js";
import ZoomIn from "lucide-vue-next/dist/esm/icons/zoom-in.js";
import Minus from "lucide-vue-next/dist/esm/icons/minus.js";
import Plus from "lucide-vue-next/dist/esm/icons/plus.js";
import RotateCw from "lucide-vue-next/dist/esm/icons/rotate-cw.js";
import RotateCcw from "lucide-vue-next/dist/esm/icons/rotate-ccw.js";
import Download from "lucide-vue-next/dist/esm/icons/download.js";
import Maximize2 from "lucide-vue-next/dist/esm/icons/maximize-2.js";
import Minimize2 from "lucide-vue-next/dist/esm/icons/minimize-2.js";
import { saveHistoryMedia } from "@/lib/tauri";
import { useToastStore } from "@/stores/toasts";

const props = defineProps<{
  kind: "image" | "video";
  historyId: string;
  value: number;
  previousDisabled: boolean;
  nextDisabled: boolean;
  decreaseDisabled: boolean;
  increaseDisabled: boolean;
  fullscreenTarget: HTMLElement | null;
}>();
const emit = defineEmits<{
  (event: "previous" | "next" | "decrease" | "increase" | "reset" | "rotate"): void;
}>();
const toastStore = useToastStore();
const saving = ref(false);
const fullscreen = ref(false);
const label = computed(() => props.kind === "image" ? `${Math.round(props.value * 100)}%` : `${props.value}×`);

async function save() {
  if (saving.value || !props.historyId) return;
  saving.value = true;
  try {
    if (await saveHistoryMedia(props.historyId)) toastStore.success("文件已另存");
  } catch (error) {
    toastStore.error(`另存失败：${String(error)}`);
  } finally {
    saving.value = false;
  }
}

async function toggleFullscreen() {
  try {
    if (document.fullscreenElement) await document.exitFullscreen();
    else if (props.fullscreenTarget) await props.fullscreenTarget.requestFullscreen();
  } catch (error) {
    toastStore.error(`无法进入全屏：${String(error)}`);
  }
}

function syncFullscreen() {
  fullscreen.value = !!props.fullscreenTarget && document.fullscreenElement === props.fullscreenTarget;
}
onMounted(() => document.addEventListener("fullscreenchange", syncFullscreen));
onUnmounted(() => {
  document.removeEventListener("fullscreenchange", syncFullscreen);
  if (props.fullscreenTarget && document.fullscreenElement === props.fullscreenTarget) {
    void document.exitFullscreen().catch(() => undefined);
  }
});
</script>

<template>
  <div data-media-preview-toolbar role="group" aria-label="预览工具栏" class="preview-toolbar">
    <Teleport v-if="kind === 'image' && fullscreenTarget" :to="fullscreenTarget">
      <button type="button" class="preview-action preview-navigation preview-navigation-left" :disabled="previousDisabled" aria-label="上一张图片" title="上一张" @click="emit('previous')"><ChevronLeft /></button>
      <button type="button" class="preview-action preview-navigation preview-navigation-right" :disabled="nextDisabled" aria-label="下一张图片" title="下一张" @click="emit('next')"><ChevronRight /></button>
    </Teleport>
    <button v-if="kind === 'video'" type="button" class="preview-action" :disabled="previousDisabled" aria-label="上一条" title="上一条" @click="emit('previous')"><ChevronLeft /></button>
    <button type="button" class="preview-action" :disabled="decreaseDisabled" :aria-label="kind === 'image' ? '缩小图片' : '降低播放速度'" :title="kind === 'image' ? '缩小图片' : '降低播放速度'" @click="emit('decrease')"><ZoomOut v-if="kind === 'image'" /><Minus v-else /></button>
    <button type="button" class="preview-value" :aria-label="kind === 'image' ? '重置视图' : '恢复正常速度'" :title="kind === 'image' ? '重置视图' : '恢复正常速度'" @click="emit('reset')">{{ label }}</button>
    <button type="button" class="preview-action" :disabled="increaseDisabled" :aria-label="kind === 'image' ? '放大图片' : '提高播放速度'" :title="kind === 'image' ? '放大图片' : '提高播放速度'" @click="emit('increase')"><ZoomIn v-if="kind === 'image'" /><Plus v-else /></button>
    <span class="preview-divider" aria-hidden="true" />
    <button type="button" class="preview-action" :aria-label="kind === 'image' ? '旋转图片' : '从头播放'" :title="kind === 'image' ? '旋转图片' : '从头播放'" @click="emit('rotate')"><RotateCw v-if="kind === 'image'" /><RotateCcw v-else /></button>
    <button type="button" class="preview-action" :disabled="saving || !historyId" :aria-busy="saving" aria-label="另存文件" title="另存文件" @click="save"><Download /></button>
    <button type="button" class="preview-action" :disabled="!fullscreenTarget" :aria-label="fullscreen ? '退出全屏' : '全屏预览'" :title="fullscreen ? '退出全屏' : '全屏预览'" @click="toggleFullscreen"><Minimize2 v-if="fullscreen" /><Maximize2 v-else /></button>
    <button v-if="kind === 'video'" type="button" class="preview-action" :disabled="nextDisabled" aria-label="下一条" title="下一条" @click="emit('next')"><ChevronRight /></button>
  </div>
</template>

<style scoped>
.preview-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  width: max-content;
  max-width: 100%;
  padding: 8px 12px;
  border: 1px solid rgb(255 255 255 / 14%);
  border-radius: 999px;
  background: rgb(38 42 46 / 86%);
  box-shadow: 0 8px 28px rgb(0 0 0 / 30%), inset 0 1px 0 rgb(255 255 255 / 4%);
  backdrop-filter: blur(20px);
  color: rgb(255 255 255 / 85%);
}
.preview-action, .preview-value {
  display: grid;
  flex-shrink: 0;
  place-items: center;
  height: 36px;
  border-radius: 999px;
  transition: background 150ms, color 150ms;
}
.preview-action { width: 36px; }
.preview-action :deep(svg) { width: 19px; height: 19px; }
.preview-navigation {
  position: absolute;
  top: 50%;
  z-index: 10;
  width: 44px;
  height: 44px;
  transform: translateY(-50%);
  border: 1px solid rgb(255 255 255 / 14%);
  background: rgb(38 42 46 / 86%);
  color: rgb(255 255 255 / 85%);
  box-shadow: 0 6px 20px rgb(0 0 0 / 25%);
  backdrop-filter: blur(20px);
}
.preview-navigation-left { left: 12px; }
.preview-navigation-right { right: 12px; }
.preview-value { min-width: 52px; font-size: 13px; font-variant-numeric: tabular-nums; }
.preview-action:hover:not(:disabled), .preview-value:hover { background: rgb(255 255 255 / 9%); color: white; }
.preview-navigation:hover:not(:disabled) { background: rgb(56 60 64 / 92%); }
.preview-action:disabled { opacity: 0.3; cursor: not-allowed; }
.preview-action:focus-visible, .preview-value:focus-visible { outline: 2px solid rgb(255 255 255 / 60%); outline-offset: 2px; }
.preview-divider { height: 22px; border-left: 1px solid rgb(255 255 255 / 10%); margin: 0 4px; }
@media (max-width: 480px) {
  .preview-toolbar { gap: 0; padding: 6px; }
  .preview-action { width: 28px; height: 32px; }
  .preview-navigation { width: 36px; height: 36px; }
  .preview-navigation-left { left: 6px; }
  .preview-navigation-right { right: 6px; }
  .preview-value { min-width: 48px; }
  .preview-divider { margin: 0 2px; }
}
@media (prefers-reduced-motion: reduce) {
  .preview-action, .preview-value { transition: none; }
}
</style>
