import assert from "node:assert/strict";

import {
  getAutomaticUpdateMessage,
  getStartupUpdatePrompt,
  getUpdateState,
  normalizeVersion,
  RELEASE_API_URL,
} from "../src/lib/about.ts";

assert.equal(normalizeVersion("v1.6.0"), "1.6.0");
assert.equal(normalizeVersion("  release-1.6.0 "), "1.6.0");

assert.deepEqual(
  getUpdateState("1.6.0", { version: "v1.6.0", url: "https://example.test/v1.6.0" }),
  { hasUpdate: false, latestVersion: "1.6.0", updateUrl: "https://example.test/v1.6.0" },
);

assert.deepEqual(
  getUpdateState("1.6.0", { version: "v1.6.1", url: "https://example.test/v1.6.1" }),
  { hasUpdate: true, latestVersion: "1.6.1", updateUrl: "https://example.test/v1.6.1" },
);

assert.deepEqual(
  getUpdateState("1.6.0", { version: "1.5.9", url: "https://example.test/v1.5.9" }),
  { hasUpdate: false, latestVersion: "1.5.9", updateUrl: "https://example.test/v1.5.9" },
);

assert.equal(
  getAutomaticUpdateMessage("1.6.0", {
    version: "v1.6.1",
    url: "https://example.test/v1.6.1",
  }),
  "发现新版本 v1.6.1，请前往关于页查看。",
);
assert.equal(
  getAutomaticUpdateMessage("1.6.0", {
    version: "v1.6.0",
    url: "https://example.test/v1.6.0",
  }),
  null,
);

assert.deepEqual(
  getStartupUpdatePrompt("1.6.0", {
    version: "v1.6.1",
    url: "https://example.test/v1.6.1",
  }),
  {
    currentVersion: "1.6.0",
    latestVersion: "1.6.1",
    updateUrl: "https://example.test/v1.6.1",
  },
);
assert.equal(
  getStartupUpdatePrompt("1.6.0", {
    version: "v1.6.0",
    url: "https://example.test/v1.6.0",
  }),
  null,
);

assert.match(RELEASE_API_URL, /api\.github\.com\/repos\/suzeccc\/Copy-share\/releases\/latest/);
