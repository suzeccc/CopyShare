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
