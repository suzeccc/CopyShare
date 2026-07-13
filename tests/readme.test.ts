import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const readme = readFileSync("README.md", "utf8");

test("README exposes current release platforms and download entry", () => {
  assert.match(readme, /img\.shields\.io\/github\/v\/release\/suzeccc\/CopyShare/);
  assert.match(readme, /github\.com\/suzeccc\/CopyShare\/releases\/latest/);
  assert.doesNotMatch(readme, /v3\.0\.0/);
  for (const label of [
    "Windows x64",
    "Windows ARM64",
    "macOS Apple Silicon",
    "macOS Intel",
    "Linux",
  ]) assert.match(readme, new RegExp(label));
});

test("README describes the complete user-facing feature set", () => {
  for (const feature of [
    "视频预览",
    "桌面浮窗",
    "常用片段",
    "收藏夹",
    "网格",
    "列表",
    "图片转文字",
    "仅支持 Windows",
    "二维码",
    "重复同步内容",
    "桌面通知",
    "缓存管理",
    "托盘",
    "检查更新",
  ]) assert.match(readme, new RegExp(feature));
});

test("README states privacy boundaries and keeps valid screenshots", () => {
  assert.match(readme, /翻译文本会发送到.*翻译服务/);
  assert.match(readme, /剪贴板.*不会上传.*CopyShare.*云端/);
  for (const image of ["1.png", "2-1.png", "2-2.png", "3.png", "4.png"]) {
    assert.equal(existsSync(`docs/images/${image}`), true, image);
    assert.match(readme, new RegExp(`docs/images/${image.replace(".", "\\.")}`));
  }
});
