import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const store = readFileSync("src/stores/mobile.ts", "utf8");
const types = readFileSync("src/types/mobile.ts", "utf8");
const tauri = readFileSync("src/lib/tauri.ts", "utf8");

assert.match(types, /\| "closed"/);
assert.match(types, /expiresAt: string \| null/);
assert.match(types, /remainingSeconds: number \| null/);

assert.match(store, /phase === "expired" \|\| phase === "closed"/);
assert.match(store, /closeSession/);
assert.match(store, /closeMobileSession\(this\.session\.id\)/);

assert.match(tauri, /closeMobileSession\(sessionId: string\): Promise<MobileSessionView>/);
