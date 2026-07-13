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
});
