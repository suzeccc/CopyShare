import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const app = readFileSync("src/App.vue", "utf8");
const style = readFileSync("src/style.css", "utf8");

assert.match(app, /const STARTUP_OVERLAY_MIN_MS = 900;/);
assert.match(app, /function isUtilityWindowStartupBypassed\(\)/);
assert.match(app, /#\/media-preview/);
assert.match(app, /#\/floating-clipboard/);
assert.match(app, /const startupVisible = ref\(!isUtilityWindowStartupBypassed\(\)\);/);
assert.match(app, /v-if="startupAnimationVisible && !isMediaPreviewRoute"/);
assert.match(app, /:startup-animation-complete="startupAnimationComplete" @ready="resolveStartupWindow"/);
assert.match(app, /performance\.now\(\)/);
assert.match(app, /Math\.max\(STARTUP_OVERLAY_MIN_MS - elapsed, 0\)/);
assert.match(app, /startupVisible\.value = false;/);

assert.match(app, /<Transition name="startup-overlay" @after-leave="resolveStartupAnimation">/);
assert.match(app, /data-startup-overlay/);
assert.match(app, /aria-live="polite"/);
assert.match(app, /CopyShare/);
assert.match(app, /正在准备同步/);
assert.match(app, /startup-logo-link/);
assert.match(app, /startup-progress/);

assert.match(style, /\.startup-overlay \{/);
assert.match(
  style,
  /\.startup-overlay \{[^}]*border-radius: 18px;[^}]*overflow: hidden;[^}]*\}/,
);
assert.match(style, /\.startup-card \{/);
const overlayStyle = style.match(/\.startup-overlay \{([^}]*)\}/)?.[1] ?? "";
assert.match(overlayStyle, /background: transparent;/);
assert.doesNotMatch(overlayStyle, /backdrop-filter|radial-gradient|rgba\(/);
const tauri = readFileSync("src/lib/tauri.ts", "utf8");
assert.match(tauri, /export async function restoreMainWindow\(\)[\s\S]*?setBackgroundColor\(TRANSPARENT_WINDOW_BACKGROUND\)/);
assert.match(style, /\.startup-logo::before/);
assert.match(style, /\.startup-logo::after/);
assert.match(style, /\.startup-logo-link \{/);
assert.match(style, /\.startup-progress::before \{/);

assert.match(
  style,
  /@keyframes startupOverlayEnter \{[\s\S]*transform: translateY\(18px\) scale\(0\.9\);[\s\S]*transform: translateY\(0\) scale\(1\);[\s\S]*\}/,
);
assert.match(
  style,
  /@keyframes startupProgress \{[\s\S]*transform: translateX\(-105%\);[\s\S]*transform: translateX\(175%\);[\s\S]*\}/,
);
const reducedMotionRule = style.match(/@media \(prefers-reduced-motion: reduce\) \{([\s\S]*?)\n\}/)?.[1] ?? "";
assert.match(reducedMotionRule, /animation: none !important/);
assert.doesNotMatch(reducedMotionRule, /\.startup-/, "explicit startup motion must remain enabled when Windows desktop animations are disabled");
