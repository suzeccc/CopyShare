type ClosestTarget = {
  closest: (selector: string) => Element | null;
};

type DragMouseEvent = Pick<MouseEvent, "button" | "target">;
type StartWindowDragMouseEvent = DragMouseEvent &
  Pick<MouseEvent, "preventDefault" | "stopPropagation">;
type StartDragging = () => Promise<void>;

let activeWindowDrag: Promise<void> | null = null;

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

export function startWindowDragFromMouseEvent(
  event: StartWindowDragMouseEvent,
  startDragging: StartDragging,
): boolean {
  if (!shouldStartWindowDrag(event)) {
    return false;
  }

  event.preventDefault();
  event.stopPropagation();

  if (activeWindowDrag) {
    return false;
  }

  let nextDrag: Promise<void>;
  try {
    nextDrag = startDragging();
  } catch (error) {
    nextDrag = Promise.reject(error);
  }
  activeWindowDrag = nextDrag;

  void nextDrag
    .catch(() => undefined)
    .finally(() => {
      if (activeWindowDrag === nextDrag) {
        activeWindowDrag = null;
      }
    });

  return true;
}
