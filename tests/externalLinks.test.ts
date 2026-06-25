import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const tauriLib = readFileSync("src/lib/tauri.ts", "utf8");
const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
const rustLib = readFileSync("src-tauri/src/lib.rs", "utf8");

assert.match(tauriLib, /export function openExternalUrl\(url: string\)/);
assert.match(tauriLib, /invoke<void>\("open_external_url"/);
assert.match(commands, /pub async fn open_external_url/);
assert.match(commands, /Url::parse/);
assert.match(commands, /InvalidInput/);
assert.match(rustLib, /commands::open_external_url/);