import assert from "node:assert/strict";

import {
  createManualConnectDraft,
  setManualConnectDraftIp,
  setManualConnectDraftPort,
} from "../src/lib/manualConnectDraft.ts";

const draft = createManualConnectDraft(8766);
assert.deepEqual(draft, { ip: "", port: 8766 });

const withIp = setManualConnectDraftIp(draft, "  10.194.33.156  ");
assert.deepEqual(withIp, { ip: "10.194.33.156", port: 8766 });

const withPort = setManualConnectDraftPort(withIp, 70000);
assert.deepEqual(withPort, { ip: "10.194.33.156", port: 65535 });

assert.deepEqual(setManualConnectDraftPort(withIp, Number.NaN), {
  ip: "10.194.33.156",
  port: 8765,
});
