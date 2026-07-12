export type MediaPreviewRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type MediaPreviewWindowPositionInput = {
  floating: MediaPreviewRect;
  monitor: MediaPreviewRect;
  preview: {
    width: number;
    height: number;
    offset: number;
  };
};

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

export function getMediaPreviewWindowPosition({
  floating,
  monitor,
  preview,
}: MediaPreviewWindowPositionInput): { x: number; y: number } {
  const leftX = floating.x - preview.width - preview.offset;
  const rightX = floating.x + floating.width + preview.offset;
  const rightFits = rightX + preview.width <= monitor.x + monitor.width;
  const leftFits = leftX >= monitor.x;
  const preferredX = rightFits || !leftFits ? rightX : leftX;
  const maxX = monitor.x + monitor.width - preview.width;
  const maxY = monitor.y + monitor.height - preview.height;

  return {
    x: Math.round(clamp(preferredX, monitor.x, maxX)),
    y: Math.round(clamp(floating.y, monitor.y, maxY)),
  };
}
