import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { adjacentMediaPreviewItem, mediaPreviewItems, nextVideoPlaybackRate } from "../src/lib/mediaPreviewControls.ts";
import type { ClipboardPreviewItem } from "../src/lib/historyPreview.ts";

const items: ClipboardPreviewItem[] = [
  { id: "image-a", text: "a.png", contentType: "image", contentHash: "", syncStatus: "synced" },
  { id: "text", text: "note", contentType: "text", contentHash: "", syncStatus: "synced" },
  { id: "video-local", text: "a.mp4 2 MB", contentType: "fileList", contentHash: "", syncStatus: "synced", direction: "local" },
  { id: "video-pending", text: "b.mp4 2 MB", contentType: "fileList", contentHash: "", syncStatus: "synced", direction: "remote", fileTransferStatus: "pending" },
  { id: "image-b", text: "b.png", contentType: "image", contentHash: "", syncStatus: "synced" },
  { id: "video-downloaded", text: "c.webm 2 MB", contentType: "fileList", contentHash: "", syncStatus: "synced", direction: "remote", fileTransferStatus: "completed" },
];
const images = mediaPreviewItems(items, "image");
assert.deepEqual(images.map(item => item.id), ["image-a", "image-b"]);
assert.equal(adjacentMediaPreviewItem(images, "image-a", -1), undefined);
assert.equal(adjacentMediaPreviewItem(images, "image-a", 1)?.id, "image-b");
assert.equal(adjacentMediaPreviewItem(images, "image-b", 1), undefined);
assert.equal(adjacentMediaPreviewItem(images, "deleted", 1), undefined);
assert.deepEqual(mediaPreviewItems(items, "video").map(item => item.id), ["video-local", "video-downloaded"]);
assert.equal(nextVideoPlaybackRate(1, 1), 1.25);
assert.equal(nextVideoPlaybackRate(1, -1), 0.75);
assert.equal(nextVideoPlaybackRate(0.25, -1), 0.25);
assert.equal(nextVideoPlaybackRate(3, 1), 3);

const toolbar = readFileSync("src/components/history/MediaPreviewToolbar.vue", "utf8");
assert.match(toolbar, /await saveHistoryMedia\(props\.historyId\)/);
assert.match(toolbar, /props\.fullscreenTarget\.requestFullscreen\(\)/);
assert.match(readFileSync("src/lib/tauri.ts", "utf8"), /invoke<boolean>\("save_history_media", \{ historyId \}\)/);
assert.match(readFileSync("src-tauri/src/lib.rs", "utf8"), /commands::save_history_media/);
for (const file of ["src/components/history/DirectImagePreview.vue", "src/pages/Clipboard.vue", "src/pages/MediaPreview.vue"]) {
  assert.match(readFileSync(file, "utf8"), /<MediaPreviewToolbar/);
}
