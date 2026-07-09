import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const style = readFileSync("src/style.css", "utf8");

assert.match(clipboardPage, /activeClipboardCategoryIndex\s*=\s*computed/);
assert.match(clipboardPage, /setActiveClipboardCategory\(category\)/);
assert.match(clipboardPage, /data-clipboard-category-tabs/);
assert.match(clipboardPage, /data-clipboard-category-indicator/);
assert.match(clipboardPage, /'--clipboard-category-index':\s*activeClipboardCategoryIndex/);
assert.match(clipboardPage, /'--clipboard-category-count':\s*clipboardCategories\.length/);
assert.match(clipboardPage, /class="clipboard-category-chip"/);
assert.match(clipboardPage, /:class="\{\s*active: activeClipboardCategory === category\s*\}"/);
assert.match(clipboardPage, /@click="setActiveClipboardCategory\(category\)"/);
assert.doesNotMatch(clipboardPage, /rounded-lg border px-3 py-1\.5 text-xs font-semibold transition/);

assert.match(style, /\.clipboard-category-tabs\s*\{/);
assert.match(style, /\.clipboard-category-indicator\s*\{/);
assert.match(style, /transform:\s*translateX\(calc\(var\(--clipboard-category-index\) \* \(var\(--clipboard-category-pill-width\) \+ var\(--clipboard-category-gap\)\)\)\)/);
assert.match(style, /transition:[\s\S]*360ms cubic-bezier\(0\.2,\s*0\.8,\s*0\.2,\s*1\)/);
assert.match(style, /\.clipboard-category-chip\s*\{/);
assert.match(style, /background:\s*transparent/);
assert.match(style, /\.clipboard-category-chip\.active\s*\{/);
assert.doesNotMatch(style, /\.clipboard-category-chip\.active\s*\{[\s\S]*background:\s*#333/);
assert.match(style, /\.clipboard-category-chip:active\s*\{[\s\S]*transform:\s*scale\(0\.96\)/);

assert.match(clipboardPage, /<TransitionGroup[\s\S]*name="clipboard-card-stagger"[\s\S]*tag="div"/);
assert.match(clipboardPage, /data-clipboard-stagger-list/);
assert.match(clipboardPage, /data-clipboard-stagger-list[\s\S]*class="[^"]*\brelative\b[^"]*"/);
assert.match(clipboardPage, /v-for="\(\s*item,\s*index\s*\) in filteredRecentSyncItems"/);
assert.match(clipboardPage, /v-for="\(\s*item,\s*index\s*\) in filteredAllClipboardItems"/);
assert.match(clipboardPage, /--clipboard-row-index:\s*\$\{index\}/);
assert.match(style, /\.clipboard-card-stagger-enter-active[\s\S]*transition-delay:\s*calc\(var\(--clipboard-row-index\) \* 38ms\)/);
assert.match(style, /\.clipboard-card-stagger-leave-active\s*\{[\s\S]*position:\s*absolute/);
assert.match(style, /\.clipboard-card-stagger-leave-active\s*\{[\s\S]*width:\s*100%/);
assert.match(style, /\.clipboard-card-stagger-enter-from[\s\S]*opacity:\s*0[\s\S]*translateY\(14px\) scale\(0\.985\)/);
assert.match(style, /prefers-reduced-motion:\s*reduce[\s\S]*\.clipboard-card-stagger-enter-active/);
assert.match(style, /prefers-reduced-motion:\s*reduce[\s\S]*\.clipboard-category-indicator/);
