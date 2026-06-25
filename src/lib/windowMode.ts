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

export const FLOATING_WINDOW_MARGIN = 16;

export const FLOATING_WINDOW_BACKGROUND = "rgba(5, 18, 39, 0.68)";
export const TRANSPARENT_WINDOW_BACKGROUND = "#00000000";
export const MAIN_WINDOW_BACKGROUND = "#10203a";

export type WindowPositionArea = {
  position: {
    x: number;
    y: number;
  };
  size: {
    width: number;
    height: number;
  };
  scaleFactor: number;
};

export function getFloatingWindowTopRightPosition(area: WindowPositionArea): {
  x: number;
  y: number;
} {
  const margin = FLOATING_WINDOW_MARGIN * area.scaleFactor;
  const width = FLOATING_WINDOW_BOUNDS.width * area.scaleFactor;

  return {
    x: Math.round(area.position.x + area.size.width - width - margin),
    y: Math.round(area.position.y + margin),
  };
}

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
