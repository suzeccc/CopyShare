export type LibraryLayout = "grid" | "list";

export const LIBRARY_LAYOUT_STORAGE_KEY = "copyshare:library-layout";

type LibraryLayoutStorage = Pick<Storage, "getItem" | "setItem">;

export function readLibraryLayout(
  storage: LibraryLayoutStorage = window.localStorage,
): LibraryLayout {
  return storage.getItem(LIBRARY_LAYOUT_STORAGE_KEY) === "list" ? "list" : "grid";
}

export function writeLibraryLayout(
  layout: LibraryLayout,
  storage: LibraryLayoutStorage = window.localStorage,
): void {
  storage.setItem(LIBRARY_LAYOUT_STORAGE_KEY, layout);
}
