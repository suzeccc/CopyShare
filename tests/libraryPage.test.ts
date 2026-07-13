import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

test("library page, navigation, cards and dialogs expose the complete phase-one UI", () => {
  const paths = {
    page: "src/pages/Library.vue",
    card: "src/components/library/LibraryCard.vue",
    snippet: "src/components/library/SnippetEditorDialog.vue",
    metadata: "src/components/library/LibraryMetadataDialog.vue",
  };
  for (const path of Object.values(paths)) assert.equal(existsSync(path), true, path);

  const router = readFileSync("src/router/index.ts", "utf8");
  const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");
  const page = readFileSync(paths.page, "utf8");
  const card = readFileSync(paths.card, "utf8");
  const snippet = readFileSync(paths.snippet, "utf8");
  const metadata = readFileSync(paths.metadata, "utf8");

  assert.match(router, /import Library from "@\/pages\/Library\.vue"/);
  assert.match(router, /path: "\/library", name: "library", component: Library/);
  assert.match(sidebar, /MessageSquareText/);
  assert.match(sidebar, /label: "常用片段", path: "\/library", icon: MessageSquareText/);
  assert.ok(sidebar.indexOf('label: "设备连接"') < sidebar.indexOf('label: "常用片段"'));
  assert.ok(sidebar.indexOf('label: "常用片段"') < sidebar.indexOf('label: "图片转文字"'));

  const snippetView = page.indexOf('{ value: "snippets", label: "常用片段" }');
  const allView = page.indexOf('{ value: "all", label: "全部收藏" }');
  const pinnedView = page.indexOf('{ value: "pinned", label: "已置顶" }');
  assert.ok(snippetView < allView && allView < pinnedView);
  assert.match(page, /const activeHeader = computed/);
  assert.match(page, /v-if="activeView === 'snippets'"/);
  assert.match(page, /<component :is="activeHeader\.icon"/);
  assert.match(page, /\{\{ activeHeader\.title \}\}/);
  assert.match(page, /\{\{ activeHeader\.description \}\}/);
  assert.match(page, /LayoutGrid/);
  assert.match(page, /List/);
  assert.match(page, /const libraryLayout = ref<LibraryLayout>\(readLibraryLayout\(\)\)/);
  assert.match(page, /writeLibraryLayout\(layout\)/);
  assert.match(page, /data-library-layout-grid/);
  assert.match(page, /data-library-layout-list/);
  assert.match(page, /:aria-pressed="libraryLayout === 'grid'"/);
  assert.match(page, /:aria-pressed="libraryLayout === 'list'"/);
  assert.match(page, /:layout="libraryLayout"/);
  assert.match(page, /libraryLayout === 'grid'[\s\S]*?md:grid-cols-2[\s\S]*?2xl:grid-cols-3/);
  assert.match(page, /libraryLayout === 'list'[\s\S]*?'grid gap-2'/);

  for (const hook of [
    "data-library-page",
    "data-library-view-all",
    "data-library-view-pinned",
    "data-library-view-snippets",
    "data-library-search",
    "data-library-type-filter",
    "data-library-tag-filter",
    "data-library-new-snippet",
    "data-library-storage-size",
    "data-library-warning",
    "data-library-list",
    "data-library-empty",
    "dropPinnedItem",
  ]) assert.match(page, new RegExp(hook));

  for (const hook of [
    "data-library-card",
    "data-library-copy",
    "data-library-pin",
    "data-library-edit",
    "data-library-convert-snippet",
    "data-library-edit-snippet",
    "data-library-remove",
  ]) assert.match(card, new RegExp(hook));
  assert.doesNotMatch(card, /data-library-export/);
  assert.match(snippet, /titleError/);
  assert.match(snippet, /contentError/);
  assert.match(snippet, /submitted && titleError/);
  assert.match(snippet, /submitted && contentError/);
  assert.match(metadata, /tagsText/);
  assert.match(metadata, /emit\("submit"/);
});
