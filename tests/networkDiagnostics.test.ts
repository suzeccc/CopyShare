import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  isNetworkDiagnosticInformational,
  isOperationalNetworkDiagnostic,
} from "../src/types/networkDiagnostics.ts";

const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
const lib = readFileSync("src-tauri/src/lib.rs", "utf8");
const diagnostics = readFileSync("src-tauri/src/network_diagnostics.rs", "utf8");
const tauriApi = readFileSync("src/lib/tauri.ts", "utf8");
const settings = readFileSync("src/pages/Settings.vue", "utf8");
const devices = readFileSync("src/pages/Devices.vue", "utf8");
const dialog = readFileSync("src/components/settings/NetworkDiagnosticsDialog.vue", "utf8");
const styles = readFileSync("src/style.css", "utf8");

assert.match(commands, /pub async fn get_network_diagnostics/);
assert.match(commands, /pub async fn repair_windows_firewall/);
assert.match(commands, /pub async fn open_windows_network_settings/);
assert.match(commands, /ms-settings:network-status/);
assert.match(lib, /commands::get_network_diagnostics/);
assert.match(lib, /commands::repair_windows_firewall/);
assert.match(lib, /commands::open_windows_network_settings/);

for (const endpoint of [
  ["TCP", "sync_port"],
  ["UDP", "DISCOVERY_PORT"],
  ["TCP", "MOBILE_HTTP_PORT"],
]) {
  assert.match(diagnostics, new RegExp(endpoint.join("[\\s\\S]*")));
}
assert.match(diagnostics, /-Profile Private/);
assert.match(diagnostics, /Start-Process[\s\S]*-Verb RunAs/);

assert.match(tauriApi, /invoke<NetworkDiagnosticReport>\("get_network_diagnostics"\)/);
assert.match(tauriApi, /invoke<NetworkDiagnosticReport>\("repair_windows_firewall"\)/);
assert.match(tauriApi, /invoke<void>\("open_windows_network_settings"\)/);
assert.doesNotMatch(settings, /data-network-diagnostics-settings/);
assert.doesNotMatch(settings, /data-network-diagnostics-entry/);
assert.doesNotMatch(settings, /NetworkDiagnosticsDialog/);
assert.doesNotMatch(settings, /data-network-diagnostics-results/);
assert.match(styles, /\[data-network-diagnostics-entry\][\s\S]*max-width:\s*640px/);
assert.match(devices, /data-device-connection-error/);
assert.match(devices, /v-if="connectionError"/);
assert.match(devices, /连接没有成功/);
assert.match(devices, /CircleAlert/);
assert.match(devices, /data-device-network-diagnostics-button/);
assert.match(devices, /NetworkDiagnosticsDialog/);
assert.match(devices, /@click="openNetworkDiagnosticsDialog"/);
assert.match(devices, /@refresh="loadNetworkDiagnostics\(true\)"/);
assert.match(devices, /@repair="repairFirewall"/);
assert.match(devices, /@open-network-settings="openSystemNetworkSettings"/);

assert.match(dialog, /data-network-diagnostics-dialog/);
assert.match(dialog, /role="dialog"/);
assert.match(dialog, /aria-modal="true"/);
assert.match(dialog, /data-network-diagnostics-results/);
assert.match(dialog, /data-network-diagnostic-check/);
assert.match(dialog, /data-network-diagnostics-summary/);
assert.match(dialog, /data-network-summary-item/);
assert.match(dialog, /data-network-repair-guidance/);
assert.match(dialog, /data-network-technical-toggle/);
assert.match(dialog, /v-show="technicalDetailsOpen"/);
assert.match(dialog, /data-open-network-settings-button/);
assert.match(dialog, /data-firewall-repair-button/);
assert.match(dialog, /v-if="firewallNeedsRepair"/);
assert.match(dialog, /data-network-diagnostics-refresh-button/);
assert.match(dialog, /@keydown\.esc\.stop\.prevent="closeDialog"/);
assert.match(dialog, /@click\.self="closeDialog"/);
assert.equal(isOperationalNetworkDiagnostic("sync-listener"), true);
assert.equal(isOperationalNetworkDiagnostic("discovery-listener"), true);
assert.equal(isOperationalNetworkDiagnostic("mobile-listener"), true);
assert.equal(isNetworkDiagnosticInformational("windows-network-profile"), true);
assert.equal(isNetworkDiagnosticInformational("windows-firewall-profile"), true);
assert.equal(isNetworkDiagnosticInformational("firewall-sync"), true);
assert.match(devices, /isOperationalNetworkDiagnostic\(item\.id\)/);
assert.match(dialog, /group\.informational \? "仅供参考"/);
assert.match(dialog, /isNetworkDiagnosticInformational\(item\.id\) \? "参考"/);
