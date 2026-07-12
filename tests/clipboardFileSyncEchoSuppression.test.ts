import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const fileTransferBackend = readFileSync("src-tauri/src/file_transfer.rs", "utf8");

const functionStart = fileTransferBackend.indexOf("async fn apply_completed_clipboard_file_sync");
assert.notEqual(functionStart, -1, "apply_completed_clipboard_file_sync must exist");

const nextFunctionStart = fileTransferBackend.indexOf(
  "\nasync fn push_pending_clipboard_file_history",
  functionStart,
);
assert.notEqual(nextFunctionStart, -1, "next file-transfer helper must exist");

const applyCompletedClipboardFileSync = fileTransferBackend.slice(functionStart, nextFunctionStart);

const buildContentIndex = applyCompletedClipboardFileSync.indexOf(
  "let content = clipboard::file_paths_to_clipboard_content(&paths)?;",
);
const buildMessageIndex = applyCompletedClipboardFileSync.indexOf("let message = ClipboardMessage");
const markRemoteAppliedIndex = applyCompletedClipboardFileSync.indexOf(
  "state.mark_remote_clipboard_applied(&message).await;",
);
const writeClipboardIndex = applyCompletedClipboardFileSync.indexOf(
  "clipboard::write_clipboard_files(app, &paths)?;",
);

assert.notEqual(buildContentIndex, -1, "completed file sync must build canonical file-list content");
assert.notEqual(buildMessageIndex, -1, "completed file sync must build a remote clipboard message");
assert.notEqual(
  markRemoteAppliedIndex,
  -1,
  "completed file sync must register remote echo suppression before changing the clipboard",
);
assert.notEqual(writeClipboardIndex, -1, "completed file sync must write received files to clipboard");

assert.ok(
  buildContentIndex < buildMessageIndex,
  "file-list content must be canonicalized before building the echo-suppression message",
);
assert.ok(
  buildMessageIndex < markRemoteAppliedIndex,
  "the remote clipboard message must exist before registering echo suppression",
);
assert.ok(
  markRemoteAppliedIndex < writeClipboardIndex,
  "echo suppression must be registered before writing files to the clipboard",
);
