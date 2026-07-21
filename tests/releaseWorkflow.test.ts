import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(".github/workflows/release.yml", "utf8");
const cargoToml = readFileSync("src-tauri/Cargo.toml", "utf8");
const cargoLock = readFileSync("src-tauri/Cargo.lock", "utf8");
const discovery = readFileSync("src-tauri/src/discovery.rs", "utf8");

test("network interface discovery uses an ARM64-compatible dependency", () => {
  assert.match(cargoToml, /^if-addrs = "0\.15"/m);
  assert.doesNotMatch(cargoToml, /^get_if_addrs\s*=/m);
  assert.match(discovery, /use if_addrs::IfAddr;/);
  assert.match(discovery, /if_addrs::get_if_addrs\(\)/);
  assert.doesNotMatch(
    cargoLock,
    /\[\[package\]\]\s+name = "get_if_addrs"\s+version = "0\.5\.3"/,
  );
  assert.doesNotMatch(
    cargoLock,
    /\[\[package\]\]\s+name = "winapi"\s+version = "0\.2\.8"/,
  );
});

test("release workflow builds Windows x64 and ARM64 as NSIS installers", () => {
  assert.match(workflow, /label: Windows x64[\s\S]*?platform: windows-latest[\s\S]*?rustTargets: x86_64-pc-windows-msvc[\s\S]*?args: --target x86_64-pc-windows-msvc --bundles nsis/);
  assert.match(workflow, /label: Windows ARM64[\s\S]*?platform: windows-latest[\s\S]*?rustTargets: aarch64-pc-windows-msvc[\s\S]*?args: --target aarch64-pc-windows-msvc --bundles nsis/);
  assert.match(workflow, /targets: \$\{\{ matrix\.rustTargets \}\}/);
  assert.doesNotMatch(workflow, /--bundles[^\n]*msi/i);
  assert.match(workflow, /- Windows x64 安装包/);
  assert.match(workflow, /- Windows ARM64 安装包/);
});

test("release workflow keeps both macOS builds and Linux", () => {
  assert.match(workflow, /label: macOS Apple Silicon/);
  assert.match(workflow, /label: macOS Intel/);
  assert.match(workflow, /label: Linux/);
});

test("release workflow creates the draft with Chinese update copy", () => {
  assert.match(workflow, /## 更新内容/);
  assert.match(workflow, /构建完成后将由发布脚本写入本版本的中文更新重点/);
  assert.doesNotMatch(workflow, /CopyShare desktop release\./);
});
