export type ToastKind = "success" | "error" | "info";

export interface ToastMessage {
  id: string;
  kind: ToastKind;
  message: string;
  createdAt: number;
}

export const TOAST_TIMEOUT_MS = 1800;
export const TOAST_LIMIT = 3;

let toastSequence = 0;

export function createToast(kind: ToastKind, message: string): ToastMessage {
  toastSequence += 1;

  return {
    id: `toast-${Date.now()}-${toastSequence}`,
    kind,
    message,
    createdAt: Date.now(),
  };
}

export function limitToastQueue(toasts: ToastMessage[]): ToastMessage[] {
  return toasts.slice(-TOAST_LIMIT);
}
