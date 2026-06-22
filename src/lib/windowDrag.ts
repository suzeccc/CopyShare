type ClosestTarget = {
  closest: (selector: string) => Element | null;
};

type DragMouseEvent = Pick<MouseEvent, "button" | "target">;

const WINDOW_DRAG_IGNORE_SELECTOR = [
  "button",
  "a",
  "input",
  "textarea",
  "select",
  "option",
  "label",
  "summary",
  "[role='button']",
  "[role='link']",
  "[contenteditable='true']",
  "[data-window-control]",
  "[data-no-window-drag]",
].join(",");

const WINDOW_DRAG_REGION_SELECTOR = [
  "[data-tauri-drag-region]",
  "[data-window-drag-region]",
].join(",");

function getClosestTarget(target: EventTarget | null): ClosestTarget | null {
  if (!target) {
    return null;
  }

  const directTarget = target as Partial<ClosestTarget>;
  if (typeof directTarget.closest === "function") {
    return directTarget as ClosestTarget;
  }

  const parentElement = (target as { parentElement?: ClosestTarget | null }).parentElement;
  if (typeof parentElement?.closest === "function") {
    return parentElement;
  }

  return null;
}

export function shouldStartWindowDrag(event: DragMouseEvent): boolean {
  if (event.button !== 0) {
    return false;
  }

  const closestTarget = getClosestTarget(event.target);
  if (!closestTarget) {
    return false;
  }

  if (closestTarget.closest(WINDOW_DRAG_IGNORE_SELECTOR)) {
    return false;
  }

  return Boolean(closestTarget.closest(WINDOW_DRAG_REGION_SELECTOR));
}
