export type AppTheme = "copyBlue" | "win11Dark" | "macosLight" | "macosDark";
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
  fileSaveDir: string | null;
  autoOpenFolderAfterSave: boolean;
  discoveryScanRanges: string[];
  desktopNotifications: boolean;
  notifyClipboard: boolean;
  notifyTrustRequired: boolean;
  notifyFileTransfer: boolean;
  notifyDeviceStatus: boolean;
  notifySyncError: boolean;
  notificationClipboardPreview: boolean;
}
