export type AppWindowMode = "main" | "floating" | "ball";

export const MAIN_WINDOW_BOUNDS = {
  width: 1120,
  height: 740,
  minWidth: 960,
  minHeight: 620,
} as const;

export const FLOATING_WINDOW_BOUNDS = {
  width: 340,
  height: 392,
  opacity: 0.66,
} as const;

export const FLOATING_BALL_BOUNDS = { width: 64, height: 64 } as const;

export const FLOATING_WINDOW_MARGIN = 16;
export const FLOATING_STARTUP_OFFSET = { right: 96, top: 192 } as const;

export function getFloatingBallDockPosition(
  area: WindowPositionArea,
  anchor: { x: number; y: number },
): { x: number; y: number } {
  const margin = FLOATING_WINDOW_MARGIN * area.scaleFactor;
  const width = FLOATING_BALL_BOUNDS.width * area.scaleFactor;
  const height = FLOATING_BALL_BOUNDS.height * area.scaleFactor;
  const left = area.position.x + margin;
  const right = area.position.x + Math.max(margin, area.size.width - width - margin);
  const top = area.position.y + margin;
  const bottom = area.position.y + Math.max(margin, area.size.height - height - margin);
  return {
    x: Math.round(Math.abs(anchor.x - left) <= Math.abs(anchor.x - right) ? left : right),
    y: Math.round(clamp(anchor.y, top, bottom)),
  };
}

export const FLOATING_WINDOW_BACKGROUND = "rgba(11, 16, 14, 0.72)";
export const TRANSPARENT_WINDOW_BACKGROUND = "#00000000";
export const MAIN_WINDOW_BACKGROUND = "#0b100e";

export async function waitForWindowSize(
  size: { width: number; height: number },
  viewport: { readonly innerWidth: number; readonly innerHeight: number } = window,
  timeoutMs = 2000,
): Promise<void> {
  // Native setters enqueue changes; wait for the webview to receive the new viewport.
  const deadline = Date.now() + timeoutMs;
  while (Math.abs(viewport.innerWidth - size.width) > 1 || Math.abs(viewport.innerHeight - size.height) > 1) {
    if (Date.now() >= deadline) throw new Error("Window resize did not reach the requested size");
    await new Promise<void>((resolve) => setTimeout(resolve, 16));
  }
}

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
  const height = FLOATING_WINDOW_BOUNDS.height * area.scaleFactor;
  const right = Math.max(margin, FLOATING_STARTUP_OFFSET.right * area.scaleFactor);
  const top = FLOATING_STARTUP_OFFSET.top * area.scaleFactor;

  return {
    x: Math.round(area.position.x + Math.max(margin, area.size.width - width - right)),
    y: Math.round(area.position.y + clamp(top, margin, Math.max(margin, area.size.height - height - margin))),
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
  const height = (MAIN_WINDOW_BOUNDS.height - 20) * area.scaleFactor;

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
