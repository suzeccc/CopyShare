import unittest
from pathlib import Path


class UiProMaxThemeTests(unittest.TestCase):
    def test_tk_ui_uses_dark_tray_console_theme(self) -> None:
        source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")

        self.assertIn("PRO_MAX_THEME", source)
        self.assertIn("Dark Tray Console Glass", source)
        self.assertIn("WINDOW_ALPHA = 1.0", source)
        self.assertIn('self.root.geometry("1120x680")', source)
        self.assertIn("self.root.minsize(980, 620)", source)
        for token in ("#162231", "#26313D", "#344454", "#C7D5E6", "#F8FAFC", "#22C55E", "#EF4444"):
            self.assertIn(token, source)
        for light_token in ("#EEF5FF", "#F6FAFF", "#F8FBFF", "#0052FF", "#4D7CFF", "#FEF2F2"):
            self.assertNotIn(light_token, source)
        self.assertNotIn("Real-Time Operations Glass", source)

    def test_windows_title_bar_matches_dark_tray_theme(self) -> None:
        source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")

        self.assertIn("def apply_dark_title_bar", source)
        self.assertIn("DWMWA_USE_IMMERSIVE_DARK_MODE = 20", source)
        self.assertIn("DWMWA_USE_IMMERSIVE_DARK_MODE_BEFORE_20H1 = 19", source)
        self.assertIn("DWMWA_BORDER_COLOR = 34", source)
        self.assertIn("DWMWA_CAPTION_COLOR = 35", source)
        self.assertIn("DWMWA_TEXT_COLOR = 36", source)
        self.assertIn('dwm_set_window_attribute(hwnd, DWMWA_CAPTION_COLOR, "#162231")', source)
        self.assertIn('dwm_set_window_attribute(hwnd, DWMWA_BORDER_COLOR, "#344454")', source)
        self.assertIn('dwm_set_window_attribute(hwnd, DWMWA_TEXT_COLOR, "#F8FAFC")', source)
        self.assertIn("apply_dark_title_bar(window, hwnd)", source)

    def test_preview_uses_same_dark_tray_console_direction(self) -> None:
        html = Path(".superpowers/brainstorm/ui-stable/index.html").read_text(encoding="utf-8")

        self.assertIn("UI UX Pro Max", html)
        self.assertIn("Dark Tray Console Glass", html)
        self.assertIn("--tray-bg: #162231", html)
        self.assertIn("--surface: #26313D", html)
        self.assertIn("--line: #344454", html)
        self.assertIn("--ink: #F8FAFC", html)
        self.assertIn("--muted: #C7D5E6", html)
        self.assertIn("--glass: rgba(38, 49, 61, .82)", html)
        self.assertIn("backdrop-filter: blur(24px) saturate(170%)", html)
        for token in ("#162231", "#26313D", "#344454", "#C7D5E6", "#F8FAFC", "#22C55E", "#EF4444"):
            self.assertIn(token, html)
        for light_token in ("#EEF5FF", "#0052FF", "#4D7CFF", "rgba(255, 255, 255, .54)", "rgba(255,255,255,.58)"):
            self.assertNotIn(light_token, html)
        self.assertNotIn("Real-Time Operations Glass", html)

    def test_typography_is_refined_for_chinese_glass_ui(self) -> None:
        source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")
        html = Path(".superpowers/brainstorm/ui-stable/index.html").read_text(encoding="utf-8")

        self.assertIn('FONT_UI = "Microsoft YaHei UI"', source)
        self.assertIn('FONT_DATA = "Cascadia Mono"', source)
        for token in ('"button"', '"panel_title"', '"port_label"', '"port_value"', '"port_status"'):
            self.assertIn(token, source)
        self.assertIn("--font-ui:", html)
        self.assertIn('--font-ui: "Microsoft YaHei UI"', html)
        self.assertNotIn('--font-ui: "Inter"', html)
        self.assertIn("Microsoft YaHei UI", html)
        self.assertIn("Noto Sans SC", html)
        self.assertIn("--font-data:", html)
        self.assertIn("font-variant-numeric: tabular-nums;", html)

    def test_numbers_use_softer_ui_font_and_smaller_scale(self) -> None:
        source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")
        html = Path(".superpowers/brainstorm/ui-stable/index.html").read_text(encoding="utf-8")

        self.assertIn('"metric": (FONT_UI, 20, "bold")', source)
        self.assertIn('"port_value": (FONT_UI, 26, "bold")', source)
        self.assertNotIn('"metric": (FONT_DATA, 24, "bold")', source)
        self.assertNotIn('"port_value": (FONT_DATA, 31, "bold")', source)
        self.assertIn(".side-card b {", html)
        self.assertIn("font-family: var(--font-ui);", html)
        self.assertIn("font-size: 29px;", html)
        self.assertIn(".metric b {", html)
        self.assertIn("font-size: 20px;", html)
        self.assertNotIn("font-size: 34px;", html)
        self.assertNotIn("font-size: 24px;", html)

    def test_controls_use_rounded_pill_glass_shapes(self) -> None:
        source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")
        html = Path(".superpowers/brainstorm/ui-stable/index.html").read_text(encoding="utf-8")

        self.assertIn("CARD_RADIUS = 22", source)
        self.assertIn("PILL_RADIUS = 999", source)
        self.assertIn("BUTTON_PADDING = (24, 10)", source)
        self.assertIn("ENTRY_PADDING = (16, 10)", source)
        self.assertIn("--radius-card: 22px;", html)
        self.assertIn("--radius-pill: 999px;", html)
        self.assertIn("border-radius: var(--radius-card);", html)
        self.assertIn("border-radius: var(--radius-pill);", html)
        for old_radius in (
            "border-radius: 8px;",
            "border-radius: 10px;",
        ):
            self.assertNotIn(old_radius, html)

    def test_dashboard_values_and_actions_use_centered_alignment(self) -> None:
        source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")
        html = Path(".superpowers/brainstorm/ui-stable/index.html").read_text(encoding="utf-8")

        self.assertIn('label.pack(anchor="center")', source)
        self.assertIn('anchor="center", pady=(8, 0)', source)
        self.assertIn('font=GLASS_FONTS["button"]', source)
        self.assertIn(".metric {", html)
        self.assertIn("text-align: center;", html)
        self.assertIn("place-items: center;", html)

    def test_click_feedback_avoids_synchronous_redraw_jank(self) -> None:
        source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")

        self.assertIn("def _queue_redraw", source)
        self.assertIn("_redraw_scheduled", source)
        self.assertIn("_last_draw_state", source)
        self.assertIn("self.after_idle", source)
        self.assertIn("ROUND_SPLINESTEPS = 8", source)
        self.assertNotIn("self._redraw()\n        if was_pressed", source)


if __name__ == "__main__":
    unittest.main()
