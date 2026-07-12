import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const devices = readFileSync("src/pages/Devices.vue", "utf8");
const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");
const router = readFileSync("src/router/index.ts", "utf8");

assert.equal(existsSync("src/components/mobile/MobileConnectDialog.vue"), true);
const dialog = readFileSync("src/components/mobile/MobileConnectDialog.vue", "utf8");

assert.match(devices, /MobileConnectDialog/);
assert.match(devices, /data-device-action-grid/);
assert.match(devices, /data-lan-discovery-card/);
assert.match(devices, /data-mobile-connect-card/);
assert.match(devices, /data-mobile-connect-dialog-button/);
assert.match(devices, /showMobileConnectDialog/);
assert.match(devices, /@click="showMobileConnectDialog = true"/);

const lanCardClass = devices.match(/data-lan-discovery-card[\s\S]*?class="([^"]+)"/)?.[1] ?? "";
const mobileCardClass = devices.match(/data-mobile-connect-card[\s\S]*?class="([^"]+)"/)?.[1] ?? "";
assert.equal(mobileCardClass, lanCardClass);
assert.doesNotMatch(mobileCardClass, /accent|shadow-\[inset/);
assert.match(devices, /data-lan-discovery-card[\s\S]*?<div class="min-w-0 flex-1">/);
assert.match(devices, /data-mobile-connect-card[\s\S]*?<div class="min-w-0 flex-1">/);
assert.match(devices, /data-mobile-connect-dialog-button[\s\S]*class="shrink-0"/);
assert.match(devices, /data-mobile-connect-dialog-button[\s\S]*variant="secondary"/);

assert.match(dialog, /data-mobile-connect-dialog/);
assert.match(dialog, /data-mobile-connect-qr-zone/);
assert.match(dialog, /生成二维码/);
assert.match(dialog, /复制链接/);
assert.match(dialog, /结束会话/);
assert.match(dialog, /defineModel<boolean>/);
assert.match(dialog, /useMobileStore/);
assert.match(dialog, /QRCode/);
assert.match(dialog, /@click\.self="closeDialog"/);
assert.match(dialog, /Monitor/);
assert.match(dialog, /Smartphone/);
assert.match(dialog, /电脑剪贴板/);
assert.match(dialog, /手机提交/);
assert.match(dialog, /<Monitor class="h-4 w-4 text-\[color:var\(--accent-text\)\]" \/>\s*电脑剪贴板/);
assert.match(dialog, /<Smartphone class="h-4 w-4 text-\[color:var\(--accent-text\)\]" \/>\s*手机提交/);
assert.doesNotMatch(dialog, /<Smartphone class="h-4 w-4 text-\[color:var\(--accent-text\)\]" \/>\s*电脑剪贴板/);
assert.doesNotMatch(dialog, /<ShieldCheck class="h-4 w-4 text-emerald-300" \/>\s*手机提交/);

assert.doesNotMatch(sidebar, /label: "手机连接"/);
assert.doesNotMatch(sidebar, /path: "\/mobile"/);
assert.doesNotMatch(sidebar, /\n\s+Smartphone,\n/);
assert.doesNotMatch(sidebar, /icon: Smartphone/);

assert.match(router, /path: "\/mobile"/);
assert.match(router, /name: "mobile"/);
