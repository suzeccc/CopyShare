import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const tauri = readFileSync("src/lib/tauri.ts", "utf8");
const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
const lib = readFileSync("src-tauri/src/lib.rs", "utf8");
const transfer = readFileSync("src-tauri/src/file_transfer.rs", "utf8");
const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");

assert.match(tauri, /openHistoryFileLocation/);
assert.match(tauri, /invoke<void>\("open_history_file_location", \{ historyId \}\)/);
assert.match(commands, /pub async fn open_history_file_location/);
assert.match(commands, /file_transfer::open_history_file_location\(&item\)/);
assert.match(lib, /commands::open_history_file_location/);
assert.match(transfer, /source_file_path_from_history_item/);
assert.match(transfer, /clipboard_content_to_file_paths/);
assert.match(transfer, /reveal_path_with_system_file_manager/);
assert.match(clipboardPage, /openHistoryFileLocation/);
assert.match(clipboardPage, /action === "openSourceLocation"/);
assert.match(clipboardPage, /await openHistoryFileLocation\(item\.id\)/);
assert.match(floatingPanel, /openHistoryFileLocation/);
assert.match(floatingPanel, /action === "openSourceLocation"/);
assert.match(floatingPanel, /await openHistoryFileLocation\(item\.id\)/);
