import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const settings = readFileSync("src/pages/Settings.vue", "utf8");
const tauriApi = readFileSync("src/lib/tauri.ts", "utf8");
const tauriLib = readFileSync("src-tauri/src/lib.rs", "utf8");
const commands = readFileSync("src-tauri/src/commands.rs", "utf8");

test("settings page groups startup inside basic settings and cache management inside storage", () => {
  assert.match(settings, /data-cache-management-settings/);
  assert.match(settings, />缓存管理</);
  assert.match(settings, /缓存大小计算中/);
  assert.match(settings, /刷新大小/);
  assert.match(settings, /清除缓存/);
  assert.match(settings, /包含图片历史、图片缩略图、视频缩略图等本地缓存/);

  const startupStart = settings.indexOf("data-startup-settings");
  const basicStart = settings.indexOf('data-settings-image2-section="basic"');
  const storageStart = settings.indexOf("data-storage-settings");
  const cacheStart = settings.indexOf("data-cache-management-settings");
  const syncStart = settings.indexOf(">同步内容<");
  assert.equal(startupStart >= 0, true);
  assert.equal(basicStart >= 0, true);
  assert.equal(storageStart >= 0, true);
  assert.equal(cacheStart >= 0, true);
  assert.equal(syncStart >= 0, true);
  assert.equal(basicStart < startupStart && startupStart < storageStart, true);
  assert.equal(storageStart < cacheStart, true);
  assert.equal(cacheStart < syncStart, true);
  assert.match(
    settings,
    /<section data-storage-settings[\s\S]*?data-download-location-setting[\s\S]*?data-cache-management-settings[\s\S]*?<\/section>/,
  );
  assert.doesNotMatch(settings, /<section data-cache-management-settings/);
});

test("cache management uses dedicated Tauri commands and refreshes after clearing", () => {
  assert.match(settings, /import \{[\s\S]*clearCache[\s\S]*getCacheSize[\s\S]*\} from "@\/lib\/tauri"/);
  assert.match(settings, /const cacheSizeBytes = ref<number \| null>\(null\)/);
  assert.match(settings, /const cacheSizeLoading = ref\(false\)/);
  assert.match(settings, /const cacheClearing = ref\(false\)/);
  assert.match(settings, /async function loadCacheSize\(\)/);
  assert.match(settings, /async function clearLocalCache\(\)/);
  assert.match(settings, /cacheSizeBytes\.value = await getCacheSize\(\)/);
  assert.match(settings, /cacheSizeBytes\.value = await clearCache\(\)/);
  assert.match(settings, /onMounted\(\(\) => \{[^}]*\bloadCacheSize\(\)/);
});

test("Tauri API exposes cache commands", () => {
  assert.match(tauriApi, /export function getCacheSize\(\): Promise<number>/);
  assert.match(tauriApi, /invoke<number>\("get_cache_size"\)/);
  assert.match(tauriApi, /export function clearCache\(\): Promise<number>/);
  assert.match(tauriApi, /invoke<number>\("clear_cache"\)/);

  assert.match(commands, /pub async fn get_cache_size\(app: AppHandle\) -> AppResult<u64>/);
  assert.match(commands, /pub async fn clear_cache\(app: AppHandle, state: State<'_, AppState>\) -> AppResult<u64>/);
  assert.match(tauriLib, /commands::get_cache_size/);
  assert.match(tauriLib, /commands::clear_cache/);
});
