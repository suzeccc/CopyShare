import assert from "node:assert/strict";

import { copyTextToClipboard, getCopyableText } from "../src/lib/clipboard.ts";

assert.equal(getCopyableText("  hello  "), "hello");
assert.equal(getCopyableText("   "), null);
assert.equal(getCopyableText(null), null);

const writes: string[] = [];
const copied = await copyTextToClipboard("  latest sync  ", {
  writeText: async (text) => {
    writes.push(text);
  },
});

assert.equal(copied, "copied");
assert.deepEqual(writes, ["latest sync"]);
assert.equal(await copyTextToClipboard(""), "empty");
assert.equal(await copyTextToClipboard("text", null), "unsupported");
