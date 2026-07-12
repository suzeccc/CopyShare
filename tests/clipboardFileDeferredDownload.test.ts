import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const tauriApi = readFileSync("src/lib/tauri.ts", "utf8");
const fileTransferBackend = readFileSync("src-tauri/src/file_transfer.rs", "utf8");
const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");
const copyButton = readFileSync("src/components/ui/CopyTextButton.vue", "utf8");
const historyPreview = readFileSync("src/lib/historyPreview.ts", "utf8");
const fileCardActions = readFileSync("src/lib/clipboardFileDownload.ts", "utf8");
const historyTypes = readFileSync("src/types/history.ts", "utf8");
const fileTransferTypes = readFileSync("src/types/fileTransfer.ts", "utf8");

assert.match(tauriApi, /type CopyHistoryResult = "copied" \| "downloadStarted" \| "downloading"/);
assert.match(tauriApi, /copyHistoryItem\(historyId: string\): Promise<CopyHistoryResult>/);

assert.match(historyTypes, /fileTransferId\?: string/);
assert.match(historyTypes, /fileTransferStatus\?: FileTransferStatus/);
assert.match(historyPreview, /direction:\s*item\.direction/);
assert.match(fileTransferTypes, /clipboardSync: boolean/);
assert.match(fileCardActions, /item\.direction === "local"/);
assert.match(fileCardActions, /return "openSourceLocation"/);

assert.match(clipboardPage, /handleClipboardItemClick/);
assert.match(clipboardPage, /item\.contentType !== "fileList"/);
assert.match(clipboardPage, /@click="handleClipboardItemClick\(item\)"/);
assert.match(clipboardPage, /isClipboardFileCardInteractive/);
assert.match(
  clipboardPage,
  /'cursor-wait': isClipboardFileCardInteractive\([\s\S]*historyStore\.isFileDownloadActive/,
);

assert.match(floatingPanel, /handleClipboardItemClick/);
assert.match(floatingPanel, /@click="handleFloatingClipboardItemClick\(item\)"/);
assert.match(floatingPanel, /isClipboardFileCardInteractive/);
assert.match(
  floatingPanel,
  /'cursor-wait': isFloatingClipboardItemInteractive\(item\) && historyStore\.isFileDownloadActive/,
);

assert.match(copyButton, /result\.value = await copyHistoryItem\(props\.historyItemId\)/);
assert.match(copyButton, /result\.value === "downloadStarted"/);
assert.match(copyButton, /result\.value === "downloading"/);

assert.doesNotMatch(fileTransferBackend, /AUTO_CLIPBOARD_FILE_SYNC_SIZE/);
assert.match(fileTransferBackend, /push_pending_clipboard_file_history/);
assert.match(fileTransferBackend, /copy_clipboard_file_history_item/);
