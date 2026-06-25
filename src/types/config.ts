export type AppTheme = "copyBlue" | "win11Dark";
export type CloseAction = "ask" | "minimize" | "exit";

export interface AppConfig {
  configVersion: number;
  deviceName: string;
  deviceId: string;
  theme: AppTheme;
  closeAction: CloseAction;
  port: number;
  autoStart: boolean;
  autoSync: boolean;
  saveHistory: boolean;
  trustedDevices: string[];
  syncText: boolean;
  syncImage: boolean;
  syncFiles: boolean;
}
