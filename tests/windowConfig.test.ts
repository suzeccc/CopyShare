import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const tauriConfig = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const defaultCapability = JSON.parse(
  readFileSync("src-tauri/capabilities/default.json", "utf8"),
);

assert.equal(tauriConfig.app.windows[0].label, "main");
assert.equal(tauriConfig.app.windows[0].center, true);
assert.equal(defaultCapability.permissions.includes("core:window:allow-center"), true);
