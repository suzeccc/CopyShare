import type { AppWindowMode } from "@/lib/windowMode";

export type WindowTransitionPhase =
  | "idle"
  | "main-exit"
  | "floating-enter"
  | "floating-exit"
  | "main-enter";

export const WINDOW_MODE_EXIT_MS = 120;
export const WINDOW_MODE_ENTER_MS = 170;

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
