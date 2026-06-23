export type ManualConnectDraft = {
  ip: string;
  port: number;
};

export function createManualConnectDraft(defaultPort = 8765): ManualConnectDraft {
  return {
    ip: "",
    port: clampPortValue(defaultPort),
  };
}

export function setManualConnectDraftIp(
  draft: ManualConnectDraft,
  ip: string,
): ManualConnectDraft {
  return {
    ...draft,
    ip: ip.trim(),
  };
}

export function setManualConnectDraftPort(
  draft: ManualConnectDraft,
  port: number,
): ManualConnectDraft {
  return {
    ...draft,
    port: clampPortValue(port),
  };
}

function clampPortValue(value: number): number {
  if (!Number.isFinite(value)) {
    return 8765;
  }

  return Math.min(65535, Math.max(1, Math.round(value)));
}
