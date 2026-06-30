import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import assert from "node:assert/strict";

const html = readFileSync(
  resolve("previews", "startup-loading-options.html"),
  "utf8",
);

const requiredText = [
  "CopyShare 启动动画本地预览",
  "方案一：链路唤醒",
  "方案二：剪贴板流光",
  "方案三：极简浮层",
  "适合场景",
  "优点",
  "取舍",
];

for (const text of requiredText) {
  assert.ok(html.includes(text), `missing required preview text: ${text}`);
}

for (const option of ["link-wakeup", "clipboard-flow", "minimal-overlay"]) {
  assert.ok(
    html.includes(`data-preview-option="${option}"`),
    `missing selectable preview marker: ${option}`,
  );
}

for (const keyframe of ["linkWake", "clipboardFlow", "minimalOverlay"]) {
  assert.ok(
    html.includes(`@keyframes ${keyframe}`),
    `missing animation keyframes: ${keyframe}`,
  );
}

assert.ok(
  html.includes("@media (prefers-reduced-motion: reduce)"),
  "preview should respect reduced motion",
);
assert.doesNotMatch(html, /https?:\/\//, "preview must not load network resources");
assert.doesNotMatch(html, /<(script|link)\b/i, "preview must be self-contained HTML/CSS");
