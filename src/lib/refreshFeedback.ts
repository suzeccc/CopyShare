export type RefreshFeedbackState = "idle" | "refreshing" | "done";

type RefreshFeedbackView = {
  label: string;
  iconClass: string;
  buttonClass: string;
  disabled: boolean;
};

const views: Record<RefreshFeedbackState, RefreshFeedbackView> = {
  idle: {
    label: "刷新",
    iconClass: "",
    buttonClass: "",
    disabled: false,
  },
  refreshing: {
    label: "刷新中",
    iconClass: "animate-spin text-[color:var(--accent-text)]",
    buttonClass: "border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]",
    disabled: true,
  },
  done: {
    label: "已刷新",
    iconClass: "text-emerald-100",
    buttonClass: "border-emerald-300/50 bg-emerald-500/[0.14] text-emerald-100",
    disabled: false,
  },
};

export function getRefreshFeedbackView(state: RefreshFeedbackState): RefreshFeedbackView {
  return views[state];
}
