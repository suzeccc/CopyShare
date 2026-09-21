export type DiagnosticStatus = "pass" | "warning" | "error" | "unknown";

export interface LocalNetworkAddress {
  adapterName: string;
  address: string;
  private: boolean;
}

export interface NetworkDiagnosticCheck {
  id: string;
  status: DiagnosticStatus;
  title: string;
  detail: string;
  recommendation: string | null;
  protocol: string | null;
  port: number | null;
}

export interface NetworkDiagnosticReport {
  generatedAt: string;
  platform: string;
  preferredLocalIp: string | null;
  localAddresses: LocalNetworkAddress[];
  syncRunning: boolean;
  repairSupported: boolean;
  checks: NetworkDiagnosticCheck[];
}

const OPERATIONAL_NETWORK_DIAGNOSTIC_IDS = new Set([
  "local-address",
  "sync-listener",
  "discovery-listener",
  "mobile-listener",
]);

export function isOperationalNetworkDiagnostic(id: string): boolean {
  return OPERATIONAL_NETWORK_DIAGNOSTIC_IDS.has(id);
}

export function isNetworkDiagnosticInformational(id: string): boolean {
  return !isOperationalNetworkDiagnostic(id);
}
