import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const tauri = readFileSync("src/lib/tauri.ts", "utf8");
const tauriConfig = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
const history = readFileSync("src-tauri/src/history.rs", "utf8");
const lib = readFileSync("src-tauri/src/lib.rs", "utf8");

assert.match(tauri, /convertFileSrc/);
assert.match(tauri, /getHistoryFilePreviewPath/);
assert.match(tauri, /invoke<string>\("get_history_file_preview_path"/);
assert.match(tauri, /convertLocalFileSrc/);

assert.match(commands, /pub async fn get_history_file_preview_path/);
assert.match(commands, /history::get_history_file_preview_path\(&item\)/);
assert.match(lib, /commands::get_history_file_preview_path/);

assert.match(history, /pub fn get_history_file_preview_path/);
assert.match(history, /视频文件不存在或尚未下载/);
assert.match(history, /is_video_file_path/);

assert.match(clipboardPage, /previewVideoItem/);
assert.match(clipboardPage, /previewVideoSrc/);
assert.match(clipboardPage, /openClipboardVideoPreview/);
assert.match(clipboardPage, /getHistoryFilePreviewPath/);
assert.match(clipboardPage, /convertLocalFileSrc/);
assert.match(clipboardPage, /data-clipboard-file-media-button/);
assert.match(clipboardPage, /@click\.stop="openClipboardVideoPreview\(item\)"/);
assert.match(clipboardPage, /data-clipboard-video-preview-modal/);
assert.match(clipboardPage, /<video[\s\S]*controls[\s\S]*autoplay/);
assert.match(clipboardPage, /无法预览视频/);
assert.match(clipboardPage, /previewVideoError/);
assert.match(clipboardPage, /handleClipboardVideoPreviewError/);
assert.match(clipboardPage, /@error="handleClipboardVideoPreviewError"/);
assert.match(clipboardPage, /preload="metadata"/);
assert.match(clipboardPage, /无法播放此视频/);

assert.equal(tauriConfig.app.security.assetProtocol.enable, true);
assert.deepEqual(tauriConfig.app.security.assetProtocol.scope, ["**"]);
