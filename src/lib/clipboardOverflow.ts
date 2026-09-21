import type { ObjectDirective } from "vue";
import { shouldShowClipboardItemMore } from "./historyPreview";

const observers = new WeakMap<HTMLElement, ResizeObserver>();

function updateMoreButton(row: HTMLElement) {
  const text = row.querySelector<HTMLElement>("[data-clipboard-preview-text]");
  const button = row.querySelector<HTMLElement>("[data-floating-clipboard-item-more-button]");
  if (button) {
    // Measure without the button so content that fits can use all available space.
    button.style.display = "none";
    if (text && shouldShowClipboardItemMore(text)) button.style.removeProperty("display");
  }
}

export const vClipboardOverflow: ObjectDirective<HTMLElement> = {
  mounted(row) {
    const observer = new ResizeObserver(() => updateMoreButton(row));
    observer.observe(row);
    observers.set(row, observer);
    updateMoreButton(row);
  },
  updated: updateMoreButton,
  unmounted(row) {
    observers.get(row)?.disconnect();
    observers.delete(row);
  },
};
