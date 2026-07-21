export type FileTransferStatus =
  | "pending"
  | "accepted"
  | "transferring"
  | "completed"
  | "failed"
  | "canceled"
  | "rejected";

export type FileTransferDirection = "send" | "receive";

export type FileTransferFileStatus =
  | "pending"
  | "transferring"
  | "completed"
  | "failed"
  | "canceled";

export interface FileTransferFile {
  id: string;
  name: string;
  size: number;
  sha256: string;
  savedPath: string | null;
  transferredBytes: number;
  status: FileTransferFileStatus;
  error: string | null;
}

export interface FileTransferTask {
  transferId: string;
  direction: FileTransferDirection;
  peerDeviceId: string;
  peerDeviceName: string;
  clipboardSync: boolean;
  files: FileTransferFile[];
  totalSize: number;
  transferredBytes: number;
  status: FileTransferStatus;
  createdAt: string;
  completedAt: string | null;
  error: string | null;
}

export interface SelectedTransferFile {
  path: string;
  name: string;
  size: number;
  sha256: string;
}

export interface FileTransferProgressEvent {
  transferId: string;
  fileId: string;
  fileTransferredBytes: number;
  fileSize: number;
  totalTransferredBytes: number;
  totalSize: number;
  status?: FileTransferStatus;
}
