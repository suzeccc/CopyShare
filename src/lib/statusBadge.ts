import type { SyncState } from "@/types/status";

type ConnectionBadgeView = {
  containerClass: string;
  dotClass: string;
};

const badgeViews: Record<SyncState, ConnectionBadgeView> = {
  running: {
    containerClass: "border-emerald-400/50 bg-emerald-400/10 text-emerald-200",
    dotClass: "bg-emerald-300 shadow-[0_0_10px_rgba(110,231,183,0.45)]",
  },
  stopped: {
    containerClass: "border-white/55 bg-white/[0.08] text-white",
    dotClass: "bg-white shadow-[0_0_10px_rgba(255,255,255,0.32)]",
  },
  error: {
    containerClass: "border-red-400/50 bg-red-400/10 text-red-100",
    dotClass: "bg-red-300",
  },
};

export function getConnectionBadgeView(state: SyncState): ConnectionBadgeView {
  return badgeViews[state];
}
