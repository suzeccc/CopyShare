import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const settings = readFileSync("src/pages/Settings.vue", "utf8");

assert.match(settings, /const configMutationSaving = computed\(\(\) =>/);
for (const flag of [
  "configStore.saving",
  "basicSettingsSaving.value",
  "syncContentSaving.value",
  "notificationSettingsSaving.value",
  "downloadLocationSaving.value",
]) {
  assert.match(settings, new RegExp(flag.replace(".", "\\.")));
}

assert.ok(
  (settings.match(/:disabled="configMutationSaving/g) ?? []).length >= 15,
  "all config-backed controls should share one mutation lock",
);
assert.doesNotMatch(settings, /:disabled="basicSettingsSaving"/);
assert.doesNotMatch(settings, /:disabled="syncContentSaving"/);
assert.doesNotMatch(settings, /:disabled="notificationSettingsSaving/);

assert.match(settings, /function restoreDraftFromConfig/);
assert.match(
  settings,
  /if \(configStore\.saving[\s\S]*restoreDraftFromConfig\(\);[\s\S]*return;/,
);
