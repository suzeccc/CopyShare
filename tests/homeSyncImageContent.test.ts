import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const home = readFileSync("src/pages/Home.vue", "utf8");

assert.doesNotMatch(home, /后续版本支持图片剪贴板/);
assert.match(home, /支持截图和图片复制/);
assert.match(home, /state: configStore\.config\.syncImage \? "已启用" : "已关闭"/);
assert.match(home, /enabled: configStore\.config\.syncImage/);
