import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

test("history and cache clearing never mutate the library", () => {
  const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
  const clearHistory = commands.match(/pub async fn clear_history[\s\S]*?\n}\n/)?.[0] ?? "";
  assert.match(clearHistory, /history::clear_history\(&app\)/);
  assert.match(clearHistory, /state\.replace_history\(Vec::new\(\)\)\.await/);
  assert.doesNotMatch(clearHistory, /library|prune_assets|remove_library_item/);

  const clearCache = commands.match(/pub async fn clear_cache[\s\S]*?\n}\n/)?.[0] ?? "";
  assert.match(clearCache, /history::clear_cache\(&app\)/);
  assert.doesNotMatch(clearCache, /library|library-assets|library-thumbnails/);
});

test("history pinning persists history without mutating the library", () => {
  const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
  const pinHistory = commands.match(/pub async fn set_history_item_pinned[\s\S]*?\n}\n/)?.[0] ?? "";

  assert.match(pinHistory, /history::set_history_item_pinned/);
  assert.match(pinHistory, /history::save_history/);
  assert.match(pinHistory, /history-updated/);
  assert.doesNotMatch(pinHistory, /mutate_library|collect_history_item|set_library_item_pinned/);
});
