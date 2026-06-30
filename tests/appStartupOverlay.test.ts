import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const app = readFileSync("src/App.vue", "utf8");
const style = readFileSync("src/style.css", "utf8");

assert.match(app, /const STARTUP_OVERLAY_MIN_MS = 900;/);
assert.match(app, /const startupVisible = ref\(true\);/);
assert.match(app, /performance\.now\(\)/);
assert.match(app, /Math\.max\(STARTUP_OVERLAY_MIN_MS - elapsed, 0\)/);
assert.match(app, /startupVisible\.value = false;/);

assert.match(app, /<Transition name="startup-overlay">/);
assert.match(app, /data-startup-overlay/);
assert.match(app, /aria-live="polite"/);
assert.match(app, /CopyShare/);
assert.match(app, /正在准备同步/);
assert.match(app, /startup-logo-link/);
assert.match(app, /startup-progress/);

assert.match(style, /\.startup-overlay \{/);
assert.match(style, /\.startup-card \{/);
assert.match(style, /\.startup-logo::before/);
assert.match(style, /\.startup-logo::after/);
assert.match(style, /\.startup-logo-link \{/);
assert.match(style, /\.startup-progress::before \{/);

assert.match(
  style,
  /@keyframes startupOverlayEnter \{[\s\S]*transform: translateY\(14px\) scale\(0\.94\);[\s\S]*transform: translateY\(0\) scale\(1\);[\s\S]*\}/,
);
assert.match(
  style,
  /@keyframes startupProgress \{[\s\S]*transform: translateX\(-105%\);[\s\S]*transform: translateX\(82%\);[\s\S]*\}/,
);
assert.match(
  style,
  /@media \(prefers-reduced-motion: reduce\) \{[\s\S]*\.startup-card,[\s\S]*\.startup-progress::before[\s\S]*animation: none !important;[\s\S]*\}/,
);
