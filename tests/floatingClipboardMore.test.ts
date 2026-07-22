import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");
const appShell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const tauri = readFileSync("src/lib/tauri.ts", "utf8");
const router = readFileSync("src/router/index.ts", "utf8");
const app = readFileSync("src/App.vue", "utf8");
const defaultCapability = JSON.parse(
  readFileSync("src-tauri/capabilities/default.json", "utf8"),
);
const floatingClipboardWindowPath = "src/pages/FloatingClipboardHistory.vue";

assert.match(floatingPanel, /data-floating-more-clipboard-button/);
assert.match(floatingPanel, /v-if="clipboardHistoryItems\.length > clipboardItems\.length"/);
assert.match(floatingPanel, /MoreHorizontal/);
assert.match(floatingPanel, /openFloatingClipboardHistoryWindow/);
assert.match(floatingPanel, /openFloatingClipboardHistoryWindow\(\{\s*items:\s*clipboardHistoryItems\s*\}\)/);
assert.match(floatingPanel, /shouldShowFloatingClipboardItemMore/);
assert.match(floatingPanel, /data-floating-clipboard-item-more-button/);
assert.match(floatingPanel, /data-floating-clipboard-content/);
assert.match(
  floatingPanel,
  /data-floating-clipboard-content[\s\S]*?min-w-0[\s\S]*?flex-1[\s\S]*?overflow-hidden/,
);
assert.match(
  floatingPanel,
  /data-floating-clipboard-actions[\s\S]*?flex[\s\S]*?shrink-0/,
);
assert.match(
  floatingPanel,
  /data-floating-clipboard-link-button[\s\S]*?flex-1[\s\S]*?overflow-hidden/,
);
assert.match(floatingPanel, /selectedClipboardItem/);
assert.match(floatingPanel, /data-floating-clipboard-full-content/);
assert.doesNotMatch(floatingPanel, /showClipboardHistoryModal/);
assert.doesNotMatch(floatingPanel, /data-floating-clipboard-modal/);

assert.match(appShell, /const clipboardItems = computed\(\(\) =>\s*clipboardHistoryItems\.value\.slice\(0, FLOATING_CLIPBOARD_PREVIEW_LIMIT\),\s*\)/);
assert.doesNotMatch(appShell, /FLOATING_CLIPBOARD_MORE_LIMIT/);
assert.match(appShell, /FLOATING_CLIPBOARD_HISTORY_LIMIT/);
assert.match(appShell, /FLOATING_CLIPBOARD_PREVIEW_LIMIT/);
assert.match(appShell, /const clipboardHistoryItems = computed\(\(\) =>\s*getFloatingClipboardItems\(\s*systemClipboardItems\.value,\s*historyStore\.items,\s*FLOATING_CLIPBOARD_HISTORY_LIMIT,\s*\),\s*\)/);
assert.match(appShell, /updateFloatingClipboardHistoryWindow/);
assert.match(appShell, /watch\(\s*clipboardHistoryItems,/);
assert.match(appShell, /if \(!isFloating\.value\)/);
assert.match(appShell, /updateFloatingClipboardHistoryWindow\(\{\s*items,\s*\}\)/);

assert.match(tauri, /FLOATING_CLIPBOARD_WINDOW_LABEL/);
assert.match(tauri, /openFloatingClipboardHistoryWindow/);
assert.match(tauri, /updateFloatingClipboardHistoryWindow/);
assert.match(tauri, /FloatingClipboardHistoryPayload/);
assert.match(tauri, /FLOATING_CLIPBOARD_HISTORY_STORAGE_KEY/);
assert.match(tauri, /localStorage\.setItem\(FLOATING_CLIPBOARD_HISTORY_STORAGE_KEY/);
assert.match(tauri, /emitTo\(FLOATING_CLIPBOARD_WINDOW_LABEL,\s*"floating-clipboard-refresh",\s*payload\)/);
assert.match(tauri, /export async function updateFloatingClipboardHistoryWindow\(payload: FloatingClipboardHistoryPayload\): Promise<void>/);
assert.match(tauri, /const existing = await WebviewWindow\.getByLabel\(FLOATING_CLIPBOARD_WINDOW_LABEL\)/);
assert.match(tauri, /WebviewWindow/);
assert.match(tauri, /\/#\/floating-clipboard/);

assert.match(router, /FloatingClipboardHistory/);
assert.match(router, /path:\s*"\/floating-clipboard"/);
assert.match(app, /floating-clipboard/);

assert.ok(defaultCapability.windows.includes("floating-clipboard-history"));

assert.equal(existsSync(floatingClipboardWindowPath), true, "floating clipboard history window page must exist");
const floatingClipboardWindow = readFileSync(floatingClipboardWindowPath, "utf8");
const applyFloatingClipboardPayload = floatingClipboardWindow.match(
  /function applyFloatingClipboardPayload\(payload: FloatingClipboardHistoryPayload\) \{[\s\S]*?\n\}/,
)?.[0] ?? "";

assert.match(floatingClipboardWindow, /data-floating-clipboard-window/);
assert.match(floatingClipboardWindow, /floating-clipboard-history-surface/);
assert.match(floatingClipboardWindow, /data-window-drag-region/);
assert.match(floatingClipboardWindow, /startWindowDrag/);
assert.match(floatingClipboardWindow, /FloatingClipboardHistoryPayload/);
assert.match(floatingClipboardWindow, /applyFloatingClipboardPayload/);
assert.match(applyFloatingClipboardPayload, /resolveFloatingClipboardSelection/);
assert.doesNotMatch(applyFloatingClipboardPayload, /selectedClipboardItem\.value = null/);
assert.match(floatingClipboardWindow, /listen<FloatingClipboardHistoryPayload>/);
assert.match(floatingClipboardWindow, /localStorage\.getItem\(FLOATING_CLIPBOARD_HISTORY_STORAGE_KEY\)/);
assert.doesNotMatch(floatingClipboardWindow, /FLOATING_CLIPBOARD_MORE_LIMIT/);
assert.match(floatingClipboardWindow, /FLOATING_CLIPBOARD_HISTORY_LIMIT/);
assert.match(floatingClipboardWindow, /payload\.items\.slice\(0,\s*FLOATING_CLIPBOARD_HISTORY_LIMIT\)/);
assert.match(floatingClipboardWindow, /data-floating-clipboard-history-row/);
assert.match(floatingClipboardWindow, /data-floating-clipboard-history-content/);
assert.match(
  floatingClipboardWindow,
  /data-floating-clipboard-history-content[\s\S]*?min-w-0[\s\S]*?overflow-hidden/,
);
assert.match(
  floatingClipboardWindow,
  /data-floating-clipboard-actions[\s\S]*?shrink-0/,
);
assert.match(floatingClipboardWindow, /shouldShowFloatingClipboardHistoryItemMore/);
assert.match(floatingClipboardWindow, /data-floating-clipboard-item-more-button/);
assert.match(floatingClipboardWindow, /v-if="shouldShowFloatingClipboardHistoryItemMore\(item\)"/);
assert.match(floatingClipboardWindow, /@click\.stop="openFullClipboardItem\(item\)"/);
assert.match(floatingClipboardWindow, /selectedClipboardItem/);
assert.match(floatingClipboardWindow, /data-floating-clipboard-full-content/);
assert.match(floatingClipboardWindow, /content-type="text"/);

const stylesheet = readFileSync("src/style.css", "utf8");
assert.match(stylesheet, /--floating-clipboard-history-bg:/);
assert.match(stylesheet, /\.floating-clipboard-history-surface\s*\{/);
assert.match(stylesheet, /background:\s*var\(--floating-clipboard-history-bg\)/);
