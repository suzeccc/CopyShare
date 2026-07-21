<script setup lang="ts">
import {
  ClipboardList,
  Keyboard,
  Languages,
  MessageSquareText,
  RefreshCw,
  RotateCcw,
  ScanText,
  X,
} from "lucide-vue-next";
import type { Component } from "vue";
import { computed, nextTick, onBeforeUnmount, reactive, ref, watch } from "vue";

import Button from "@/components/ui/Button.vue";
import Switch from "@/components/ui/Switch.vue";
import {
  formatShortcutLabel,
  SHORTCUT_DEFINITIONS,
  shortcutFromKeyboardEvent,
  type ShortcutAction,
  type ShortcutDefinition,
} from "@/lib/globalShortcut";
import { useConfigStore } from "@/stores/config";
import { useShortcutStore } from "@/stores/shortcuts";
import { useToastStore } from "@/stores/toasts";
import type { AppConfig } from "@/types/config";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();

const configStore = useConfigStore();
const shortcutStore = useShortcutStore();
const toastStore = useToastStore();
const draft = reactive<AppConfig>({ ...configStore.config });
const dialogRef = ref<HTMLElement | null>(null);
const recordingAction = ref<ShortcutAction | null>(null);
const savingAction = ref<ShortcutAction | "all" | null>(null);
const rowErrors = reactive<Partial<Record<ShortcutAction, string>>>({});

const actionIcons: Record<ShortcutAction, Component> = {
  quickPanel: ClipboardList,
  ocr: ScanText,
  translate: Languages,
  snippets: MessageSquareText,
  toggleSync: RefreshCw,
};

const enabledCount = computed(() =>
  SHORTCUT_DEFINITIONS.filter((definition) => draft[definition.enabledKey]).length,
);

function clearErrors() {
  for (const action of Object.keys(rowErrors) as ShortcutAction[]) {
    delete rowErrors[action];
  }
}

function syncDraft() {
  Object.assign(draft, configStore.config);
}

function stopShortcutRecording() {
  recordingAction.value = null;
  window.removeEventListener("keydown", handleShortcutRecordingKeydown, true);
}

async function cancelShortcutRecording() {
  if (!recordingAction.value) return;
  stopShortcutRecording();
  await shortcutStore.apply(configStore.config);
}

function duplicateDefinition(definition: ShortcutDefinition, shortcut: string) {
  return SHORTCUT_DEFINITIONS.find((candidate) =>
    candidate.action !== definition.action
    && draft[candidate.enabledKey]
    && draft[candidate.shortcutKey].trim() === shortcut,
  );
}

async function saveShortcut(
  definition: ShortcutDefinition,
  enabled: boolean,
  shortcut: string,
) {
  if (savingAction.value || configStore.saving) return false;
  const normalizedShortcut = shortcut.trim() || definition.defaultShortcut;
  const duplicate = enabled ? duplicateDefinition(definition, normalizedShortcut) : undefined;
  if (duplicate) {
    rowErrors[definition.action] = `与“${duplicate.label}”使用了相同组合键`;
    await shortcutStore.apply(configStore.config);
    syncDraft();
    return false;
  }

  const previousConfig = { ...configStore.config };
  const nextConfig: AppConfig = {
    ...previousConfig,
    [definition.enabledKey]: enabled,
    [definition.shortcutKey]: normalizedShortcut,
  };
  draft[definition.enabledKey] = enabled;
  draft[definition.shortcutKey] = normalizedShortcut;
  savingAction.value = definition.action;
  delete rowErrors[definition.action];

  try {
    const registration = await shortcutStore.apply(nextConfig);
    if (!registration.ok) {
      await shortcutStore.apply(previousConfig);
      syncDraft();
      rowErrors[definition.action] = registration.error ?? "快捷键注册失败";
      toastStore.error("快捷键注册失败，可能已被其他程序占用");
      return false;
    }

    await configStore.save(nextConfig);
    if (configStore.error) {
      await shortcutStore.apply(previousConfig);
      syncDraft();
      rowErrors[definition.action] = "设置保存失败，请重试";
      toastStore.error("快捷键设置保存失败");
      return false;
    }

    syncDraft();
    toastStore.success(enabled ? "快捷键已更新" : "快捷键已关闭");
    return true;
  } finally {
    savingAction.value = null;
  }
}

async function toggleShortcut(definition: ShortcutDefinition, enabled: boolean) {
  if (recordingAction.value) stopShortcutRecording();
  await saveShortcut(definition, enabled, draft[definition.shortcutKey]);
}

async function startShortcutRecording(definition: ShortcutDefinition) {
  if (savingAction.value || configStore.saving) return;
  if (recordingAction.value === definition.action) {
    await cancelShortcutRecording();
    return;
  }
  if (recordingAction.value) await cancelShortcutRecording();

  const suspended = await shortcutStore.suspend();
  if (!suspended.ok) {
    rowErrors[definition.action] = suspended.error ?? "无法暂停当前快捷键";
    await shortcutStore.apply(configStore.config);
    return;
  }

  delete rowErrors[definition.action];
  recordingAction.value = definition.action;
  window.addEventListener("keydown", handleShortcutRecordingKeydown, true);
}

function handleShortcutRecordingKeydown(event: KeyboardEvent) {
  const action = recordingAction.value;
  if (!action || event.repeat) return;
  event.preventDefault();
  event.stopImmediatePropagation();

  if (event.key === "Escape") {
    void cancelShortcutRecording();
    return;
  }

  const shortcut = shortcutFromKeyboardEvent(event);
  if (!shortcut) return;
  const definition = SHORTCUT_DEFINITIONS.find((item) => item.action === action);
  if (!definition) return;

  stopShortcutRecording();
  void saveShortcut(definition, true, shortcut);
}

async function restoreShortcutDefault(definition: ShortcutDefinition) {
  if (recordingAction.value) stopShortcutRecording();
  await saveShortcut(
    definition,
    draft[definition.enabledKey],
    definition.defaultShortcut,
  );
}

async function restoreAllDefaults() {
  if (savingAction.value || configStore.saving) return;
  if (recordingAction.value) stopShortcutRecording();

  const previousConfig = { ...configStore.config };
  const nextConfig = { ...previousConfig };
  for (const definition of SHORTCUT_DEFINITIONS) {
    nextConfig[definition.enabledKey] = definition.defaultEnabled;
    nextConfig[definition.shortcutKey] = definition.defaultShortcut;
  }
  savingAction.value = "all";
  clearErrors();

  try {
    const registration = await shortcutStore.apply(nextConfig);
    if (!registration.ok) {
      await shortcutStore.apply(previousConfig);
      syncDraft();
      if (registration.action && registration.error) {
        rowErrors[registration.action] = registration.error;
      }
      toastStore.error("默认快捷键注册失败，可能已被其他程序占用");
      return;
    }
    await configStore.save(nextConfig);
    if (configStore.error) {
      await shortcutStore.apply(previousConfig);
      syncDraft();
      toastStore.error("快捷键设置保存失败");
      return;
    }
    syncDraft();
    toastStore.success("已恢复默认快捷键");
  } finally {
    savingAction.value = null;
  }
}

function statusText(definition: ShortcutDefinition) {
  if (rowErrors[definition.action]) return rowErrors[definition.action];
  if (recordingAction.value === definition.action) return "请按下组合键，Esc 取消";
  if (!draft[definition.enabledKey]) return "未启用";
  if (shortcutStore.registeredShortcuts[definition.action] === draft[definition.shortcutKey]) {
    return "已注册";
  }
  return "等待注册";
}

async function closeDialog() {
  if (savingAction.value) return;
  await cancelShortcutRecording();
  clearErrors();
  emit("close");
}

watch(
  () => props.open,
  async (open) => {
    if (!open) {
      await cancelShortcutRecording();
      return;
    }
    clearErrors();
    syncDraft();
    await nextTick();
    dialogRef.value?.focus();
  },
);

watch(
  () => configStore.config,
  () => {
    if (!savingAction.value) syncDraft();
  },
  { deep: true },
);

onBeforeUnmount(() => {
  const wasRecording = Boolean(recordingAction.value);
  stopShortcutRecording();
  if (wasRecording) void shortcutStore.apply(configStore.config);
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      data-shortcut-settings-dialog
      class="fixed inset-0 z-[90] flex items-center justify-center bg-[color:var(--dialog-overlay-bg)] px-5 py-8 backdrop-blur-sm"
      @click.self="closeDialog"
    >
      <section
        ref="dialogRef"
        class="flex max-h-full w-full max-w-[680px] flex-col overflow-hidden rounded-2xl border border-[color:var(--main-line)] bg-[color:var(--dialog-bg)] text-slate-100 shadow-[0_28px_90px_rgba(0,0,0,0.58)] outline-none"
        role="dialog"
        aria-modal="true"
        aria-labelledby="shortcut-settings-title"
        tabindex="-1"
        @keydown.esc.stop.prevent="closeDialog"
      >
        <header class="flex items-start justify-between gap-4 border-b border-[color:var(--main-line-soft)] px-5 py-4">
          <div class="flex min-w-0 items-start gap-3">
            <span class="grid h-10 w-10 shrink-0 place-items-center rounded-xl border border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]">
              <Keyboard class="h-5 w-5" />
            </span>
            <span class="grid gap-1">
              <span id="shortcut-settings-title" class="text-[17px] font-bold text-white">快捷键设置</span>
              <span class="text-[12px] leading-5 text-[color:var(--muted-text)]">
                已启用 {{ enabledCount }} 个；新增功能默认关闭，按需开启
              </span>
            </span>
          </div>
          <button
            type="button"
            class="grid h-8 w-8 shrink-0 place-items-center rounded-lg text-slate-400 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white focus-visible:outline focus-visible:outline-2 focus-visible:outline-[color:var(--accent-line)]"
            aria-label="关闭快捷键设置"
            :disabled="Boolean(savingAction)"
            @click="closeDialog"
          >
            <X class="h-4 w-4" />
          </button>
        </header>

        <div class="min-h-0 flex-1 overflow-y-auto px-3 py-2 sm:px-5 sm:py-3">
          <div class="overflow-hidden rounded-xl border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg)]">
            <div
              v-for="(definition, index) in SHORTCUT_DEFINITIONS"
              :key="definition.action"
              :data-shortcut-action="definition.action"
              class="grid gap-3 px-3 py-3.5 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center sm:px-4"
              :class="index > 0 ? 'border-t border-[color:var(--main-line-soft)]' : ''"
            >
              <div class="flex min-w-0 items-start gap-3">
                <span
                  class="mt-0.5 grid h-9 w-9 shrink-0 place-items-center rounded-lg border transition"
                  :class="draft[definition.enabledKey]
                    ? 'border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]'
                    : 'border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-muted)] text-slate-500'"
                >
                  <component :is="actionIcons[definition.action]" class="h-4 w-4" />
                </span>
                <span class="grid min-w-0 gap-0.5">
                  <span class="text-[14px] font-bold text-white">{{ definition.label }}</span>
                  <span class="text-[12px] leading-5 text-[color:var(--muted-text)]">{{ definition.description }}</span>
                  <span
                    class="text-[11px]"
                    :class="rowErrors[definition.action] ? 'text-red-300' : 'text-[color:var(--subtle-text)]'"
                  >
                    {{ statusText(definition) }}
                  </span>
                </span>
              </div>

              <div class="flex items-center justify-end gap-2 pl-12 sm:pl-0">
                <button
                  data-shortcut-recorder
                  type="button"
                  class="h-9 min-w-[150px] rounded-lg border px-3 font-mono text-[12px] font-bold transition focus-visible:outline focus-visible:outline-2 focus-visible:outline-[color:var(--accent-line)]"
                  :class="recordingAction === definition.action
                    ? 'border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)] ring-2 ring-[color:var(--accent-soft)]'
                    : 'border-[color:var(--main-line-soft)] bg-[color:var(--field-bg)] text-slate-100 hover:border-[color:var(--accent-line)]'"
                  :disabled="Boolean(savingAction)"
                  :aria-pressed="recordingAction === definition.action"
                  @click="startShortcutRecording(definition)"
                >
                  {{ recordingAction === definition.action
                    ? "请按组合键…"
                    : formatShortcutLabel(draft[definition.shortcutKey]) }}
                </button>
                <button
                  type="button"
                  class="grid h-9 w-9 shrink-0 place-items-center rounded-lg text-slate-500 transition hover:bg-[color:var(--main-bg-muted)] hover:text-white disabled:opacity-35"
                  :disabled="Boolean(savingAction) || draft[definition.shortcutKey] === definition.defaultShortcut"
                  :aria-label="`恢复${definition.label}默认快捷键`"
                  title="恢复此项默认按键"
                  @click="restoreShortcutDefault(definition)"
                >
                  <RotateCcw class="h-3.5 w-3.5" />
                </button>
                <Switch
                  control-only
                  :model-value="draft[definition.enabledKey]"
                  :label="`${definition.label}快捷键`"
                  :disabled="Boolean(savingAction)"
                  @update:model-value="toggleShortcut(definition, $event)"
                />
              </div>
            </div>
          </div>
          <p class="px-1 pt-3 text-[11px] leading-5 text-[color:var(--muted-text)]">
            快速面板内的 ↑、↓、Enter 和 Esc 为固定操作键，不占用全局快捷键。
          </p>
        </div>

        <footer class="flex items-center justify-between gap-3 border-t border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-muted)] px-5 py-3">
          <Button
            size="sm"
            variant="ghost"
            :disabled="Boolean(savingAction)"
            @click="restoreAllDefaults"
          >
            <RotateCcw class="h-3.5 w-3.5" />
            恢复全部默认
          </Button>
          <Button
            size="sm"
            variant="primary"
            :disabled="Boolean(savingAction)"
            @click="closeDialog"
          >
            完成
          </Button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>
