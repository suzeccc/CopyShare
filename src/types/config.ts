export interface AppConfig {
  deviceName: string;
  port: number;
  autoStart: boolean;
  autoSync: boolean;
  saveHistory: boolean;
  trustedDevices: string[];
  syncText: boolean;
  syncImage: boolean;
  syncFiles: boolean;
}
