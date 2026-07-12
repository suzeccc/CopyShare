export type MediaPreviewImagePoint = {
  x: number;
  y: number;
};

export const MEDIA_PREVIEW_IMAGE_MIN_SCALE = 1;
export const MEDIA_PREVIEW_IMAGE_MAX_SCALE = 5;
export const MEDIA_PREVIEW_IMAGE_WHEEL_STEP = 0.15;

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function roundScale(value: number): number {
  return Math.round(value * 100) / 100;
}

export function getNextMediaPreviewImageScale(
  currentScale: number,
  wheelDeltaY: number,
): number {
  const direction = wheelDeltaY < 0 ? 1 : -1;
  return roundScale(
    clamp(
      currentScale + direction * MEDIA_PREVIEW_IMAGE_WHEEL_STEP,
      MEDIA_PREVIEW_IMAGE_MIN_SCALE,
      MEDIA_PREVIEW_IMAGE_MAX_SCALE,
    ),
  );
}

export function getNextMediaPreviewImageOffset(
  originOffset: MediaPreviewImagePoint,
  originPointer: MediaPreviewImagePoint,
  nextPointer: MediaPreviewImagePoint,
): MediaPreviewImagePoint {
  return {
    x: originOffset.x + nextPointer.x - originPointer.x,
    y: originOffset.y + nextPointer.y - originPointer.y,
  };
}

export function shouldPanMediaPreviewImage(scale: number): boolean {
  return scale >= MEDIA_PREVIEW_IMAGE_MIN_SCALE;
}
