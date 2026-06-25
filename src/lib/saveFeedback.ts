export type SaveFeedbackState = "idle" | "saving" | "saved" | "error";

type SaveFeedbackView = {
  label: string;
  iconClass: string;
  buttonClass: string;
  disabled: boolean;
};

const views: Record<SaveFeedbackState, SaveFeedbackView> = {
  idle: {
    label: "保存设置",
    iconClass: "",
    buttonClass: "",
    disabled: false,
  },
  saving: {
    label: "保存中",
    iconClass: "animate-spin text-[color:var(--accent-text)]",
    buttonClass: "border-[color:var(--accent-line)] bg-[color:var(--accent-soft)] text-[color:var(--accent-text)]",
    disabled: true,
  },
  saved: {
    label: "已保存",
    iconClass: "text-emerald-100",
    buttonClass: "border-emerald-300/50 bg-emerald-500/[0.14] text-emerald-100",
    disabled: false,
  },
  error: {
    label: "保存失败",
    iconClass: "text-red-100",
    buttonClass: "border-red-400/50 bg-red-500/[0.14] text-red-100",
    disabled: false,
  },
};

export function getSaveFeedbackView(state: SaveFeedbackState): SaveFeedbackView {
  return views[state];
}
