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

test("library mutations surface prune failures after synchronizing state", () => {
  const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
  const mutation = commands.match(
    /async fn mutate_library[\s\S]*?\r?\n}\r?\n\r?\n#\[tauri::command\]/,
  )?.[0] ?? "";

  assert.match(
    mutation,
    /let prune_error = library::prune_library_resources\(&root, &next\)\.err\(\)/,
  );
  assert.match(
    mutation,
    /state\.replace_library\(snapshot\.clone\(\)\)\.await;[\s\S]*?if let Some\(error\) = prune_error \{[\s\S]*?return Err\(error\);/,
  );
  assert.doesNotMatch(mutation, /failed to prune library resources/);
});

test("library file-copy cache creation is serialized with mutations", () => {
  const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
  const copyCommand = commands.match(
    /pub async fn copy_library_item[\s\S]*?\r?\n}\r?\n\r?\n#\[tauri::command\]/,
  )?.[0] ?? "";

  assert.match(copyCommand, /let _guard = state\.lock_library_mutation\(\)\.await;/);
  assert.match(
    copyCommand,
    /lock_library_mutation\(\)\.await;[\s\S]*?\.library\(\)[\s\S]*?\.await/,
  );
  assert.match(copyCommand, /commit_file_copy_cache\(&root, &paths\)/);
  assert.match(copyCommand, /discard_file_copy_cache\(&paths\)/);
  assert.match(copyCommand, /clear_file_copy_cache\(&root\)/);
});
