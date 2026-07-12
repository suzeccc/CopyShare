import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
const history = readFileSync("src-tauri/src/history.rs", "utf8");
const transfer = readFileSync("src-tauri/src/file_transfer.rs", "utf8");
const lib = readFileSync("src-tauri/src/lib.rs", "utf8");
const tauri = readFileSync("src/lib/tauri.ts", "utf8");
const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");

assert.equal(existsSync("src/components/history/HistoryFileThumb.vue"), true);
const fileThumb = readFileSync("src/components/history/HistoryFileThumb.vue", "utf8");

assert.match(tauri, /getHistoryFileThumbnail/);
assert.match(tauri, /invoke<string>\("get_history_file_thumbnail"/);
assert.match(commands, /pub async fn get_history_file_thumbnail/);
assert.match(commands, /history::get_history_file_thumbnail\(&app, &item, max_size\)/);
assert.match(lib, /commands::get_history_file_thumbnail/);

assert.match(history, /pub fn get_history_file_thumbnail/);
assert.match(history, /fn is_video_file_path/);
assert.match(history, /video_thumbnail_path/);
assert.match(history, /thumbnail_from_windows_shell/);
assert.match(history, /\$ErrorActionPreference = "Stop"/);
assert.match(history, /Add-Type -ReferencedAssemblies System\.Drawing -TypeDefinition/);
assert.match(history, /SIIGBF\.BiggerSizeOk \| SIIGBF\.ThumbnailOnly/);
assert.match(history, /ClipboardContentType::FileList/);
assert.match(transfer, /pub fn file_path_from_history_item/);

assert.match(fileThumb, /data-history-file-thumb/);
assert.match(fileThumb, /getHistoryFileThumbnail/);
assert.match(fileThumb, /Video/);
assert.match(fileThumb, /data:image\/png;base64/);
assert.doesNotMatch(fileThumb, /v-if="isVideo && !failed"/);
assert.match(fileThumb, /v-if="isVideo"/);
assert.match(fileThumb, /data-history-file-thumb-placeholder/);

assert.match(clipboardPage, /HistoryFileThumb/);
assert.match(clipboardPage, /data-clipboard-file-media/);
assert.match(clipboardPage, /<HistoryFileThumb[\s\S]*:history-id="item\.id"/);
assert.match(floatingPanel, /HistoryFileThumb/);
assert.match(floatingPanel, /<HistoryFileThumb[\s\S]*:history-id="item\.id"/);
