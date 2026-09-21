import assert from "node:assert/strict";
import { translateSource } from "../src/i18n/index.ts";

const conversionError = "clipboard error: The image or the text that was about the be transferred to/from the clipboard could not be converted to the appropriate format.";
assert.equal(translateSource(conversionError, "zh-CN"), "剪贴板错误：图片或文字无法转换成剪贴板支持的格式");
assert.equal(translateSource(conversionError, "en-US"), "Clipboard error: The image or text could not be converted to a format supported by the clipboard");
assert.equal(translateSource(translateSource(conversionError, "en-US"), "zh-CN"), translateSource(conversionError, "zh-CN"));
assert.equal(translateSource("clipboard error: The native clipboard is not accessible due to being held by another party.", "zh-CN"), "剪贴板错误：剪贴板被其他程序占用，暂时无法访问");
assert.match(translateSource("clipboard error: The clipboard contents were not available in the requested format or the clipboard is empty.", "zh-CN"), /剪贴板为空/);
assert.match(translateSource("clipboard error: The selected clipboard is not supported with the current system configuration.", "zh-CN"), /系统配置不支持/);
assert.equal(translateSource("clipboard error: Unknown error while interacting with the clipboard: OS 123 / C:\\file.png", "zh-CN"), "剪贴板错误：访问剪贴板时发生未知错误：OS 123 / C:\\file.png");
assert.equal(translateSource(translateSource("clipboard error: Unknown error while interacting with the clipboard: OS 123", "en-US"), "zh-CN"), "剪贴板错误：访问剪贴板时发生未知错误：OS 123");
assert.equal(translateSource("unrecognized OS message / C:\\file.png", "zh-CN"), "unrecognized OS message / C:\\file.png");
assert.equal(translateSource("文件传输失败", "en-US"), "File transfer failed");
