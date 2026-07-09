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

export const FLOATING_WINDOW_BACKGROUND = "rgba(11, 16, 14, 0.72)";
export const TRANSPARENT_WINDOW_BACKGROUND = "#00000000";
export const MAIN_WINDOW_BACKGROUND = "#0b100e";

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

export type FloatingWindowPointer = {
  screenX: number;
  screenY: number;
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

export function getFloatingWindowPointerPosition(
  area: WindowPositionArea,
  pointer: FloatingWindowPointer,
): {
  x: number;
  y: number;
} {
  const margin = FLOATING_WINDOW_MARGIN * area.scaleFactor;
  const width = FLOATING_WINDOW_BOUNDS.width * area.scaleFactor;
  const height = FLOATING_WINDOW_BOUNDS.height * area.scaleFactor;
  const minX = area.position.x + margin;
  const minY = area.position.y + margin;
  const maxX = area.position.x + area.size.width - width - margin;
  const maxY = area.position.y + area.size.height - height - margin;

  return {
    x: Math.round(clamp(pointer.screenX - width / 2, minX, maxX)),
    y: Math.round(clamp(pointer.screenY - height / 2, minY, maxY)),
  };
}

export function getMainWindowCenteredPosition(area: WindowPositionArea): {
  x: number;
  y: number;
} {
  const width = MAIN_WINDOW_BOUNDS.width * area.scaleFactor;
  const height = MAIN_WINDOW_BOUNDS.height * area.scaleFactor;

  return {
    x: Math.round(area.position.x + (area.size.width - width) / 2),
    y: Math.round(area.position.y + (area.size.height - height) / 2),
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

export function getClipboardPreview(summary: string | null | undefined): string {
  const preview = summary?.trim();
  return preview ? preview : "暂无剪贴板内容";
}

export function getLatencyLabel(status: {
  running: boolean;
  connectedCount: number;
  latencyMs: number | null;
}): string {
  if (!status.running || status.connectedCount === 0) {
    return "-- ms";
  }

  if (status.latencyMs === null) {
    return "检测中";
  }

  return `${Math.max(0, Math.round(status.latencyMs))} ms`;

  return "检测中";
}
