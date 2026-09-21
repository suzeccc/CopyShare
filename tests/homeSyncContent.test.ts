import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";

const home = readFileSync("src/pages/Home.vue", "utf8");
const source = home.match(/async function saveSyncContent\([\s\S]*?\n\}/)?.[0];
assert.ok(source);
const errors: string[] = [];
const store = {
  config: { syncText: true, syncImage: true, syncFiles: true, deviceName: "QA" },
  saving: false, error: null as string | null, calls: 0,
  async save(next: typeof this.config) { this.calls++; if (!this.error) this.config = next; },
};
const save = new Function("configStore", "toastStore", ts.transpile(source) + ";return saveSyncContent;")(store, { error: (message: string) => errors.push(message) });
for (const key of ["syncText", "syncImage", "syncFiles"] as const) {
  await save(key, false);
  assert.equal(store.config[key], false);
}
assert.equal(store.config.deviceName, "QA");
await save("syncFiles", false);
assert.equal(store.calls, 3, "unchanged values should not be saved");
store.saving = true;
await save("syncText", true);
assert.equal(store.calls, 3, "do not overlap saves");
store.saving = false;
store.error = "disk error";
await save("syncText", true);
assert.equal(store.config.syncText, false, "a failed save must preserve the previous state");
assert.deepEqual(errors, ["保存失败"]);
const switchSource = readFileSync("src/components/ui/Switch.vue", "utf8").match(/function updateValue\([\s\S]*?\n\}/)?.[0];
assert.ok(switchSource);
let requested: boolean | undefined;
const change = new Function("props", "emit", ts.transpile(switchSource) + ";return updateValue;")({ modelValue: true }, (_: string, value: boolean) => { requested = value; });
const input = { checked: false };
change({ target: input });
assert.equal(requested, false);
assert.equal(input.checked, true, "the checkbox must follow its saved model while an asynchronous save is pending or failed");
for (const path of ["src/pages/Settings.vue", "src-tauri/src/commands.rs"]) {
  assert.doesNotMatch(readFileSync(path, "utf8"), /syncText: true|next_config\.sync_text = true/);
}
