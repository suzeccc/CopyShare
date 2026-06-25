import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const packageJson = JSON.parse(readFileSync("package.json", "utf8"));

assert.equal(packageJson.scripts["build:exe"], "tauri build --no-bundle");
