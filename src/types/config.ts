export type AppTheme = "copyBlue" | "win11Dark";

export interface AppConfig {
  deviceName: string;
  deviceId: string;
  theme: AppTheme;
  port: number;
  autoStart: boolean;
  autoSync: boolean;
  saveHistory: boolean;
  trustedDevices: string[];
  syncText: boolean;
  syncImage: boolean;
  syncFiles: boolean;
}
