export const FILE_SIZE_LIMIT_MIN_MIB = 100;
export const FILE_SIZE_LIMIT_MAX_MIB = 2048;
export const FILE_SIZE_LIMIT_WHEEL_STEP_MIB = 100;

export function clampFileSizeLimitMib(value: number) {
  if (!Number.isFinite(value)) {
    return FILE_SIZE_LIMIT_MAX_MIB;
  }

  return Math.min(
    FILE_SIZE_LIMIT_MAX_MIB,
    Math.max(FILE_SIZE_LIMIT_MIN_MIB, Math.round(value)),
  );
}

export function adjustFileSizeLimitFromWheel(value: number, deltaY: number) {
  if (deltaY === 0) {
    return clampFileSizeLimitMib(value);
  }

  const direction = deltaY < 0 ? 1 : -1;
  return clampFileSizeLimitMib(value + direction * FILE_SIZE_LIMIT_WHEEL_STEP_MIB);
}

export function formatFileSizeLimit(valueMib: number) {
  const value = clampFileSizeLimitMib(valueMib);
  if (value < 1024) {
    return `${value} MiB`;
  }

  const gib = (value / 1024).toFixed(2).replace(/\.?0+$/, "");
  return `${gib} GiB`;
}
