import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const historyStore = readFileSync("src/stores/history.ts", "utf8");
const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");
const floatingHistory = readFileSync("src/pages/FloatingClipboardHistory.vue", "utf8");
const copyButton = readFileSync("src/components/ui/CopyTextButton.vue", "utf8");
const feedbackComponent = readFileSync(
  "src/components/history/ClipboardFileDownloadStatus.vue",
  "utf8",
);

assert.match(historyStore, /fileDownloads:/);
assert.match(historyStore, /"file-transfer-progress"/);
assert.match(historyStore, /"file-transfer-completed"/);
assert.match(historyStore, /"file-transfer-failed"/);
assert.match(historyStore, /status === "retrying"/);
assert.match(historyStore, /文件下载完成，已写入剪贴板/);
assert.match(historyStore, /文件下载失败/);

assert.match(feedbackComponent, /data-clipboard-file-download-status/);
assert.match(feedbackComponent, /data-clipboard-file-download-progress/);
assert.match(feedbackComponent, /inlineBadge/);
assert.doesNotMatch(feedbackComponent, /progressOnly/);
assert.match(feedbackComponent, /LoaderCircle/);
assert.match(feedbackComponent, /PauseCircle/);
assert.match(feedbackComponent, /feedback\.value\?\.percent/);

assert.match(clipboardPage, /ClipboardFileDownloadStatus/);
assert.match(clipboardPage, /historyStore\.isFileDownloadActive/);
assert.match(clipboardPage, /historyStore\.beginFileDownload/);
assert.match(clipboardPage, /openTransferFolder/);
assert.match(clipboardPage, /getClipboardFileCardAction/);
assert.match(clipboardPage, /resumeFileTransfer/);
assert.match(clipboardPage, /data-clipboard-file-summary/);
assert.match(clipboardPage, /clipboardFileNameClass/);
assert.match(clipboardPage, /clipboardFileNameClass[\s\S]*isClipboardFileCardInteractive/);
assert.match(clipboardPage, /data-clipboard-file-name[\s\S]*:class="clipboardFileNameClass\(item\)"/);
assert.match(clipboardPage, /hover:text-\[color:var\(--accent-text\)\]/);
assert.match(clipboardPage, /hover:underline/);
assert.match(clipboardPage, /gap-2\.5/);
assert.match(clipboardPage, /select-none/);
assert.match(clipboardPage, /data-clipboard-file-statuses/);
assert.match(clipboardPage, /inline-badge/);
assert.match(clipboardPage, /data-clipboard-history-sync-status/);
assert.doesNotMatch(clipboardPage, /progress-only/);
assert.doesNotMatch(clipboardPage, /v-if="item\.contentType !== 'fileList'"/);

assert.match(floatingPanel, /ClipboardFileDownloadStatus/);
assert.match(floatingPanel, /historyStore\.isFileDownloadActive/);
assert.match(floatingPanel, /openTransferFolder/);
assert.match(floatingPanel, /resumeFileTransfer/);
assert.match(floatingPanel, /data-floating-clipboard-file-summary/);
assert.match(floatingPanel, /compact/);

assert.match(copyButton, /fileTransferId/);
assert.match(copyButton, /fileDownloadActive/);
assert.match(copyButton, /LoaderCircle/);
assert.match(copyButton, /resumeFileTransfer/);

assert.match(floatingHistory, /CopyTextButton/);
assert.match(floatingHistory, /:file-transfer-status="item\.fileTransferStatus"/);

for (const surface of [clipboardPage, floatingPanel, copyButton]) {
  assert.match(surface, /toastStore\.success\("开始下载"\)/);
  assert.doesNotMatch(surface, /卡片会显示实时进度|列表会显示实时进度/);
}
