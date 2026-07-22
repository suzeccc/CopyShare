import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const router = readFileSync("src/router/index.ts", "utf8");
const qrCode = readFileSync("src/lib/qrCode.ts", "utf8");
const libraryStore = readFileSync("src/stores/library.ts", "utf8");
const statusStore = readFileSync("src/stores/status.ts", "utf8");

function sourceFiles(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? sourceFiles(path) : [path];
  });
}

for (const page of [
  "About",
  "Clipboard",
  "Devices",
  "FloatingClipboardHistory",
  "Library",
  "Logs",
  "MediaPreview",
  "MobileQr",
  "Ocr",
  "Settings",
  "Translate",
]) {
  assert.match(router, new RegExp(`const ${page} = \\(\\) => import\\("@/pages/${page}\\.vue"\\)`));
}

assert.match(router, /import Home from "@\/pages\/Home\.vue"/);
assert.match(qrCode, /if \(!url\)[\s\S]*return "";[\s\S]*import\("qrcode"\)/);
assert.match(libraryStore, /savedItemsByHistoryId\(state\): ReadonlyMap<string, LibraryItem>/);
assert.match(libraryStore, /this\.savedItemsByHistoryId\.get\(historyId\)/);
assert.doesNotMatch(statusStore, /await import\("@\/stores\/devices"\)/);

for (const path of sourceFiles("src").filter((file) => /\.(?:ts|vue)$/.test(file))) {
  assert.doesNotMatch(
    readFileSync(path, "utf8"),
    /from "lucide-vue-next"/,
    `${path} should import icons directly`,
  );
}
