import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const home = readFileSync("src/pages/Home.vue", "utf8");

assert.match(home, /data-home-device-fields/);
assert.match(home, /sm:grid-cols-\[0\.62fr_1\.38fr\]/);
assert.match(home, /data-home-port-block/);
assert.match(home, /data-home-address-block/);
assert.match(home, /data-home-address-value/);

const addressValue = home.match(
  /<span[^>]*data-home-address-value[^>]*class="([^"]+)"/,
);

assert.ok(addressValue, "address value span should be easy to inspect");
assert.doesNotMatch(addressValue[1], /\btruncate\b/);
assert.match(addressValue[1], /\bwhitespace-nowrap\b/);
