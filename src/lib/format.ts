export function formatTime(value: string | null): string {
  if (!value) {
    return "暂无";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(date);
}

export function deviceAddress(ip: string, port: number): string {
  const normalizedIp = normalizeDeviceHost(ip);
  return `${normalizedIp}:${port}`;
}

function normalizeDeviceHost(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) {
    return trimmed;
  }

  try {
    const url = new URL(trimmed);
    return url.hostname || trimmed;
  } catch {
    const match = trimmed.match(/^(.+):(\d+)$/);
    return match?.[1] || trimmed;
  }
}

export function clampPort(value: number): number {
  if (!Number.isFinite(value)) {
    return 8765;
  }

  return Math.min(65535, Math.max(1, Math.round(value)));
}
