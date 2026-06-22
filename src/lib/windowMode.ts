export type AppWindowMode = "main" | "floating";

export const MAIN_WINDOW_BOUNDS = {
  width: 1120,
  height: 720,
  minWidth: 960,
  minHeight: 620,
} as const;

export const FLOATING_WINDOW_BOUNDS = {
  width: 340,
  height: 320,
  opacity: 0.66,
} as const;

export const FLOATING_WINDOW_BACKGROUND = "rgba(7, 24, 49, 0.66)";
export const TRANSPARENT_WINDOW_BACKGROUND = "#00000000";
export const MAIN_WINDOW_BACKGROUND = "#172746";

export function getClipboardPreview(summary: string | null | undefined): string {
  const preview = summary?.trim();
  return preview ? preview : "暂无剪贴板内容";
}

export function getLatencyLabel(status: {
  running: boolean;
  connectedCount: number;
}): string {
  if (!status.running || status.connectedCount === 0) {
    return "-- ms";
  }

  return "检测中";
}
