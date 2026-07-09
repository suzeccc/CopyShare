import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const home = readFileSync("src/pages/Home.vue", "utf8");

assert.match(home, /文件传输/);
assert.match(home, /configStore\.config\.syncFiles/);
assert.match(home, /md:grid-cols-3/);
assert.doesNotMatch(home, /to="\/files"/);
assert.doesNotMatch(home, /route:\s*"\/files"/);
