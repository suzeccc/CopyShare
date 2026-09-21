import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import ts from "typescript";
import {
  getNextMediaPreviewImageScale,
  MEDIA_PREVIEW_IMAGE_MIN_SCALE,
} from "../src/lib/mediaPreviewImagePanZoom.ts";
import { getMediaPreviewWindowPosition } from "../src/lib/mediaPreviewWindow.ts";
import { splitClipboardFileSummary } from "../src/lib/historyPreview.ts";

const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");
const floatingHistory = readFileSync("src/pages/FloatingClipboardHistory.vue", "utf8");
const historyPreview = readFileSync("src/lib/historyPreview.ts", "utf8");
const tauri = readFileSync("src/lib/tauri.ts", "utf8");
const router = readFileSync("src/router/index.ts", "utf8");
const mediaPreview = readFileSync("src/pages/MediaPreview.vue", "utf8");
const defaultCapability = JSON.parse(
  readFileSync("src-tauri/capabilities/default.json", "utf8"),
);

assert.match(historyPreview, /FLOATING_CLIPBOARD_PREVIEW_LIMIT = 20/);
assert.match(historyPreview, /FLOATING_CLIPBOARD_HISTORY_LIMIT = 100/);

assert.match(tauri, /MEDIA_PREVIEW_WINDOW_LABEL/);
assert.match(tauri, /openMediaPreviewWindow/);
assert.match(tauri, /emitTo\(MEDIA_PREVIEW_WINDOW_LABEL,\s*"media-preview-open"/);
assert.match(tauri, /kind: "image" \| "video"/);
assert.match(tauri, /kind: payload\.kind/);

assert.match(router, /path:\s*"\/media-preview"/);
assert.match(mediaPreview, /data-media-preview-video/);
assert.match(mediaPreview, /media-preview-open/);
assert.match(mediaPreview, /data-media-preview-image/);
assert.match(mediaPreview, /<DirectImagePreview[^>]*embedded/);

assert.match(floatingPanel, /<HistoryImageThumb/);
assert.match(floatingPanel, /openFloatingVideoPreview/);
assert.match(floatingPanel, /@click\.stop="openFloatingVideoPreview\(item\)"/);
assert.doesNotMatch(floatingPanel, /DirectImagePreview|previewImageItem/);
assert.match(floatingPanel, /openFloatingImagePreview/);
assert.match(floatingPanel, /kind: "image", historyId: item\.id/);

assert.match(floatingHistory, /<HistoryImageThumb/);
assert.match(floatingHistory, /openHistoryVideoPreview/);
assert.match(floatingHistory, /@click="openHistoryVideoPreview\(item\)"/);
assert.doesNotMatch(floatingHistory, /DirectImagePreview|previewImageItem/);
assert.match(floatingHistory, /openHistoryImagePreview/);
assert.match(floatingHistory, /kind: "image", historyId: item\.id/);

assert.equal(existsSync("src/components/history/DirectImagePreview.vue"), true);
assert.equal(existsSync("src/lib/mediaPreviewImagePanZoom.ts"), true);
assert.equal(getNextMediaPreviewImageScale(1, 120), 0.85);
assert.equal(
  getNextMediaPreviewImageScale(MEDIA_PREVIEW_IMAGE_MIN_SCALE, 120),
  MEDIA_PREVIEW_IMAGE_MIN_SCALE,
);

assert.ok(defaultCapability.windows.includes("media-preview"));
assert.ok(defaultCapability.permissions.includes("core:webview:allow-create-webview-window"));
assert.ok(defaultCapability.permissions.includes("core:window:allow-set-position"));

// Run the actual window opener: images and videos share one independent window.
const windowFunctions = [
  tauri.match(/function mediaPreviewUrl\([\s\S]*?\n\}/)?.[0],
  tauri.match(/export async function openMediaPreviewWindow\([\s\S]*?\n\}/)?.[0]?.replace("export ", ""),
].join("\n");
let existingWindow: null | { show(): Promise<void>; setFocus(): Promise<void> } = null;
const created: Array<{ label: string; options: { url: string; width: number; transparent: boolean; backgroundColor: number[]; windowEffects: { effects: string[] }; shadow: boolean; resizable: boolean } }> = [];
const events: Array<{ label: string; event: string; payload: unknown }> = [];
const storage = new Map<string, string>();
let shown = 0;
let focused = 0;
const openPreview = new Function("WebviewWindow", "window", "emitTo", "mediaPreviewInitialPosition", "translateSource",
  ts.transpile(`const MEDIA_PREVIEW_WINDOW_LABEL="media-preview";
    const MEDIA_PREVIEW_ITEMS_STORAGE_KEY="copyshare:media-preview-items";
    const MEDIA_PREVIEW_WINDOW_BOUNDS={width:720,height:520};
    const TRANSPARENT_WINDOW_BACKGROUND=[0,0,0,0];
    const Effect={Acrylic:"acrylic",HudWindow:"hudWindow"};
    ${windowFunctions}\nreturn openMediaPreviewWindow;`, { target: ts.ScriptTarget.ES2022 }),
)(class {
  static async getByLabel() { return existingWindow; }
  constructor(label: string, options: (typeof created)[number]["options"]) { created.push({ label, options }); }
}, { localStorage: { setItem(key: string, value: string) { storage.set(key, value); } } },
  async (label: string, event: string, payload: unknown) => { events.push({ label, event, payload }); },
  async () => ({ x: 100, y: 200 }), (value: string) => value);
const imagePayload = { kind: "image", historyId: "image-a", title: "A & B.png", src: "", items: [{ id: "image-a", contentType: "image" }] };
await openPreview(imagePayload);
assert.equal(created.length, 1);
assert.equal(created[0].label, "media-preview");
assert.equal(created[0].options.transparent, true);
assert.deepEqual(created[0].options.backgroundColor, [0,0,0,0]);
assert.deepEqual(created[0].options.windowEffects.effects, ["acrylic","hudWindow"]);
assert.equal(created[0].options.shadow, true);
assert.equal(created[0].options.resizable, true);
const params = new URLSearchParams(created[0].options.url.split("?")[1]);
assert.equal(params.get("kind"), "image");
assert.equal(params.get("historyId"), "image-a");
assert.equal(params.get("title"), "A & B.png");
existingWindow = { async show() { shown++; }, async setFocus() { focused++; } };
const videoPayload = { kind: "video", historyId: "video-a", title: "A.mp4", src: "asset://A.mp4", items: [] };
await openPreview(videoPayload);
assert.equal(created.length, 1);
assert.deepEqual(events[0], { label: "media-preview", event: "media-preview-open", payload: videoPayload });
assert.equal(shown, 1);
assert.equal(focused, 1);
assert.deepEqual(JSON.parse(storage.get("copyshare:media-preview-items")!), []);

// Location actions and the title must follow the image selected inside the preview.
const imageChange = mediaPreview.match(/function handleImageChange\([\s\S]*?\n\}/)?.[0];
assert.ok(imageChange);
const selectedId = { value: "image-a" };
const selectedTitle = { value: "A.png" };
const selectImage = new Function("historyId", "title", "playlist", "splitClipboardFileSummary",
  ts.transpile(imageChange) + ";return handleImageChange;",
)(selectedId, selectedTitle, { value: [{ id: "image-b", text: "B with spaces.png 2 KB" }] }, splitClipboardFileSummary);
selectImage("image-b");
assert.equal(selectedId.value, "image-b");
assert.equal(selectedTitle.value, "B with spaces.png");

const monitor = { x: 0, y: 0, width: 1600, height: 900 };

assert.deepEqual(
  getMediaPreviewWindowPosition({
    floating: { x: 860, y: 160, width: 340, height: 320 },
    monitor,
    preview: { width: 720, height: 520, offset: 14 },
  }),
  { x: 126, y: 160 },
);

assert.deepEqual(
  getMediaPreviewWindowPosition({
    floating: { x: 80, y: 160, width: 340, height: 320 },
    monitor,
    preview: { width: 720, height: 520, offset: 14 },
  }),
  { x: 434, y: 160 },
);

assert.deepEqual(
  getMediaPreviewWindowPosition({
    floating: { x: 500, y: 700, width: 340, height: 320 },
    monitor,
    preview: { width: 720, height: 520, offset: 14 },
  }),
  { x: 854, y: 380 },
);
