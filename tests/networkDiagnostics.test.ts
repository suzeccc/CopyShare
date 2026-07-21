import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
const lib = readFileSync("src-tauri/src/lib.rs", "utf8");
const diagnostics = readFileSync("src-tauri/src/network_diagnostics.rs", "utf8");
const tauriApi = readFileSync("src/lib/tauri.ts", "utf8");
const settings = readFileSync("src/pages/Settings.vue", "utf8");

assert.match(commands, /pub async fn get_network_diagnostics/);
assert.match(commands, /pub async fn repair_windows_firewall/);
assert.match(lib, /commands::get_network_diagnostics/);
assert.match(lib, /commands::repair_windows_firewall/);

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
assert.match(settings, /data-network-diagnostics-settings/);
assert.match(settings, /data-firewall-repair-button/);
assert.match(settings, /data-network-diagnostics-results/);
assert.match(settings, /void loadNetworkDiagnostics\(\);/);
