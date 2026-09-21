import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const lib = readFileSync("src-tauri/src/lib.rs", "utf8");
const sync = readFileSync("src-tauri/src/sync.rs", "utf8");
const transfer = readFileSync("src-tauri/src/file_transfer.rs", "utf8");
const userGuide = readFileSync("docs/用户指南.md", "utf8");
const troubleshooting = readFileSync("docs/故障排查.md", "utf8");
const development = readFileSync("docs/开发指南.md", "utf8");
const testingAndBuild = readFileSync("docs/测试与构建.md", "utf8");
const architecture = readFileSync("docs/架构与扩展点.md", "utf8");
const protocol = readFileSync("docs/局域网协议.md", "utf8");
const readme = readFileSync("README.md", "utf8");
const readmeEnglish = readFileSync("README_EN.md", "utf8");

assert.match(lib, /mod safe_json_store;/);
assert.match(lib, /mod sync_engine;/);
assert.match(lib, /mod file_transfer_paths;/);
assert.match(sync, /pub use crate::sync_engine::\{content_hash, SyncEngine\};/);
assert.match(transfer, /file_transfer_paths::\{/);

assert.match(architecture, /## 2\. Rust 模块职责/);
assert.match(architecture, /## 4\. 数据与崩溃恢复/);
assert.match(architecture, /## 7\. 常见扩展点/);
assert.match(protocol, /UDP discovery/);
assert.match(protocol, /WebSocket/);
assert.match(protocol, /file-resume-v1/);
assert.match(protocol, /## 9\. 安全边界/);

assert.match(userGuide, /## 3\. 连接并信任设备/);
assert.match(userGuide, /## 5\. 文件传输/);
assert.match(troubleshooting, /## 3\. 搜不到设备/);
assert.match(troubleshooting, /## 6\. 文件下载失败、暂停或无法继续/);
assert.match(development, /## 3\. 项目目录/);
assert.match(development, /## 5\. 常见开发流程/);
assert.match(testingAndBuild, /## 3\. 完整质量门禁/);
assert.match(testingAndBuild, /## 5\. 本地主程序与安装包/);

for (const [label, fileName] of [
  ["用户指南", "用户指南"],
  ["故障排查", "故障排查"],
  ["开发指南", "开发指南"],
  ["测试与构建", "测试与构建"],
  ["架构与扩展点", "架构与扩展点"],
  ["局域网协议", "局域网协议"],
]) {
  assert.match(readme, new RegExp(`\\[${label}\\]\\(docs/${fileName}\\.md\\)`));
}

for (const [label, fileName] of [
  ["User guide", "用户指南"],
  ["Troubleshooting", "故障排查"],
  ["Development guide", "开发指南"],
  ["Testing and builds", "测试与构建"],
  ["Architecture and extension points", "架构与扩展点"],
  ["LAN protocol", "局域网协议"],
]) {
  assert.match(readmeEnglish, new RegExp(`\\[${label}\\]\\(docs/${fileName}\\.md\\)`));
}
