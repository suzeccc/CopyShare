import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const deviceToast = readFileSync("src/lib/deviceToast.ts", "utf8");
const devicesStore = readFileSync("src/stores/devices.ts", "utf8");

assert.match(deviceToast, /connectionSuccessMessage\(device: DeviceInfo\): string/);
assert.match(deviceToast, /displayDeviceName\(device\)/);
assert.match(deviceToast, /deviceAddress\(device\.ip, device\.port\)/);
assert.match(deviceToast, /连接成功/);
assert.match(deviceToast, /对方设备/);
assert.match(deviceToast, /isConnectionAddress/);
assert.match(deviceToast, /value\.includes\(":\/\/"\)/);

assert.match(devicesStore, /connectionSuccessMessage\(device\)/);
assert.match(devicesStore, /hasRealDeviceName\(device\)/);
assert.doesNotMatch(devicesStore, /\$\{device\.name\}.*连接成功/);
