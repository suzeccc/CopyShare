export type ClipboardWriter = {
  writeText: (text: string) => Promise<void>;
};

export type CopyTextResult = "copied" | "empty" | "unsupported" | "failed";

export function getCopyableText(text: string | null | undefined): string | null {
  const value = text?.trim();
  return value ? value : null;
}

export async function copyTextToClipboard(
  text: string | null | undefined,
  writer: ClipboardWriter | null | undefined = globalThis.navigator?.clipboard,
): Promise<CopyTextResult> {
  const value = getCopyableText(text);

  if (!value) {
    return "empty";
  }

  if (!writer?.writeText) {
    return "unsupported";
  }

  try {
    await writer.writeText(value);
    return "copied";
  } catch {
    return "failed";
  }
}
