import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const app = readFileSync("src/App.vue", "utf8");
const wizard = readFileSync("src/components/onboarding/FirstRunWizard.vue", "utf8");
const configType = readFileSync("src/types/config.ts", "utf8");
const configStore = readFileSync("src/stores/config.ts", "utf8");
const shell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const rustModels = readFileSync("src-tauri/src/models.rs", "utf8");
const rustConfig = readFileSync("src-tauri/src/config.rs", "utf8");

assert.match(configType, /onboardingCompleted: boolean/);
assert.match(configStore, /onboardingCompleted: false/);
assert.match(rustModels, /pub onboarding_completed: bool/);
assert.match(rustConfig, /config\.onboarding_completed = true/);

assert.match(app, /import FirstRunWizard/);
assert.match(app, /!configStore\.config\.onboardingCompleted/);
assert.match(app, /<FirstRunWizard v-if="onboardingVisible"/);

assert.match(wizard, /data-onboarding-wizard/);
assert.match(wizard, /data-onboarding-device-step/);
assert.match(wizard, /data-onboarding-storage-step/);
assert.match(wizard, /data-onboarding-language/);
assert.match(wizard, /select:not\(:disabled\)/);
assert.match(wizard, /data-onboarding-connect-step/);
assert.match(wizard, /data-onboarding-lan-primary/);
assert.match(wizard, /data-onboarding-scan/);
assert.match(wizard, /data-onboarding-manual-fallback/);
assert.match(wizard, /data-onboarding-ip-dialog/);
assert.doesNotMatch(wizard, /data-onboarding-mobile|<MobileConnectDialog/);
assert.match(wizard, /import Button from "@\/components\/ui\/Button\.vue"/);
assert.match(wizard, /<ManualConnectForm/);
assert.match(wizard, /data-onboarding-discovered-devices/);
assert.match(wizard, /selectTransferSaveDir/);
assert.match(wizard, /getTransferSaveDir/);
assert.match(wizard, /displayedTransferSaveDir/);
assert.match(wizard, /onboardingCompleted: true/);
assert.match(wizard, /router\.push\("\/devices"\)/);
assert.match(wizard, /@media \(prefers-reduced-motion: reduce\)/);

const settings = readFileSync("src/pages/Settings.vue", "utf8");
const commands = readFileSync("src-tauri/src/commands.rs", "utf8");
assert.doesNotMatch(settings, /data-new-user-guide-setting|guideOpen|FirstRunWizard/);
assert.match(shell, /keepFloatingAfterOnboarding/);
assert.match(shell, /switchWindowMode\("floating", \(\) => enterFloatingWindow\("top-right"\)\)/);
const connectStep = wizard.slice(wizard.indexOf("data-onboarding-connect-step"));
assert.ok(connectStep.indexOf("data-onboarding-lan-primary") < connectStep.indexOf("data-onboarding-discovered-devices"));
assert.ok(connectStep.indexOf("data-onboarding-discovered-devices") < connectStep.indexOf("data-onboarding-manual-fallback"));
assert.doesNotMatch(wizard, /<details|两台电脑如何互相信任|开始使用 CopyShare|当前设置/);
assert.match(wizard, /data-onboarding-dismiss/);
assert.match(wizard, /\.onboarding-scroll \{[^}]*min-height: 0;[^}]*overflow-y: auto;/);
assert.match(wizard, /\.onboarding-content footer \{[^}]*flex-shrink: 0;/);
const picker = commands.match(/pub async fn select_transfer_save_dir\([\s\S]*?\n\}/)?.[0] ?? "";
assert.match(picker, /if persist\.unwrap_or\(true\) \{\s*config::save_config[\s\S]*state\.set_config[\s\S]*app\.emit/);
assert.doesNotMatch(picker.slice(picker.indexOf("    Ok(Some(next_config))")), /save_config|set_config|emit/);

const logic = wizard.slice(wizard.indexOf("function previousStep()"), wizard.indexOf("</script>"));
function scenario(review = false) {
  const saved: Record<string, unknown>[] = [];
  const routes: string[] = [];
  const closed: string[] = [];
  const original = { deviceName: "Original", fileSaveDir: null, autoStart: false, autoSync: true, uiLanguage: "system", translationProxy: "keep", onboardingCompleted: review };
  const configStore = {
    config: { ...original }, saving: false, error: null as string | null, fail: false,
    async save(next: typeof original) { saved.push(next); this.error = this.fail ? "disk failure" : null; if (!this.error) this.config = next; },
  };
  const context = vm.createContext({
    Error,
    props: { review }, configStore, draft: { ...original },
    step: { value: 1 }, canContinue: { value: true },
    selectingDirectory: { value: false }, finishing: { value: false }, saveError: { value: "" },
    selectTransferSaveDir: async (persist: boolean) => { assert.equal(persist, false); return { ...original, fileSaveDir: "D:/chosen" }; },
    router: { push: async (route: string) => { routes.push(route); } },
    emit: (event: string) => { closed.push(event); }, toastStore: { error() {} },
  });
  context.busy = { get value() { return context.selectingDirectory.value || context.finishing.value || configStore.saving; } };
  vm.runInContext(ts.transpile(logic), context);
  return { context, saved, routes, closed, original, configStore };
}

const setup = scenario();
await vm.runInContext("chooseDirectory()", setup.context);
assert.equal(setup.context.draft.fileSaveDir, "D:/chosen");
assert.deepEqual(setup.configStore.config, setup.original, "directory selection must not write config");
assert.equal(setup.saved.length, 0);
setup.context.draft.deviceName = "  Study  ";
setup.context.draft.autoStart = true;
await vm.runInContext("finish()", setup.context);
assert.deepEqual(JSON.parse(JSON.stringify(setup.saved)), [{ ...setup.original, deviceName: "Study", fileSaveDir: "D:/chosen", autoStart: true, onboardingCompleted: true }]);
assert.deepEqual(setup.routes, ["/devices"]);
assert.deepEqual(setup.closed, ["close"]);

const skip = scenario();
skip.context.draft.deviceName = "";
skip.context.draft.fileSaveDir = "D:/discarded";
skip.context.draft.autoStart = true;
await vm.runInContext("finish(true)", skip.context);
assert.deepEqual(JSON.parse(JSON.stringify(skip.saved)), [{ ...skip.original, onboardingCompleted: true }]);
assert.deepEqual(skip.routes, []);
assert.deepEqual(skip.closed, ["close"]);

const guide = scenario(true);
await vm.runInContext("chooseDirectory()", guide.context);
guide.context.draft.deviceName = "  Bedroom  ";
guide.context.draft.autoSync = false;
await vm.runInContext("finish()", guide.context);
assert.deepEqual(JSON.parse(JSON.stringify(guide.saved)), [{ ...guide.original, deviceName: "Bedroom", fileSaveDir: "D:/chosen", autoSync: false }]);
assert.deepEqual(guide.routes, ["/devices"]);
assert.deepEqual(guide.closed, ["close"]);

const dismissedGuide = scenario(true);
dismissedGuide.context.draft.deviceName = "Discarded";
await vm.runInContext("finish(true)", dismissedGuide.context);
assert.equal(dismissedGuide.saved.length, 0);
assert.deepEqual(dismissedGuide.closed, ["close"]);

const steps = scenario();
await vm.runInContext("nextStep()", steps.context);
assert.equal(steps.context.step.value, 2);
assert.equal(steps.saved.length, 0);
steps.context.draft.deviceName = "  Living room  ";
steps.context.draft.autoStart = true;
steps.context.draft.uiLanguage = "ja-JP";
await vm.runInContext("nextStep()", steps.context);
assert.equal(steps.context.step.value, 3);
assert.deepEqual(JSON.parse(JSON.stringify(steps.saved)), [{ ...steps.original, deviceName: "Living room", autoStart: true, uiLanguage: "ja-JP" }]);
assert.equal(steps.configStore.config.onboardingCompleted, false);

const failedStep = scenario();
failedStep.context.step.value = 2;
failedStep.configStore.fail = true;
await vm.runInContext("nextStep()", failedStep.context);
assert.equal(failedStep.context.step.value, 2, "failed save must keep the user on the editable step");
assert.equal(failedStep.context.saveError.value, "disk failure");

const failed = scenario();
failed.configStore.fail = true;
await vm.runInContext("finish()", failed.context);
assert.equal(failed.context.finishing.value, false);
assert.equal(failed.context.saveError.value, "disk failure");
assert.deepEqual(failed.closed, []);
assert.deepEqual(failed.routes, []);
assert.equal(failed.configStore.config.onboardingCompleted, false);
failed.configStore.fail = false;
await vm.runInContext("finish()", failed.context);
assert.deepEqual(failed.closed, ["close"], "failed save must allow retry");

const cancelled = scenario();
cancelled.context.selectTransferSaveDir = async () => null;
await vm.runInContext("chooseDirectory()", cancelled.context);
assert.equal(cancelled.context.draft.fileSaveDir, null);
cancelled.context.selectTransferSaveDir = async () => { throw new Error("picker unavailable"); };
await vm.runInContext("chooseDirectory()", cancelled.context);
assert.equal(cancelled.context.saveError.value, "picker unavailable");
assert.equal(cancelled.context.selectingDirectory.value, false);
assert.equal(cancelled.saved.length, 0);
cancelled.context.draft.deviceName = " ";
await vm.runInContext("finish()", cancelled.context);
assert.equal(cancelled.saved.length, 0);
assert.equal(cancelled.context.step.value, 1);

const noDevices = scenario();
noDevices.context.step.value = 3;
noDevices.context.lanDiscoveryScanning = { value: false };
noDevices.context.scanNotice = { value: "" };
noDevices.context.manualConnectAvailable = { value: false };
noDevices.context.showManualConnectDialog = { value: false };
noDevices.context.visibleDevices = { value: [] };
noDevices.context.devicesStore = { scanLanDevices: async () => ({ total: 0, newCount: 0 }) };
await vm.runInContext("scanLanDevices()", noDevices.context);
assert.equal(noDevices.context.manualConnectAvailable.value, true);
assert.equal(noDevices.context.showManualConnectDialog.value, true);
assert.equal(noDevices.context.lanDiscoveryScanning.value, false);

const foundDevices = scenario();
foundDevices.context.step.value = 3;
foundDevices.context.lanDiscoveryScanning = { value: false };
foundDevices.context.scanNotice = { value: "" };
foundDevices.context.manualConnectAvailable = { value: false };
foundDevices.context.showManualConnectDialog = { value: false };
foundDevices.context.visibleDevices = { value: [{ id: "device-1" }] };
foundDevices.context.devicesStore = { scanLanDevices: async () => ({ total: 1, newCount: 1 }) };
await vm.runInContext("scanLanDevices()", foundDevices.context);
assert.equal(foundDevices.context.showManualConnectDialog.value, false);

// Tab wraps within the dialog, including its connection controls.
const focus = scenario();
const controls = [0, 1, 2].map(() => ({ offsetParent: {}, focus() { focus.context.document.activeElement = this; } }));
focus.context.dialogRef = { value: { querySelectorAll: () => controls } };
focus.context.showManualConnectDialog = { value: false };
focus.context.document = { activeElement: controls[2] };
let prevented = 0;
focus.context.event = { shiftKey: false, preventDefault() { prevented++; } };
const trap = wizard.match(/function trapFocus\([\s\S]*?\n\}/)?.[0] ?? "";
vm.runInContext(ts.transpile(trap), focus.context);
vm.runInContext("trapFocus(event)", focus.context);
assert.equal(focus.context.document.activeElement, controls[0]);
focus.context.event.shiftKey = true;
vm.runInContext("trapFocus(event)", focus.context);
assert.equal(focus.context.document.activeElement, controls[2]);
assert.equal(prevented, 2);
