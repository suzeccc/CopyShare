import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const clipboardPage = readFileSync("src/pages/Clipboard.vue", "utf8");
const floatingPanel = readFileSync("src/components/layout/FloatingPanel.vue", "utf8");

assert.match(clipboardPage, /openExternalUrl/);
assert.match(clipboardPage, /getClipboardLinkUrl/);
assert.match(clipboardPage, /data-clipboard-link-button/);
assert.match(clipboardPage, /cursor-pointer select-none/);
assert.match(clipboardPage, /data-clipboard-link-button\s+class="[^"]*w-fit[^"]*max-w-full/);
assert.match(clipboardPage, /@click\.stop="openClipboardLink\(item\)"/);

assert.match(floatingPanel, /openExternalUrl/);
assert.match(floatingPanel, /getClipboardLinkUrl/);
assert.match(floatingPanel, /data-floating-clipboard-link-button/);
assert.match(floatingPanel, /cursor-pointer select-none/);
assert.match(
  floatingPanel,
  /data-floating-clipboard-link-button\s+class="[^"]*min-w-0[^"]*flex-1[^"]*overflow-hidden/,
);
assert.match(floatingPanel, /@click\.stop="openClipboardLink\(item\)"/);

for (const source of [clipboardPage, floatingPanel]) {
  assert.doesNotMatch(source, /'cursor-pointer':\s*getClipboardLinkUrl/);
}

const clipboardLinkClasses = clipboardPage.match(
  /data-clipboard-link-button\s+class="([^"]+)"/,
)?.[1] ?? "";
const floatingLinkClasses = floatingPanel.match(
  /data-floating-clipboard-link-button\s+class="([^"]+)"/,
)?.[1] ?? "";

for (const classes of [clipboardLinkClasses, floatingLinkClasses]) {
  assert.match(classes, /hover:text-\[color:var\(--accent-text\)\]/);
  assert.match(classes, /hover:underline/);
}
