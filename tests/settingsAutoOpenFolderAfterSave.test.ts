import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const settings = readFileSync("src/pages/Settings.vue", "utf8");
const configTypes = readFileSync("src/types/config.ts", "utf8");
const configStore = readFileSync("src/stores/config.ts", "utf8");
const models = readFileSync("src-tauri/src/models.rs", "utf8");
const fileTransfer = readFileSync("src-tauri/src/file_transfer.rs", "utf8");

assert.match(configTypes, /autoOpenFolderAfterSave:\s*boolean/);
assert.match(configStore, /autoOpenFolderAfterSave:\s*false/);
assert.match(models, /pub auto_open_folder_after_save:\s*bool/);
assert.match(models, /auto_open_folder_after_save:\s*false/);

assert.match(settings, /autoOpenFolderAfterSave/);
assert.match(settings, /saveAutoOpenFolderAfterSave/);
assert.match(settings, /\u81ea\u52a8\u6253\u5f00\u6587\u4ef6\u5939/);
assert.match(settings, /<Switch[\s\S]*:model-value="draft\.autoOpenFolderAfterSave"[\s\S]*@update:model-value="saveAutoOpenFolderAfterSave"/);
assert.doesNotMatch(settings, /<span class="text-\[15px\] font-bold text-white">\u6587\u4ef6\u4fdd\u5b58\u540e\u64cd\u4f5c<\/span>[\s\S]*\u6253\u5f00\u6587\u4ef6\u5939[\s\S]*\u6062\u590d\u9ed8\u8ba4/);

assert.match(fileTransfer, /maybe_open_folder_after_save/);
assert.match(fileTransfer, /config\.auto_open_folder_after_save/);
assert.match(fileTransfer, /open_folder_with_system_file_manager\(save_dir\)/);
