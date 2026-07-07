import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const home = readFileSync("src/pages/Home.vue", "utf8");

assert.doesNotMatch(home, /后续版本支持文件列表同步/);
assert.doesNotMatch(home, /state:\s*"暂未开放"/);
assert.match(home, /文件传输/);
assert.match(home, /to="\/files"/);
