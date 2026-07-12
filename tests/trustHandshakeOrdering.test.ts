import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const syncSource = readFileSync("src-tauri/src/sync.rs", "utf8");
const registerPeerIndex = syncSource.indexOf("state.register_peer(connection_id, sender, join).await;");
const releaseSocketIndex = syncSource.indexOf("start_sender.send(())");

assert.notEqual(registerPeerIndex, -1, "socket peer must be registered");
assert.notEqual(releaseSocketIndex, -1, "socket task must wait for registration");
assert.ok(
  registerPeerIndex < releaseSocketIndex,
  "the socket task must start only after its outbound sender is registered",
);
