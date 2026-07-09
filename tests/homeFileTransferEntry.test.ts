import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const home = readFileSync("src/pages/Home.vue", "utf8");

assert.doesNotMatch(home, /文件传输|鏂囦欢浼犺緭/);
assert.doesNotMatch(home, /to="\/files"/);
assert.doesNotMatch(home, /route:\s*"\/files"/);
