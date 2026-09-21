import assert from "node:assert/strict";
import { createPinia, setActivePinia } from "pinia";
import { useUpdaterStore } from "../src/stores/updater.ts";

// Exercise the real updater client and store; only the native IPC boundary is mocked.
const calls: string[] = [];
let checkFailure = false;
let downloadFailure = false;
let installFailure = false;
let transferActive = false;
let restartFailure = false;
let candidate: object | null = null;
let finishDownload: (() => void) | undefined;
let callbackId = 0;
Object.assign(globalThis, { window: {
  __TAURI_INTERNALS__: {
    transformCallback: () => ++callbackId,
    async invoke(command: string, args: any) {
      calls.push(command);
      switch (command) {
        case "plugin:updater|check":
          if (checkFailure) throw new Error("offline");
          return candidate;
        case "plugin:updater|download":
          args.onEvent.onmessage({ event: "Started", data: { contentLength: 100 } });
          args.onEvent.onmessage({ event: "Progress", data: { chunkLength: 40 } });
          await new Promise<void>((resolve) => { finishDownload = resolve; });
          args.onEvent.onmessage({ event: "Progress", data: { chunkLength: 60 } });
          args.onEvent.onmessage({ event: "Finished" });
          if (downloadFailure) throw new Error("invalid signature");
          return 101;
        case "prepare_app_update":
          if (transferActive) throw new Error("active transfer");
          return;
        case "plugin:updater|install":
          assert.equal(args.bytesRid, 101);
          assert.equal(args.restartAfterInstall, true);
          if (installFailure) throw new Error("installer unavailable");
          return;
        case "restart_app":
          if (restartFailure) throw new Error("restart unavailable");
          return;
        case "plugin:resources|close": return;
        default: throw new Error(`Unexpected IPC: ${command}`);
      }
    },
  },
} });
setActivePinia(createPinia());
const store = useUpdaterStore();

checkFailure = true;
await store.checkForUpdate(true);
assert.equal(store.phase, "idle");
assert.equal(store.error, null, "startup network failures stay silent");
await store.checkForUpdate();
assert.match(store.error!, /offline/);
checkFailure = false;
await store.checkForUpdate();
assert.equal(store.phase, "idle");
assert.match(store.message!, /已是最新版本/);

candidate = { rid: 100, currentVersion: "3.4.0", version: "3.4.1",
  body: "<script>untrusted release notes</script>", rawJson: {} };
await store.checkForUpdate();
assert.equal(store.phase, "available");
assert.equal(store.version, "3.4.1");
assert.equal(store.notes, "<script>untrusted release notes</script>");
await store.installUpdate();
assert.ok(!calls.includes("plugin:updater|install"), "cannot install before verification");

downloadFailure = true;
const failedDownload = store.downloadUpdate();
assert.equal(store.phase, "downloading");
assert.equal(store.progress, 40);
await store.downloadUpdate();
assert.equal(calls.filter(c => c === "plugin:updater|download").length, 1, "duplicate download is ignored");
finishDownload!();
await failedDownload;
assert.equal(store.phase, "available");
assert.match(store.error!, /invalid signature/);
await store.installUpdate();
assert.ok(!calls.includes("plugin:updater|install"), "bad signatures cannot reach installer");

downloadFailure = false;
const downloaded = store.downloadUpdate();
finishDownload!();
await downloaded;
assert.equal(store.phase, "ready");
assert.equal(store.progress, 100);
const checkCount = calls.filter(c => c === "plugin:updater|check").length;
await store.checkForUpdate();
assert.equal(calls.filter(c => c === "plugin:updater|check").length, checkCount,
  "rechecking must not discard a verified package");

transferActive = true;
await store.installUpdate();
assert.equal(store.phase, "ready");
assert.match(store.error!, /active transfer/);
assert.ok(!calls.includes("plugin:updater|install"), "busy transfers prevent native installation");
transferActive = false;
installFailure = true;
await store.installUpdate();
assert.equal(store.phase, "ready", "failed installation retains verified download for retry");
assert.ok(!calls.includes("restart_app"));
installFailure = false;
restartFailure = true;
await store.installUpdate();
assert.equal(store.phase, "installed");
assert.match(store.error!, /手动重新启动/);
assert.deepEqual(calls.slice(-3), ["prepare_app_update", "plugin:updater|install", "restart_app"]);
restartFailure = false;
await store.restartApp();
assert.equal(calls.at(-1), "restart_app");
assert.equal(store.error, null);
