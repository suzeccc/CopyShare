import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  createToast,
  limitToastQueue,
  TOAST_LIMIT,
  TOAST_TIMEOUT_MS,
} from "../src/lib/toasts.ts";

const toast = createToast("success", "复制成功");
assert.equal(toast.kind, "success");
assert.equal(toast.message, "复制成功");
assert.match(toast.id, /^toast-/);
assert.equal(typeof toast.createdAt, "number");
assert.equal(TOAST_TIMEOUT_MS, 1800);
assert.equal(TOAST_LIMIT, 3);
assert.equal(
  limitToastQueue([
    createToast("success", "1"),
    createToast("success", "2"),
    createToast("success", "3"),
    createToast("success", "4"),
  ]).map((item) => item.message).join(","),
  "2,3,4",
);

const toastStore = readFileSync("src/stores/toasts.ts", "utf8");
const toastStack = readFileSync("src/components/ui/ToastStack.vue", "utf8");
const app = readFileSync("src/App.vue", "utf8");
const refreshButton = readFileSync("src/components/ui/RefreshButton.vue", "utf8");
const copyButton = readFileSync("src/components/ui/CopyTextButton.vue", "utf8");
const settings = readFileSync("src/pages/Settings.vue", "utf8");
const devicesStore = readFileSync("src/stores/devices.ts", "utf8");

assert.match(toastStore, /defineStore\("toasts"/);
assert.match(toastStore, /success\(message: string\)/);
assert.match(toastStore, /error\(message: string\)/);

assert.match(toastStack, /data-toast-stack/);
assert.match(toastStack, /data-toast-item/);
assert.match(toastStack, /CircleCheck/);
assert.match(toastStack, /bg-\[rgba\(72,75,82,0\.92\)\]/);

assert.match(app, /ToastStack/);
assert.match(refreshButton, /useToastStore/);
assert.match(refreshButton, /刷新成功/);
assert.match(refreshButton, /failed\?:/);
assert.match(refreshButton, /props\.failed\?\.\(\)/);
assert.match(copyButton, /useToastStore/);
assert.match(copyButton, /复制成功/);
assert.match(settings, /useToastStore/);
assert.match(settings, /保存成功/);
assert.match(settings, /保存失败/);
assert.match(devicesStore, /useToastStore/);
assert.match(devicesStore, /连接成功/);
assert.match(devicesStore, /连接失败/);
