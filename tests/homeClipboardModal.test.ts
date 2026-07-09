import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const historyPreview = readFileSync("src/lib/historyPreview.ts", "utf8");
const modalIndex = clipboardPage.indexOf("data-clipboard-history-modal");
const defaultRecentSection = modalIndex === -1 ? clipboardPage : clipboardPage.slice(0, modalIndex);
const modalSection = modalIndex === -1 ? "" : clipboardPage.slice(modalIndex);

assert.match(clipboardPage, /ref\(false\)/);
assert.match(clipboardPage, /showClipboardHistoryModal/);
assert.match(clipboardPage, /clipboardSearch/);
assert.match(clipboardPage, /clipboardCategories/);
assert.match(clipboardPage, /activeClipboardCategory/);
assert.match(clipboardPage, /搜索剪切板\.\.\./);

for (const category of ["全部", "文本", "图片", "链接", "文件"]) {
  assert.match(historyPreview, new RegExp(`"${category}"`));
}

assert.doesNotMatch(historyPreview, /"API Key"/);
assert.match(clipboardPage, /allClipboardItems\s*=\s*computed/);
assert.match(clipboardPage, /getRecentClipboardItems\(historyStore\.items,\s*historyStore\.items\.length\)/);
assert.match(clipboardPage, /filteredRecentSyncItems/);
assert.match(clipboardPage, /filteredAllClipboardItems/);
assert.match(clipboardPage, /filterClipboardItems\(\s*recentSyncItems\.value,\s*activeClipboardCategory\.value,\s*""\s*\)/);
assert.match(clipboardPage, /filterClipboardItems\(\s*allClipboardItems\.value,\s*activeClipboardCategory\.value,\s*clipboardSearch\.value\s*\)/);

assert.match(clipboardPage, /data-more-clipboard-button/);
assert.match(clipboardPage, /@click="showClipboardHistoryModal = true"/);
assert.match(clipboardPage, /data-clipboard-history-modal/);
assert.match(clipboardPage, /v-if="showClipboardHistoryModal"/);
assert.match(clipboardPage, /@click\.self="showClipboardHistoryModal = false"/);
assert.match(clipboardPage, /@click="showClipboardHistoryModal = false"/);
assert.match(clipboardPage, /v-for="\(\s*item,\s*index\s*\) in filteredAllClipboardItems"/);
assert.match(clipboardPage, /data-clipboard-history-row/);
assert.match(clipboardPage, /data-clipboard-type-label/);
assert.match(clipboardPage, /clipboardTypeIcon/);
assert.match(clipboardPage, /ClipboardIcon/);
assert.match(clipboardPage, /Link2/);
assert.match(clipboardPage, /<component :is="clipboardTypeIcon\(getClipboardDisplayType\(item\)\)"/);
assert.doesNotMatch(clipboardPage, /getClipboardDisplayType\(item\)\.icon/);
assert.doesNotMatch(defaultRecentSection, /data-clipboard-search-input/);
assert.match(modalSection, /data-clipboard-search-input/);
assert.match(clipboardPage, /data-clipboard-category-button/);
assert.match(clipboardPage, /data-clipboard-history-actions/);
assert.match(clipboardPage, /data-clipboard-history-text/);
assert.match(clipboardPage, /\bbreak-all\b/);
assert.match(clipboardPage, /data-clipboard-card-main/);
assert.match(clipboardPage, /data-clipboard-card-meta/);
assert.match(clipboardPage, /data-clipboard-card-time/);
assert.match(clipboardPage, /data-clipboard-card-footer class="[^"]*text-\[color:var\(--clipboard-card-footer-text\)\]/);
assert.match(clipboardPage, /data-clipboard-history-device[\s\S]*class="[^"]*text-\[color:var\(--clipboard-card-footer-text\)\]/);
assert.match(clipboardPage, /data-clipboard-card-action/);
assert.match(clipboardPage, /clipboard-preview-card/);
assert.match(clipboardPage, /bg-\[color:var\(--clipboard-card-bg\)\]/);
assert.match(clipboardPage, /border-\[color:var\(--clipboard-card-line\)\]/);
assert.match(clipboardPage, /hover:bg-\[color:var\(--clipboard-card-bg-hover\)\]/);
assert.match(clipboardPage, /hover:border-\[color:var\(--clipboard-card-line-hover\)\]/);
assert.match(clipboardPage, /hover:shadow-\[var\(--clipboard-card-shadow-hover\)\]/);
assert.match(clipboardPage, /hover:scale-\[1\.01\]/);
assert.match(clipboardPage, /hover:z-10/);
assert.match(clipboardPage, /clipboard-preview-card-accent/);
assert.match(clipboardPage, /min-h-\[86px\]/);
assert.match(clipboardPage, /left-3 top-2\.5 bottom-2\.5 w-0\.5/);
assert.match(clipboardPage, /px-5 py-2\.5/);
assert.match(clipboardPage, /data-clipboard-history-sync-status/);
assert.match(clipboardPage, /data-clipboard-history-device/);
assert.doesNotMatch(clipboardPage, /data-clipboard-history-device[\s\S]*sr-only/);
assert.match(clipboardPage, /<CopyTextButton/);
assert.match(clipboardPage, /:text="item\.text"/);
assert.match(clipboardPage, /:content-type="item\.contentType"/);
assert.match(clipboardPage, /:history-item-id="item\.id"/);

for (const accentColor of ["#007aff", "#af52de", "#34a851", "#7b5520"]) {
  assert.match(clipboardPage, new RegExp(`bg-\\[\\${accentColor}\\]`));
}
for (const oldAccentColor of ["#7FA88A", "#7B93A8", "#9BA36A"]) {
  assert.doesNotMatch(clipboardPage, new RegExp(`bg-\\[\\${oldAccentColor}\\]`));
}
assert.doesNotMatch(clipboardPage, /bg-\[\#B08A5A\]/);
assert.doesNotMatch(clipboardPage, /#A87983/);
assert.match(clipboardPage, /\bw-0\.5\b/);
assert.doesNotMatch(clipboardPage, /shadow-\[0_0_18px_var\(--accent-glow\)\]/);
assert.doesNotMatch(clipboardPage, /\bbg-violet-300\b/);
