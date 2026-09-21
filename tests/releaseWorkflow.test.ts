import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(".github/workflows/release.yml", "utf8");

test("release workflow builds Windows x64 and ARM64 as NSIS installers", () => {
  assert.match(workflow, /label: Windows x64[\s\S]*?platform: windows-latest[\s\S]*?rustTargets: x86_64-pc-windows-msvc[\s\S]*?args: --target x86_64-pc-windows-msvc --bundles nsis/);
  assert.match(workflow, /label: Windows ARM64[\s\S]*?platform: windows-latest[\s\S]*?rustTargets: aarch64-pc-windows-msvc[\s\S]*?args: --target aarch64-pc-windows-msvc --bundles nsis/);
  assert.match(workflow, /targets: \$\{\{ matrix\.rustTargets \}\}/);
  assert.doesNotMatch(workflow, /--bundles[^\n]*msi/i);
  assert.match(workflow, /- Windows x64 installer/);
  assert.match(workflow, /- Windows ARM64 installer/);
});

test("release workflow keeps both macOS builds and Linux", () => {
  assert.match(workflow, /label: macOS Apple Silicon/);
  assert.match(workflow, /label: macOS Intel/);
  assert.match(workflow, /label: Linux/);
  assert.match(workflow, /clang[\s\S]*?libleptonica-dev[\s\S]*?libtesseract-dev/);
});

test("updates use signed artifacts from this repository and only the main window", () => {
  const config = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
  const capability = JSON.parse(readFileSync("src-tauri/capabilities/updater.json", "utf8"));
  assert.equal(config.bundle.createUpdaterArtifacts, true);
  assert.ok(Buffer.from(config.plugins.updater.pubkey, "base64").toString().includes("minisign public key"));
  assert.deepEqual(config.plugins.updater.endpoints,
    ["https://github.com/suzeccc/CopyShare/releases/latest/download/latest.json"]);
  assert.equal(config.plugins.updater.windows.installMode, "passive");
  assert.deepEqual(capability.windows, ["main"]);
  assert.deepEqual(capability.permissions, ["updater:default"]);
  assert.match(workflow, /TAURI_SIGNING_PRIVATE_KEY: \$\{\{ secrets\.TAURI_SIGNING_PRIVATE_KEY \}\}/);
  assert.match(workflow, /uploadUpdaterJson: true/);
  assert.match(workflow, /uploadUpdaterSignatures: true/);
  assert.match(workflow, /updaterJsonPreferNsis: true/);
  assert.match(workflow, /releaseDraft: true/);
});
