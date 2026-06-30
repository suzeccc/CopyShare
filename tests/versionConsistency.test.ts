import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const expectedVersion = "2.5.0";

const packageJson = JSON.parse(readFileSync("package.json", "utf8"));
const packageLock = JSON.parse(readFileSync("package-lock.json", "utf8"));
const tauriConfig = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const cargoToml = readFileSync("src-tauri/Cargo.toml", "utf8");
const cargoLock = readFileSync("src-tauri/Cargo.lock", "utf8");

assert.equal(packageJson.version, expectedVersion);
assert.equal(packageLock.version, expectedVersion);
assert.equal(packageLock.packages[""].version, expectedVersion);
assert.equal(tauriConfig.version, expectedVersion);
assert.match(cargoToml, /^version = "2\.5\.0"$/m);
assert.match(cargoLock, /\[\[package\]\]\s+name = "copyshare"\s+version = "2\.5\.0"/m);
