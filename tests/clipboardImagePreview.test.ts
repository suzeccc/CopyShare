import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const imageThumb = readFileSync("src/components/history/HistoryImageThumb.vue", "utf8");

assert.match(clipboardPage, /previewImageItem/);
assert.match(clipboardPage, /previewImageOffset/);
assert.match(clipboardPage, /previewImageDrag/);
assert.match(clipboardPage, /openClipboardImagePreview/);
assert.match(clipboardPage, /closeClipboardImagePreview/);
assert.match(clipboardPage, /startClipboardImageDrag/);
assert.match(clipboardPage, /moveClipboardImageDrag/);
assert.match(clipboardPage, /endClipboardImageDrag/);
assert.match(clipboardPage, /data-clipboard-image-preview-button/);
assert.match(clipboardPage, /@click="openClipboardImagePreview\(item\)"/);
assert.match(clipboardPage, /data-clipboard-image-preview-modal/);
assert.match(clipboardPage, /@click\.self="closeClipboardImagePreview"/);
assert.match(clipboardPage, /data-clipboard-image-preview-close/);
assert.match(clipboardPage, /@pointerdown="startClipboardImageDrag"/);
assert.match(clipboardPage, /@pointermove="moveClipboardImageDrag"/);
assert.match(clipboardPage, /@pointerup="endClipboardImageDrag"/);
assert.match(clipboardPage, /@pointercancel="endClipboardImageDrag"/);
assert.match(clipboardPage, /data-clipboard-image-preview-zoom-area/);
assert.match(clipboardPage, /cursor-grab/);
assert.match(clipboardPage, /cursor-grabbing/);
assert.match(clipboardPage, /variant="preview"/);
assert.match(clipboardPage, /:max-size="1400"/);
assert.match(clipboardPage, /previewImageItem\.id/);
assert.match(clipboardPage, /translate\(\$\{previewImageOffset\.x\.value\}px, \$\{previewImageOffset\.y\.value\}px\) scale\(\$\{previewImageScale\.value\}\)/);

assert.match(imageThumb, /variant\?: "thumb" \| "preview"/);
assert.match(imageThumb, /rootClass/);
assert.match(imageThumb, /props\.variant === "preview"/);
assert.match(imageThumb, /max-h-\[72vh\]/);
assert.match(imageThumb, /object-contain/);
assert.match(imageThumb, /alt\?: string/);
assert.match(imageThumb, /:alt="props\.alt"/);
