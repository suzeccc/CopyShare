import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");
const router = readFileSync("src/router/index.ts", "utf8");
const about = readFileSync("src/pages/About.vue", "utf8");
const aboutMeta = readFileSync("src/lib/about.ts", "utf8");

assert.match(sidebar, /label: "关于"/);
assert.match(sidebar, /path: "\/about"/);
assert.match(sidebar, /Info/);

assert.match(router, /const About = \(\) => import\("@\/pages\/About\.vue"\)/);
assert.match(router, /path: "\/about"/);
assert.match(router, /name: "about"/);

assert.match(aboutMeta, /APP_VERSION/);
assert.match(aboutMeta, /GITHUB_REPOSITORY_URL\s*=\s*"https:\/\/github\.com\/suzeccc\/CopyShare"/);
assert.match(aboutMeta, /AUTHOR_NAME\s*=\s*"suzecc"/);
assert.match(aboutMeta, /UPDATE_URL\s*=\s*`\$\{GITHUB_REPOSITORY_URL\}\/releases\/latest`/);

assert.match(about, /关于 CopyShare/);
assert.match(about, /版本信息/);
assert.match(about, /GitHub 仓库/);
assert.match(about, /作者/);
assert.match(about, /检查更新/);
assert.match(about, /checkForUpdate/);
assert.match(about, /useUpdaterStore/);
assert.match(about, /updater\.downloadUpdate/);
assert.match(about, /updater\.installUpdate/);
assert.match(about, /data-update-progress/);
assert.match(about, /data-update-notes/);
assert.match(about, /<p data-i18n-ignore[^>]*>{{ updater\.notes }}<\/p>/);
assert.doesNotMatch(about, /v-html/);
assert.match(about, /openExternalUrl/);
assert.match(about, /openRepository/);
assert.match(about, /openLatestRelease/);
assert.match(about, /data-github-star-hint/);
assert.match(about, /一颗 Star/);
assert.doesNotMatch(about, /window\.open/);
assert.doesNotMatch(about, /<a :href="UPDATE_URL"[\s\S]*检查更新/);
assert.match(about, /APP_VERSION/);
assert.match(about, /GITHUB_REPOSITORY_URL/);
assert.match(about, /AUTHOR_NAME/);
assert.match(about, /UPDATE_URL/);
assert.doesNotMatch(about, /<a :href="GITHUB_REPOSITORY_URL"/);
assert.doesNotMatch(about, /<a :href="UPDATE_URL"/);
