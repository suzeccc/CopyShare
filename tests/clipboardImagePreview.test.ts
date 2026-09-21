import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const imageThumb = readFileSync("src/components/history/HistoryImageThumb.vue", "utf8");
const directImagePreview = readFileSync("src/components/history/DirectImagePreview.vue", "utf8");
const historyModalStart = clipboardPage.indexOf("data-clipboard-history-modal");
const historyModalEnd = clipboardPage.indexOf("data-clipboard-video-preview-modal", historyModalStart);
const historyModal = clipboardPage.slice(historyModalStart, historyModalEnd);

assert.match(clipboardPage, /data-clipboard-image-summary/);
assert.match(clipboardPage, /data-clipboard-image-name/);
assert.match(clipboardPage, /data-clipboard-image-size/);
assert.match(clipboardPage, /clipboardFileSummary\(item\)\.name/);
assert.match(clipboardPage, /clipboardFileSummary\(item\)\.size/);
assert.match(clipboardPage, /previewImageItem/);
assert.match(clipboardPage, /openClipboardImagePreview/);
assert.match(clipboardPage, /data-clipboard-image-preview-button/);
assert.match(clipboardPage, /@click="openClipboardImagePreview\(item\)"/);
assert.match(clipboardPage, /<DirectImagePreview/);

assert.match(historyModal, /class="flex h-full max-h-full w-full max-w-4xl flex-col/);
assert.match(historyModal, /v-if="filteredAllClipboardItems\.length" class="min-h-0 flex-1/);
assert.match(historyModal, /v-else class="m-5 grid min-h-0 flex-1 place-items-center/);

assert.match(imageThumb, /variant\?: "thumb" \| "preview"/);
assert.match(imageThumb, /props\.variant === "preview"/);
assert.match(imageThumb, /"h-full w-full object-contain"/);
assert.doesNotMatch(imageThumb, /max-h-\[72vh\]|max-w-\[82vw\]/);
assert.match(imageThumb, /alt\?: string/);
assert.match(imageThumb, /:alt="props\.alt"/);

assert.match(directImagePreview, /data-direct-image-preview-fixed-canvas/);
assert.match(directImagePreview, /h-\[82vh\] w-\[90vw\] max-h-\[720px\] max-w-\[1200px\]/);
assert.match(directImagePreview, /data-direct-image-preview-surface/);
assert.match(directImagePreview, /class="grid h-full w-full touch-none/);
assert.match(directImagePreview, /variant="preview"/);
assert.match(directImagePreview, /!h-full !w-full/);
