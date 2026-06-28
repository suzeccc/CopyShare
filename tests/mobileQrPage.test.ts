import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const page = readFileSync("src/pages/MobileQr.vue", "utf8");

assert.match(page, /手机扫码/);
assert.match(page, /手机扫码进入同一局域网页/);
assert.match(page, /生成二维码/);
assert.match(page, /结束会话/);
assert.match(page, /保持到手动结束/);
assert.match(page, /本次运行期有效/);
assert.match(page, /电脑剪贴板/);
assert.match(page, /手机提交/);
assert.match(page, /自动写入/);
assert.match(page, /data-mobile-intro-copy/);
assert.doesNotMatch(page, /data-mobile-flow-points/);
assert.doesNotMatch(page, /手机取用/);
assert.doesNotMatch(page, /临时安全/);
assert.doesNotMatch(page, /扫码后手机可选择复制电脑剪贴板/);
assert.doesNotMatch(page, /手机发送会自动写入电脑剪贴板/);
assert.doesNotMatch(page, /同一局域网内使用，退出程序或手动结束后失效/);
assert.doesNotMatch(page, />手机可复制<\/span>/);
assert.doesNotMatch(page, />自动写入电脑<\/span>/);
assert.doesNotMatch(page, />访问链接<\/p>/);
assert.doesNotMatch(page, /data-mobile-url-box/);
assert.doesNotMatch(page, /data-mobile-url-value/);
assert.doesNotMatch(page, /\{\{ mobileStore\.session\?\.url \?\?/);
assert.match(page, /contentItems/);
assert.match(page, /submittedItems/);
assert.match(page, /最近 5 条/);
assert.doesNotMatch(page, /15 分钟/);
assert.doesNotMatch(page, /一次性 token/);
assert.doesNotMatch(page, /countdownPercent/);
assert.doesNotMatch(page, /linear-gradient|radial-gradient/);
assert.match(page, /QRCode/);
assert.match(page, /data-mobile-session-card/);
assert.match(page, /data-mobile-layout="qr-left-content-right"/);
assert.match(page, /data-mobile-qr-rail/);
assert.match(page, /data-mobile-qr-zone/);
assert.match(page, /data-mobile-content-panel/);
assert.match(page, /data-mobile-content-panel[\s\S]*data-mobile-session-lifetime-notice/);
assert.doesNotMatch(page, /data-mobile-qr-rail[\s\S]*data-mobile-session-lifetime-notice[\s\S]*data-mobile-content-panel/);
assert.match(page, /data-mobile-layout="qr-left-content-right"[^>]*w-full[^>]*max-w-full[^>]*overflow-hidden/);
assert.match(page, /data-mobile-content-panel[^>]*w-full[^>]*max-w-full[^>]*overflow-hidden/);
assert.match(page, /data-mobile-summary-grid[^>]*lg:grid-cols-\[minmax\(0,1fr\)_minmax\(0,1fr\)\]/);
assert.match(page, /data-mobile-clipboard-card[^>]*min-w-0[^>]*overflow-hidden/);
assert.match(page, /data-mobile-submit-card[^>]*min-w-0[^>]*overflow-hidden/);
assert.match(page, /data-mobile-qr-rail[\s\S]*data-mobile-content-panel/);
assert.match(page, /grid-cols-\[minmax\(320px,0\.46fr\)_minmax\(0,0\.54fr\)\]/);
assert.match(page, /max-w-\[320px\]/);
assert.match(page, /h-\[232px\] w-\[232px\]/);
assert.doesNotMatch(page, /data-mobile-send-card/);
assert.doesNotMatch(page, /data-mobile-receive-card/);

assert.match(page, /useMobileStore/);
assert.doesNotMatch(page, /const session = ref<MobileSessionView \| null>/);


