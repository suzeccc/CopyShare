import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const tauri = readFileSync("src/lib/tauri.ts", "utf8");
const ocr = readFileSync("src-tauri/src/ocr.rs", "utf8");
const errors = readFileSync("src-tauri/src/error.rs", "utf8");
const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
const libRs = readFileSync("src-tauri/src/lib.rs", "utf8");
const cargo = readFileSync("src-tauri/Cargo.toml", "utf8");

assert.match(tauri, /import type \{ OcrResponse \} from "@\/types\/ocr"/);
assert.match(tauri, /export function recognizeClipboardImage\(\): Promise<OcrResponse>/);
assert.match(tauri, /invoke<OcrResponse>\("recognize_clipboard_image"\)/);

assert.match(errors, /Ocr\(String\)/);
assert.match(commands, /pub async fn recognize_clipboard_image/);
assert.match(commands, /clipboard::read_clipboard_image_base64/);
assert.match(commands, /spawn_blocking/);
assert.match(libRs, /mod ocr;/);
assert.match(libRs, /commands::recognize_clipboard_image/);

assert.match(ocr, /cfg\(target_os = "windows"\)/);
assert.match(ocr, /OcrEngine::TryCreateFromUserProfileLanguages/);
assert.match(ocr, /BitmapDecoder::CreateAsync/);
assert.match(ocr, /cfg\(not\(target_os = "windows"\)\)/);

for (const feature of [
  "Media_Ocr",
  "Graphics_Imaging",
  "Storage_Streams",
  "Globalization",
  "Win32_System_Com",
]) {
  assert.match(cargo, new RegExp(`"${feature}"`));
}
