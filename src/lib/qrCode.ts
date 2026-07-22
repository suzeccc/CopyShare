type QrCodeApi = typeof import("qrcode");

let qrCodeModule: Promise<QrCodeApi> | undefined;

export async function createMobileQrCodeDataUrl(url: string | undefined): Promise<string> {
  if (!url) {
    return "";
  }

  qrCodeModule ??= import("qrcode").then((module) =>
    (module as unknown as { default?: QrCodeApi }).default ?? module);
  const QRCode = await qrCodeModule;
  return QRCode.toDataURL(url, {
    margin: 1,
    width: 232,
    color: { dark: "#020617", light: "#ffffff" },
  });
}
