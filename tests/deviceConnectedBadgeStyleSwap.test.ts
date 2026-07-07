import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const deviceCard = readFileSync("src/components/devices/DeviceCard.vue", "utf8");

const historyConnectedBranch = deviceCard.match(
  /if \(props\.device\.trusted && props\.device\.remoteTrusted\) \{[\s\S]*?return \{([\s\S]*?)\};/,
)?.[1];
const activeConnectedBranch = deviceCard.match(
  /if \(props\.mode === "connected"\) \{[\s\S]*?return \{([\s\S]*?)\};/,
)?.[1];

assert.ok(historyConnectedBranch);
assert.ok(activeConnectedBranch);
assert.match(
  historyConnectedBranch,
  /badgeClass: "border-white\/15 bg-white\/\[0\.07\] text-slate-100"/,
);
assert.match(
  historyConnectedBranch,
  /dotClass: "bg-\[#8fd6a8\] shadow-\[0_0_10px_rgba\(143,214,168,0\.34\)\]"/,
);
assert.match(
  activeConnectedBranch,
  /badgeClass: "border-emerald-400\/45 bg-emerald-400\/10 text-emerald-100"/,
);
assert.match(
  activeConnectedBranch,
  /dotClass: "bg-emerald-300 shadow-\[0_0_14px_rgba\(110,231,183,0\.65\)\]"/,
);
