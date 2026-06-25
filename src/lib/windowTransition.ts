import type { AppWindowMode } from "@/lib/windowMode";

export type WindowTransitionPhase =
  | "idle"
  | "main-exit"
  | "floating-enter"
  | "floating-exit"
  | "main-enter";

export const WINDOW_MODE_EXIT_MS = 120;
export const WINDOW_MODE_ENTER_MS = 170;

export type WindowTransitionPointer = {
  clientX: number;
  clientY: number;
};

type WindowTransitionRect = {
  left: number;
  top: number;
  width: number;
  height: number;
};

export function getWindowModeTransition(
  currentMode: AppWindowMode,
  nextMode: AppWindowMode,
): { exitPhase: WindowTransitionPhase; enterPhase: WindowTransitionPhase } | null {
  if (currentMode === nextMode) {
    return null;
  }

  return currentMode === "main"
    ? { exitPhase: "main-exit", enterPhase: "floating-enter" }
    : { exitPhase: "floating-exit", enterPhase: "main-enter" };
}

export function getWindowTransitionOrigin(
  pointer: WindowTransitionPointer,
  rect: WindowTransitionRect,
): string {
  const x = clamp(Math.round(pointer.clientX - rect.left), 0, rect.width);
  const y = clamp(Math.round(pointer.clientY - rect.top), 0, rect.height);

  return `${x}px ${y}px`;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
