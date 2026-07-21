import { emit } from "@tauri-apps/api/event";
import { register, unregister } from "@tauri-apps/plugin-global-shortcut";
import { defineStore } from "pinia";

import {
  GlobalShortcutController,
  shortcutBindingsFromConfig,
  type ShortcutAction,
  type ShortcutApplyResult,
} from "@/lib/globalShortcut";
import type { AppConfig } from "@/types/config";

export type ShortcutRegistrationStatus =
  | "idle"
  | "applying"
  | "registered"
  | "disabled"
  | "suspended"
  | "error";

const controller = new GlobalShortcutController({
  register: async (shortcut, handler) => {
    await register(shortcut, (event) => handler(event.state));
  },
  unregister,
  trigger: (action) => {
    void emit("global-shortcut-triggered", action);
  },
});

export const useShortcutStore = defineStore("shortcuts", {
  state: () => ({
    status: "idle" as ShortcutRegistrationStatus,
    registeredShortcuts: {} as Partial<Record<ShortcutAction, string>>,
    errors: {} as Partial<Record<ShortcutAction, string>>,
  }),
  actions: {
    async apply(config: AppConfig): Promise<ShortcutApplyResult> {
      this.status = "applying";
      this.errors = {};
      const bindings = shortcutBindingsFromConfig(config);
      const result = await controller.apply(bindings);
      this.registeredShortcuts = { ...controller.registeredShortcuts };
      if (!result.ok && result.action && result.error) {
        this.errors = { [result.action]: result.error };
      }
      this.status = result.ok
        ? bindings.some((binding) => binding.enabled) ? "registered" : "disabled"
        : "error";
      return result;
    },
    async suspend(): Promise<ShortcutApplyResult> {
      const result = await controller.suspend();
      this.registeredShortcuts = { ...controller.registeredShortcuts };
      this.errors = result.action && result.error ? { [result.action]: result.error } : {};
      this.status = result.ok ? "suspended" : "error";
      return result;
    },
    async dispose() {
      await controller.dispose();
      this.registeredShortcuts = {};
      this.errors = {};
      this.status = "idle";
    },
  },
});
