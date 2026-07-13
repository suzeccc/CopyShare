import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

test("library backend state and commands are registered", () => {
  const lib = readFileSync("src-tauri/src/lib.rs", "utf8");
  const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
  const models = readFileSync("src-tauri/src/models.rs", "utf8");
  const state = readFileSync("src-tauri/src/state.rs", "utf8");

  assert.match(lib, /mod library;/);
  for (const name of [
    "get_library",
    "collect_history_item",
    "create_text_snippet",
    "update_library_item",
    "convert_library_item_to_snippet",
    "set_library_item_pinned",
    "reorder_pinned_library_items",
    "remove_library_item",
    "copy_library_item",
    "get_library_storage_size",
    "get_library_image_thumbnail",
  ]) {
    assert.match(commands, new RegExp(`fn ${name}\\b`), `command ${name}`);
    assert.match(lib, new RegExp(`commands::${name}\\b`), `registered ${name}`);
  }
  for (const name of [
    "LibraryItem",
    "LibraryAssetRef",
    "LibrarySnapshot",
    "LibraryItemUpdate",
  ]) {
    assert.match(models, new RegExp(`struct ${name}\\b`));
  }
  assert.match(state, /library: RwLock<LibrarySnapshot>/);
  assert.match(state, /library_mutation: Mutex<\(\)>/);
  assert.match(state, /library::load_library\(app\)/);
});
