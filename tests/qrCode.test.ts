import assert from "node:assert/strict";
import test from "node:test";

import { createMobileQrCodeDataUrl } from "../src/lib/qrCode.ts";

test("mobile QR rendering skips empty input and supports the lazy qrcode module", async () => {
  assert.equal(await createMobileQrCodeDataUrl(undefined), "");
  assert.match(
    await createMobileQrCodeDataUrl("https://copyshare.test/mobile"),
    /^data:image\/png;base64,/,
  );
});
