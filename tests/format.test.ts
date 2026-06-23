import assert from "node:assert/strict";

import { deviceAddress } from "../src/lib/format.ts";

assert.equal(deviceAddress("10.194.33.156", 8765), "10.194.33.156:8765");
assert.equal(deviceAddress("ws://10.194.33.156:8765/", 8765), "10.194.33.156:8765");
assert.equal(deviceAddress("10.194.33.156:8765", 8765), "10.194.33.156:8765");
