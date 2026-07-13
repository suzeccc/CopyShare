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
