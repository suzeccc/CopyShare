import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const tauriConfig = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const configuredIcons: string[] = tauriConfig.bundle.icon;

assert.ok(existsSync("src-tauri/icons/icon.png"), "src-tauri/icons/icon.png must exist for Tauri generate_context on macOS/Linux");

for (const icon of configuredIcons) {
  assert.ok(existsSync(`src-tauri/${icon}`), `configured Tauri icon is missing: src-tauri/${icon}`);
}
