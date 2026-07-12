import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const stores = [
  "src/stores/config.ts",
  "src/stores/devices.ts",
  "src/stores/history.ts",
];

for (const storePath of stores) {
  const source = readFileSync(storePath, "utf8");

  assert.match(
    source,
    /unlisteners:\s*\[\]\s+as\s+\(\(\)\s*=>\s*void\)\[\]/,
    `${storePath} should keep Tauri unlisten callbacks`,
  );
  assert.match(
    source,
    /if\s*\(\s*this\.unlisteners\.length\s*\)\s*\{\s*return;\s*\}/,
    `${storePath} subscribe() should be idempotent`,
  );
  assert.match(
    source,
    /this\.unlisteners\s*=\s*await\s+Promise\.all\(/,
    `${storePath} should retain every event subscription cleanup callback`,
  );
}

const appSource = readFileSync("src/App.vue", "utf8");
assert.match(
  appSource,
  /let\s+navigateUnlisten:\s*\(\(\)\s*=>\s*void\)\s*\|\s*undefined/,
  "src/App.vue should retain the navigate-to-page event cleanup callback",
);
assert.match(
  appSource,
  /onBeforeUnmount\(\s*\(\)\s*=>\s*\{/,
  "src/App.vue should clean root event subscriptions on unmount",
);
assert.match(
  appSource,
  /navigateUnlisten\?\.\(\)/,
  "src/App.vue should call the navigate-to-page unlisten callback",
);
