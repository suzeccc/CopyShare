import assert from "node:assert/strict";
import test from "node:test";

import {
  DEFAULT_SHORTCUTS,
  GlobalShortcutController,
  SHORTCUT_DEFINITIONS,
  shortcutFromKeyboardEvent,
  type ShortcutAction,
  type ShortcutBinding,
} from "../src/lib/globalShortcut.ts";

test("shortcut defaults keep only the quick panel enabled", () => {
  assert.equal(SHORTCUT_DEFINITIONS.length, 5);
  assert.deepEqual(
    SHORTCUT_DEFINITIONS.filter((item) => item.defaultEnabled).map((item) => item.action),
    ["quickPanel"],
  );
  assert.deepEqual(DEFAULT_SHORTCUTS, {
    quickPanel: "Alt+Shift+V",
    ocr: "Alt+Shift+O",
    translate: "Alt+Shift+T",
    snippets: "Alt+Shift+B",
    toggleSync: "Alt+Shift+S",
  });
});

test("shortcut recorder requires a modifier and normalizes portable combinations", () => {
  assert.equal(shortcutFromKeyboardEvent({ code: "KeyV", key: "v", altKey: true }), "Alt+V");
  assert.equal(
    shortcutFromKeyboardEvent({ code: "KeyK", key: "k", ctrlKey: true, shiftKey: true }),
    "CommandOrControl+Shift+K",
  );
  assert.equal(shortcutFromKeyboardEvent({ code: "KeyV", key: "v" }), null);
  assert.equal(shortcutFromKeyboardEvent({ code: "AltLeft", key: "Alt", altKey: true }), null);
});

test("multi-action controller routes pressed events to the matching action", async () => {
  const handlers = new Map<string, (state: "Pressed" | "Released") => void>();
  const triggered: ShortcutAction[] = [];
  const controller = new GlobalShortcutController({
    register: async (shortcut, handler) => {
      handlers.set(shortcut, handler);
    },
    unregister: async (shortcut) => {
      handlers.delete(shortcut);
    },
    trigger: (action) => triggered.push(action),
  });
  const bindings: ShortcutBinding[] = [
    { action: "quickPanel", enabled: true, shortcut: "Alt+Shift+V" },
    { action: "ocr", enabled: true, shortcut: "Alt+Shift+O" },
  ];

  assert.deepEqual(await controller.apply(bindings), { ok: true, error: null, action: null });
  assert.deepEqual(controller.registeredShortcuts, {
    quickPanel: "Alt+Shift+V",
    ocr: "Alt+Shift+O",
  });
  handlers.get("Alt+Shift+O")?.("Released");
  handlers.get("Alt+Shift+O")?.("Pressed");
  assert.deepEqual(triggered, ["ocr"]);
});

test("duplicate enabled shortcuts are rejected before registration", async () => {
  let registrationCount = 0;
  const controller = new GlobalShortcutController({
    register: async () => {
      registrationCount += 1;
    },
    unregister: async () => {},
    trigger: () => {},
  });

  const result = await controller.apply([
    { action: "quickPanel", enabled: true, shortcut: "Alt+Shift+V" },
    { action: "ocr", enabled: true, shortcut: "Alt+Shift+V" },
  ]);

  assert.equal(result.ok, false);
  assert.equal(result.action, "ocr");
  assert.match(result.error ?? "", /重复|duplicate/i);
  assert.equal(registrationCount, 0);
});

test("failed multi-action replacement restores every previous registration", async () => {
  const active = new Set<string>();
  const controller = new GlobalShortcutController({
    register: async (shortcut) => {
      if (shortcut === "Alt+Shift+X") throw new Error("shortcut already in use");
      active.add(shortcut);
    },
    unregister: async (shortcut) => {
      active.delete(shortcut);
    },
    trigger: () => {},
  });
  const previous: ShortcutBinding[] = [
    { action: "quickPanel", enabled: true, shortcut: "Alt+Shift+V" },
    { action: "ocr", enabled: true, shortcut: "Alt+Shift+O" },
  ];

  assert.equal((await controller.apply(previous)).ok, true);
  const result = await controller.apply([
    ...previous,
    { action: "translate", enabled: true, shortcut: "Alt+Shift+X" },
  ]);

  assert.equal(result.ok, false);
  assert.equal(result.action, "translate");
  assert.match(result.error ?? "", /shortcut already in use/);
  assert.deepEqual(controller.registeredShortcuts, {
    quickPanel: "Alt+Shift+V",
    ocr: "Alt+Shift+O",
  });
  assert.deepEqual([...active].sort(), ["Alt+Shift+O", "Alt+Shift+V"]);
});

test("suspend unregisters all active shortcuts", async () => {
  const removals: string[] = [];
  const controller = new GlobalShortcutController({
    register: async () => {},
    unregister: async (shortcut) => removals.push(shortcut),
    trigger: () => {},
  });

  await controller.apply([
    { action: "quickPanel", enabled: true, shortcut: "Alt+Shift+V" },
    { action: "ocr", enabled: true, shortcut: "Alt+Shift+O" },
  ]);
  assert.equal((await controller.suspend()).ok, true);
  assert.deepEqual(controller.registeredShortcuts, {});
  assert.deepEqual(removals, ["Alt+Shift+V", "Alt+Shift+O"]);
});
