import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(".github/workflows/release.yml", "utf8");

test("release workflow builds the five supported desktop packages", () => {
  assert.match(workflow, /label: Windows x64[\s\S]*?platform: windows-latest[\s\S]*?rustTargets: x86_64-pc-windows-msvc[\s\S]*?args: --target x86_64-pc-windows-msvc --bundles nsis/);
  assert.doesNotMatch(workflow, /Windows ARM64|aarch64-pc-windows-msvc/);
  assert.match(workflow, /targets: \$\{\{ matrix\.rustTargets \}\}/);
  assert.doesNotMatch(workflow, /--bundles[^\n]*msi/i);
  assert.match(workflow, /- Windows x64 NSIS installer/);
  assert.match(workflow, /args: --target aarch64-apple-darwin --bundles dmg/);
  assert.match(workflow, /args: --target x86_64-apple-darwin --bundles dmg/);
  assert.match(workflow, /args: --bundles appimage,deb/);
});

test("release workflow keeps both macOS builds and Linux", () => {
  assert.match(workflow, /label: macOS Apple Silicon/);
  assert.match(workflow, /label: macOS Intel/);
  assert.match(workflow, /label: Linux/);
  assert.match(workflow, /clang[\s\S]*?libleptonica-dev[\s\S]*?libtesseract-dev/);
});

test("release workflow keeps updater artifacts disabled and only the main window", () => {
  const config = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
  const capability = JSON.parse(readFileSync("src-tauri/capabilities/updater.json", "utf8"));
  assert.equal(config.bundle.createUpdaterArtifacts, false);
  assert.ok(Buffer.from(config.plugins.updater.pubkey, "base64").toString().includes("minisign public key"));
  assert.deepEqual(config.plugins.updater.endpoints,
    ["https://github.com/suzeccc/CopyShare/releases/latest/download/latest.json"]);
  assert.equal(config.plugins.updater.windows.installMode, "passive");
  assert.deepEqual(capability.windows, ["main"]);
  assert.deepEqual(capability.permissions, ["updater:default"]);
  assert.doesNotMatch(workflow, /TAURI_SIGNING_PRIVATE_KEY|uploadUpdaterJson|uploadUpdaterSignatures|updaterJsonPreferNsis/);
  assert.match(workflow, /releaseDraft: true/);
});
