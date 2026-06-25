import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const config = readFileSync("vite.config.ts", "utf8");

assert.match(config, /const projectRoot = fileURLToPath\(new URL\("\."/);
assert.match(config, /root: projectRoot/);