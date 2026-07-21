import type { AppConfig } from "@/types/config";

export const DEFAULT_SHORTCUTS = {
  quickPanel: "Alt+Shift+V",
  ocr: "Alt+Shift+O",
  translate: "Alt+Shift+T",
  snippets: "Alt+Shift+B",
  toggleSync: "Alt+Shift+S",
} as const;

export type ShortcutAction = keyof typeof DEFAULT_SHORTCUTS;
export type ShortcutEnabledKey =
  | "quickPanelShortcutEnabled"
  | "ocrShortcutEnabled"
  | "translateShortcutEnabled"
  | "snippetsShortcutEnabled"
  | "toggleSyncShortcutEnabled";
export type ShortcutKey =
  | "quickPanelShortcut"
  | "ocrShortcut"
  | "translateShortcut"
  | "snippetsShortcut"
  | "toggleSyncShortcut";

export type ShortcutDefinition = {
  action: ShortcutAction;
  label: string;
  description: string;
  enabledKey: ShortcutEnabledKey;
  shortcutKey: ShortcutKey;
  defaultEnabled: boolean;
  defaultShortcut: string;
};

export const SHORTCUT_DEFINITIONS: readonly ShortcutDefinition[] = [
  {
    action: "quickPanel",
    label: "快速剪贴板面板",
    description: "在任意应用中呼出或隐藏剪贴板历史窗口",
    enabledKey: "quickPanelShortcutEnabled",
    shortcutKey: "quickPanelShortcut",
    defaultEnabled: true,
    defaultShortcut: DEFAULT_SHORTCUTS.quickPanel,
  },
  {
    action: "ocr",
    label: "OCR 识别剪贴板图片",
    description: "识别当前剪贴板图片并打开图片转文字",
    enabledKey: "ocrShortcutEnabled",
    shortcutKey: "ocrShortcut",
    defaultEnabled: false,
    defaultShortcut: DEFAULT_SHORTCUTS.ocr,
  },
  {
    action: "translate",
    label: "翻译剪贴板文本",
    description: "读取当前剪贴板文本并立即翻译",
    enabledKey: "translateShortcutEnabled",
    shortcutKey: "translateShortcut",
    defaultEnabled: false,
    defaultShortcut: DEFAULT_SHORTCUTS.translate,
  },
  {
    action: "snippets",
    label: "打开常用片段",
    description: "显示主窗口并直接进入常用片段",
    enabledKey: "snippetsShortcutEnabled",
    shortcutKey: "snippetsShortcut",
    defaultEnabled: false,
    defaultShortcut: DEFAULT_SHORTCUTS.snippets,
  },
  {
    action: "toggleSync",
    label: "暂停 / 恢复同步",
    description: "切换当前同步运行状态，不改变自动同步设置",
    enabledKey: "toggleSyncShortcutEnabled",
    shortcutKey: "toggleSyncShortcut",
    defaultEnabled: false,
    defaultShortcut: DEFAULT_SHORTCUTS.toggleSync,
  },
];

export type ShortcutBinding = {
  action: ShortcutAction;
  enabled: boolean;
  shortcut: string;
};

export function shortcutBindingsFromConfig(config: AppConfig): ShortcutBinding[] {
  return SHORTCUT_DEFINITIONS.map((definition) => ({
    action: definition.action,
    enabled: config[definition.enabledKey],
    shortcut: config[definition.shortcutKey],
  }));
}

export type ShortcutKeyboardEvent = {
  code: string;
  key: string;
  altKey?: boolean;
  ctrlKey?: boolean;
  metaKey?: boolean;
  shiftKey?: boolean;
};

export type ShortcutEventState = "Pressed" | "Released";

export type GlobalShortcutRuntime = {
  register: (
    shortcut: string,
    handler: (state: ShortcutEventState) => void,
  ) => Promise<void>;
  unregister: (shortcut: string) => Promise<void>;
  trigger: (action: ShortcutAction) => void;
};

export type ShortcutApplyResult = {
  ok: boolean;
  error: string | null;
  action: ShortcutAction | null;
};

const MODIFIER_CODES = new Set([
  "AltLeft",
  "AltRight",
  "ControlLeft",
  "ControlRight",
  "MetaLeft",
  "MetaRight",
  "ShiftLeft",
  "ShiftRight",
]);

const SPECIAL_KEYS: Record<string, string> = {
  ArrowDown: "ArrowDown",
  ArrowLeft: "ArrowLeft",
  ArrowRight: "ArrowRight",
  ArrowUp: "ArrowUp",
  Backspace: "Backspace",
  Delete: "Delete",
  End: "End",
  Enter: "Enter",
  Equal: "Equal",
  Home: "Home",
  Insert: "Insert",
  Minus: "Minus",
  PageDown: "PageDown",
  PageUp: "PageUp",
  Space: "Space",
  Tab: "Tab",
};

function shortcutKey(code: string): string | null {
  if (MODIFIER_CODES.has(code)) return null;
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F(?:[1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  return SPECIAL_KEYS[code] ?? null;
}

export function shortcutFromKeyboardEvent(event: ShortcutKeyboardEvent): string | null {
  const key = shortcutKey(event.code);
  if (!key) return null;

  const modifiers: string[] = [];
  if (event.ctrlKey || event.metaKey) modifiers.push("CommandOrControl");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");
  if (!modifiers.length) return null;

  return [...modifiers, key].join("+");
}

export function formatShortcutLabel(shortcut: string): string {
  return shortcut
    .replace("CommandOrControl", "Ctrl / Cmd")
    .split("+")
    .join(" + ");
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function activeBindings(bindings: readonly ShortcutBinding[]): ShortcutBinding[] {
  return bindings
    .filter((binding) => binding.enabled)
    .map((binding) => ({ ...binding, shortcut: binding.shortcut.trim() }));
}

function bindingMapsEqual(
  left: Partial<Record<ShortcutAction, string>>,
  right: readonly ShortcutBinding[],
): boolean {
  const leftEntries = Object.entries(left);
  return leftEntries.length === right.length
    && right.every((binding) => left[binding.action] === binding.shortcut);
}

export class GlobalShortcutController {
  registeredShortcuts: Partial<Record<ShortcutAction, string>> = {};
  private readonly runtime: GlobalShortcutRuntime;

  constructor(runtime: GlobalShortcutRuntime) {
    this.runtime = runtime;
  }

  async apply(bindings: readonly ShortcutBinding[]): Promise<ShortcutApplyResult> {
    const desired = activeBindings(bindings);
    const validation = this.validate(desired);
    if (!validation.ok) return validation;
    if (bindingMapsEqual(this.registeredShortcuts, desired)) {
      return { ok: true, error: null, action: null };
    }

    const previous = Object.entries(this.registeredShortcuts).map(([action, shortcut]) => ({
      action: action as ShortcutAction,
      enabled: true,
      shortcut,
    }));
    const clearResult = await this.clearRegistered();
    if (!clearResult.ok) {
      const rollbackError = await this.restore(previous);
      return {
        ok: false,
        error: rollbackError ? `${clearResult.error}；恢复原快捷键失败：${rollbackError}` : clearResult.error,
        action: clearResult.action,
      };
    }

    for (const binding of desired) {
      try {
        await this.register(binding);
      } catch (error) {
        const registrationError = errorMessage(error);
        await this.clearRegistered();
        const rollbackError = await this.restore(previous);
        return {
          ok: false,
          error: rollbackError
            ? `${registrationError}；恢复原快捷键失败：${rollbackError}`
            : registrationError,
          action: binding.action,
        };
      }
    }

    return { ok: true, error: null, action: null };
  }

  async suspend(): Promise<ShortcutApplyResult> {
    return this.clearRegistered();
  }

  async dispose(): Promise<void> {
    await this.suspend();
  }

  private validate(bindings: readonly ShortcutBinding[]): ShortcutApplyResult {
    const shortcuts = new Map<string, ShortcutAction>();
    for (const binding of bindings) {
      if (!binding.shortcut) {
        return { ok: false, error: "快捷键不能为空", action: binding.action };
      }
      const duplicateAction = shortcuts.get(binding.shortcut);
      if (duplicateAction) {
        return {
          ok: false,
          error: "该组合键与其他已启用功能重复",
          action: binding.action,
        };
      }
      shortcuts.set(binding.shortcut, binding.action);
    }
    return { ok: true, error: null, action: null };
  }

  private async register(binding: ShortcutBinding): Promise<void> {
    await this.runtime.register(binding.shortcut, (state) => {
      if (state === "Pressed") this.runtime.trigger(binding.action);
    });
    this.registeredShortcuts[binding.action] = binding.shortcut;
  }

  private async clearRegistered(): Promise<ShortcutApplyResult> {
    for (const [action, shortcut] of Object.entries(this.registeredShortcuts)) {
      try {
        await this.runtime.unregister(shortcut);
        delete this.registeredShortcuts[action as ShortcutAction];
      } catch (error) {
        return {
          ok: false,
          error: errorMessage(error),
          action: action as ShortcutAction,
        };
      }
    }
    return { ok: true, error: null, action: null };
  }

  private async restore(bindings: readonly ShortcutBinding[]): Promise<string | null> {
    for (const binding of bindings) {
      if (this.registeredShortcuts[binding.action] === binding.shortcut) continue;
      try {
        await this.register(binding);
      } catch (error) {
        return errorMessage(error);
      }
    }
    return null;
  }
}
