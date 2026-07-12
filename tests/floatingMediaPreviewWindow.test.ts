import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import {
  getNextMediaPreviewImageOffset,
  getNextMediaPreviewImageScale,
  MEDIA_PREVIEW_IMAGE_MAX_SCALE,
  MEDIA_PREVIEW_IMAGE_MIN_SCALE,
  shouldPanMediaPreviewImage,
} from "../src/lib/mediaPreviewImagePanZoom.ts";
import { getMediaPreviewWindowPosition } from "../src/lib/mediaPreviewWindow.ts";

const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");
const historyPreview = readFileSync("src/lib/historyPreview.ts", "utf8");
const tauri = readFileSync("src/lib/tauri.ts", "utf8");
const router = readFileSync("src/router/index.ts", "utf8");
const defaultCapability = JSON.parse(
  readFileSync("src-tauri/capabilities/default.json", "utf8"),
);
const mediaPreviewPath = "src/pages/MediaPreview.vue";

assert.match(historyPreview, /FLOATING_CLIPBOARD_PREVIEW_LIMIT = 10/);
assert.match(historyPreview, /FLOATING_CLIPBOARD_HISTORY_LIMIT = 50/);

assert.match(tauri, /WebviewWindow/);
assert.match(tauri, /MEDIA_PREVIEW_WINDOW_LABEL/);
assert.match(tauri, /openMediaPreviewWindow/);
assert.match(tauri, /emitTo\(MEDIA_PREVIEW_WINDOW_LABEL,\s*"media-preview-open"/);
assert.match(tauri, /getByLabel\(MEDIA_PREVIEW_WINDOW_LABEL\)/);
assert.match(tauri, /LogicalPosition/);
assert.match(tauri, /MEDIA_PREVIEW_WINDOW_BOUNDS/);
assert.match(tauri, /currentMonitor/);
assert.match(tauri, /getMediaPreviewWindowPosition/);

assert.match(router, /MediaPreview/);
assert.match(router, /path:\s*"\/media-preview"/);

assert.equal(existsSync(mediaPreviewPath), true, "media preview page must exist");
const mediaPreview = readFileSync(mediaPreviewPath, "utf8");

assert.match(mediaPreview, /onMounted/);
assert.match(mediaPreview, /onUnmounted/);
assert.match(mediaPreview, /UnlistenFn/);
assert.match(mediaPreview, /media-preview-open/);
assert.match(mediaPreview, /mediaPreviewUnlisten\?\.\(\)/);
assert.match(mediaPreview, /getConfig/);
assert.match(mediaPreview, /config-updated/);
assert.match(mediaPreview, /document\.documentElement\.dataset\.appTheme/);
assert.match(mediaPreview, /document\.body\.dataset\.appTheme/);
assert.match(mediaPreview, /themeUnlisten\?\.\(\)/);
assert.match(mediaPreview, /startWindowDrag/);
assert.match(mediaPreview, /minimizeWindow/);
assert.match(mediaPreview, /imagePreviewScale/);
assert.match(mediaPreview, /imagePreviewOffset/);
assert.match(mediaPreview, /imagePreviewDragPointerId/);
assert.match(mediaPreview, /imagePreviewTransformStyle/);
assert.match(mediaPreview, /handleImagePreviewWheel/);
assert.match(mediaPreview, /handleImagePreviewDragPress/);
assert.match(mediaPreview, /handleImagePreviewDragMove/);
assert.match(mediaPreview, /finishImagePreviewDrag/);
assert.doesNotMatch(mediaPreview, /window\.setTimeout/);
assert.match(mediaPreview, /data-media-preview-window/);
assert.match(mediaPreview, /data-media-preview-minimize-button/);
assert.match(mediaPreview, /@click="minimizeWindow"/);
assert.match(mediaPreview, /data-media-preview-image/);
assert.match(mediaPreview, /data-media-preview-image-drag-surface/);
assert.match(mediaPreview, /data-media-preview-video/);
assert.match(mediaPreview, /@wheel\.prevent="handleImagePreviewWheel"/);
assert.match(mediaPreview, /@pointerdown\.left="handleImagePreviewDragPress"/);
assert.match(mediaPreview, /@pointermove="handleImagePreviewDragMove"/);
assert.match(mediaPreview, /@pointerup="finishImagePreviewDrag"/);
assert.match(mediaPreview, /@pointercancel="finishImagePreviewDrag"/);
assert.match(mediaPreview, /setPointerCapture\(event\.pointerId\)/);
assert.match(mediaPreview, /releasePointerCapture\(event\.pointerId\)/);
assert.match(mediaPreview, /@error="handleVideoPreviewError"/);

assert.match(floatingPanel, /openMediaPreviewWindow/);
assert.match(floatingPanel, /openFloatingImagePreview/);
assert.match(floatingPanel, /openFloatingVideoPreview/);
assert.match(floatingPanel, /isClipboardVideoFile/);
assert.match(floatingPanel, /data-floating-media-preview-button/);
assert.match(floatingPanel, /floating-clipboard-row/);
assert.match(floatingPanel, /floating-link-chip/);
assert.match(floatingPanel, /@click\.stop="openFloatingImagePreview\(item\)"/);
assert.match(floatingPanel, /@click\.stop="openFloatingVideoPreview\(item\)"/);

assert.ok(defaultCapability.windows.includes("media-preview"));
assert.ok(defaultCapability.permissions.includes("core:webview:allow-create-webview-window"));
assert.ok(defaultCapability.permissions.includes("core:window:allow-set-position"));

const monitor = { x: 0, y: 0, width: 1600, height: 900 };

assert.deepEqual(
  getMediaPreviewWindowPosition({
    floating: { x: 860, y: 160, width: 340, height: 320 },
    monitor,
    preview: { width: 720, height: 520, offset: 14 },
  }),
  { x: 126, y: 160 },
  "floating window near the right edge should place preview on the left",
);

assert.deepEqual(
  getMediaPreviewWindowPosition({
    floating: { x: 80, y: 160, width: 340, height: 320 },
    monitor,
    preview: { width: 720, height: 520, offset: 14 },
  }),
  { x: 434, y: 160 },
  "floating window near the left edge should place preview on the right",
);

assert.deepEqual(
  getMediaPreviewWindowPosition({
    floating: { x: 500, y: 700, width: 340, height: 320 },
    monitor,
    preview: { width: 720, height: 520, offset: 14 },
  }),
  { x: 854, y: 380 },
  "preview should be clamped inside the monitor work area",
);

assert.equal(
  getNextMediaPreviewImageScale(MEDIA_PREVIEW_IMAGE_MIN_SCALE, -120),
  1.15,
  "wheel up should zoom in",
);

assert.equal(
  getNextMediaPreviewImageScale(1.15, 120),
  MEDIA_PREVIEW_IMAGE_MIN_SCALE,
  "wheel down should zoom out",
);

assert.equal(
  getNextMediaPreviewImageScale(MEDIA_PREVIEW_IMAGE_MAX_SCALE, -120),
  MEDIA_PREVIEW_IMAGE_MAX_SCALE,
  "zoom should clamp to max scale",
);

assert.deepEqual(
  getNextMediaPreviewImageOffset(
    { x: 12, y: -4 },
    { x: 100, y: 100 },
    { x: 126, y: 82 },
  ),
  { x: 38, y: -22 },
  "dragging should pan from the press origin",
);

assert.equal(shouldPanMediaPreviewImage(0.99), false);
assert.equal(shouldPanMediaPreviewImage(1), true);
assert.equal(shouldPanMediaPreviewImage(1.01), true);
