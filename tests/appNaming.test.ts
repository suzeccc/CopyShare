import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const packageJson = JSON.parse(readFileSync("package.json", "utf8"));
const cargoToml = readFileSync("src-tauri/Cargo.toml", "utf8");
const tauriConfig = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const mainRs = readFileSync("src-tauri/src/main.rs", "utf8");
const windowTitleBar = readFileSync("src/components/layout/WindowTitleBar.vue", "utf8");
const aboutPage = readFileSync("src/pages/About.vue", "utf8");
const configStore = readFileSync("src/stores/config.ts", "utf8");

assert.equal(packageJson.name, "copyshare");

assert.match(cargoToml, /^name = "copyshare"$/m);
assert.match(cargoToml, /^authors = \["CopyShare"\]$/m);
assert.match(cargoToml, /^name = "copyshare_lib"$/m);
assert.match(cargoToml, /^\[\[bin\]\]\s+name = "CopyShare"/m);

assert.equal(tauriConfig.productName, "CopyShare");
assert.equal(tauriConfig.app.windows[0].title, "CopyShare");

assert.match(mainRs, /copyshare_lib::run\(\)/);
assert.match(windowTitleBar, /CopyShare/);
assert.match(aboutPage, /CopyShare/);
assert.match(configStore, /deviceName: "CopyShare"/);
