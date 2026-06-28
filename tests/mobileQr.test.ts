import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const router = readFileSync("src/router/index.ts", "utf8");
const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");
const tauri = readFileSync("src/lib/tauri.ts", "utf8");
const libRs = readFileSync("src-tauri/src/lib.rs", "utf8");

assert.match(router, /import MobileQr from "@\/pages\/MobileQr\.vue"/);
assert.match(router, /path: "\/mobile"/);
assert.match(router, /name: "mobile"/);

assert.match(sidebar, /Smartphone/);
assert.match(sidebar, /path: "\/mobile"/);
assert.match(sidebar, /手机扫码/);

assert.match(tauri, /createMobileSession/);
assert.match(tauri, /invoke<MobileSessionView>\("create_mobile_session"/);
assert.match(tauri, /closeMobileSession/);
assert.match(tauri, /invoke<MobileSessionView>\("close_mobile_session"/);
assert.doesNotMatch(tauri, /createMobileSendSession/);
assert.doesNotMatch(tauri, /createMobileReceiveSession/);
assert.match(tauri, /getMobileSessionStatus/);
assert.match(tauri, /confirmMobileClipboardWrite/);

assert.match(libRs, /commands::create_mobile_session/);
assert.match(libRs, /commands::close_mobile_session/);
assert.doesNotMatch(libRs, /commands::create_mobile_send_session/);
assert.doesNotMatch(libRs, /commands::create_mobile_receive_session/);
