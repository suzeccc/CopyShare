from __future__ import annotations

import argparse
import asyncio
from collections.abc import Callable
from datetime import datetime
import logging
import threading
import tkinter as tk
from tkinter import ttk

from ..app import ClipboardSyncApp
from .controller import SyncUiController
from .state import PeerStatus, SyncUiState


PRO_MAX_THEME = "UI UX Pro Max / Dark Tray Console Glass"
WINDOW_ALPHA = 1.0
FONT_UI = "Microsoft YaHei UI"
FONT_DATA = "Cascadia Mono"
CARD_RADIUS = 22
PILL_RADIUS = 999
ROUND_SPLINESTEPS = 8
BUTTON_PADDING = (24, 10)
ENTRY_PADDING = (16, 10)

GLASS_COLORS = {
    "mist": "#162231",
    "shell": "#162231",
    "sidebar": "#162231",
    "sidebar_hover": "#26313D",
    "glass_panel": "#26313D",
    "glass_panel_alt": "#2D3A48",
    "border": "#344454",
    "ink": "#F8FAFC",
    "muted": "#C7D5E6",
    "blue": "#22C55E",
    "blue_end": "#86EFAC",
    "blue_soft": "#26313D",
    "green": "#22C55E",
    "yellow": "#FACC15",
    "red": "#EF4444",
    "white": "#F8FAFC",
    "entry": "#1B2938",
    "tray_bg": "#162231",
    "tray_panel": "#26313D",
    "tray_border": "#344454",
    "tray_muted": "#C7D5E6",
}

GLASS_FONTS = {
    "title": (FONT_UI, 22, "bold"),
    "subtitle": (FONT_UI, 10, "bold"),
    "brand": (FONT_UI, 18, "bold"),
    "nav": (FONT_UI, 11, "bold"),
    "metric": (FONT_UI, 20, "bold"),
    "metric_caption": (FONT_UI, 10, "bold"),
    "panel_title": (FONT_UI, 13, "bold"),
    "body": (FONT_UI, 10),
    "body_bold": (FONT_UI, 10, "bold"),
    "button": (FONT_UI, 11, "bold"),
    "mono": (FONT_DATA, 10),
    "port_label": (FONT_UI, 12, "bold"),
    "port_value": (FONT_UI, 26, "bold"),
    "port_hint": (FONT_UI, 10, "bold"),
    "port_status": (FONT_UI, 14, "bold"),
}

GlassNavButton = "GlassNavButton"

DWMWA_USE_IMMERSIVE_DARK_MODE_BEFORE_20H1 = 19
DWMWA_USE_IMMERSIVE_DARK_MODE = 20
DWMWA_BORDER_COLOR = 34
DWMWA_CAPTION_COLOR = 35
DWMWA_TEXT_COLOR = 36
DWMWA_SYSTEMBACKDROP_TYPE = 38


class RoundedButton(tk.Canvas):
    def __init__(
        self,
        parent: tk.Widget,
        text: str,
        command: Callable[[], None],
        *,
        style: str = "TButton",
        canvas_bg: str | None = None,
    ) -> None:
        super().__init__(
            parent,
            height=46,
            width=150,
            bg=canvas_bg or GLASS_COLORS["tray_bg"],
            highlightthickness=0,
            borderwidth=0,
            cursor="hand2",
        )
        self._text = text
        self._command = command
        self._style = style
        self._pressed = False
        self._hover = False
        self._redraw_scheduled = False
        self._last_draw_state: tuple[int, int, str, bool, bool, str] | None = None
        self.bind("<Configure>", lambda _event: self._queue_redraw(force=True))
        self.bind("<ButtonPress-1>", self._press)
        self.bind("<ButtonRelease-1>", self._release)
        self.bind("<Enter>", lambda _event: self._queue_redraw(hover=True))
        self.bind("<Leave>", lambda _event: self._queue_redraw(hover=False))

    def configure(self, cnf: object = None, **kwargs: object) -> object:
        if isinstance(cnf, str):
            return super().configure(cnf)
        options = dict(cnf or {}) if isinstance(cnf, dict) else {}
        options.update(kwargs)
        if "text" in options:
            self._text = str(options.pop("text"))
        if "style" in options:
            self._style = str(options.pop("style"))
        result = super().configure(**options) if options else None
        self._queue_redraw(force=True)
        return result

    config = configure

    def _press(self, _event: tk.Event) -> None:
        self._pressed = True
        self._queue_redraw(hover=True, immediate=True)

    def _release(self, event: tk.Event) -> None:
        was_pressed = self._pressed
        self._pressed = False
        inside = was_pressed and 0 <= event.x <= self.winfo_width() and 0 <= event.y <= self.winfo_height()
        if inside:
            self._command()
            self._queue_redraw()
        else:
            self._queue_redraw(immediate=True)

    def _queue_redraw(self, hover: bool | None = None, *, force: bool = False, immediate: bool = False) -> None:
        if hover is not None:
            self._hover = hover
        if force:
            self._last_draw_state = None
        if immediate:
            self._redraw()
            return
        if self._redraw_scheduled:
            return
        self._redraw_scheduled = True
        try:
            self.after_idle(self._redraw_from_idle)
        except tk.TclError:
            self._redraw_scheduled = False

    def _redraw_from_idle(self) -> None:
        self._redraw_scheduled = False
        if self.winfo_exists():
            self._redraw()

    def _redraw(self, hover: bool | None = None) -> None:
        if hover is not None:
            self._hover = hover
        width = max(self.winfo_width(), 1)
        height = max(self.winfo_height(), 1)
        draw_state = (width, height, self._style, self._pressed, self._hover, self._text)
        if draw_state == self._last_draw_state:
            return
        self._last_draw_state = draw_state
        self.delete("all")
        colors = GLASS_COLORS
        if self._style == "Danger.TButton":
            border = colors["red"]
            foreground = colors["red"]
        elif self._style == "Primary.TButton":
            border = colors["green"]
            foreground = colors["white"]
        else:
            border = colors["green"] if self._hover else colors["tray_border"]
            foreground = colors["ink"]
        fill = colors["tray_border"] if (self._hover or self._pressed) else colors["tray_panel"]
        self._draw_rounded_rect(1, 1, width - 1, height - 1, min(height // 2, 23), fill=fill, outline=border, width=1)
        self.create_text(width // 2, height // 2, text=self._text, fill=foreground, font=GLASS_FONTS["button"])

    def _draw_rounded_rect(self, x1: int, y1: int, x2: int, y2: int, radius: int, **kwargs: object) -> int:
        points = [
            x1 + radius, y1, x2 - radius, y1, x2, y1, x2, y1 + radius,
            x2, y2 - radius, x2, y2, x2 - radius, y2, x1 + radius, y2,
            x1, y2, x1, y2 - radius, x1, y1 + radius, x1, y1,
        ]
        return self.create_polygon(points, smooth=True, splinesteps=ROUND_SPLINESTEPS, **kwargs)


class RoundedEntry(tk.Canvas):
    def __init__(self, parent: tk.Widget, textvariable: tk.StringVar, *, font: tuple[str, int]) -> None:
        super().__init__(
            parent,
            height=46,
            bg=GLASS_COLORS["glass_panel"],
            highlightthickness=0,
            borderwidth=0,
        )
        self.entry = tk.Entry(
            self,
            textvariable=textvariable,
            font=font,
            bg=GLASS_COLORS["entry"],
            fg=GLASS_COLORS["ink"],
            insertbackground=GLASS_COLORS["green"],
            relief="flat",
            borderwidth=0,
        )
        self._window = self.create_window(16, 23, anchor="w", window=self.entry)
        self.bind("<Configure>", self._redraw)
        self.bind("<Button-1>", lambda _event: self.entry.focus_set())

    def _redraw(self, event: tk.Event) -> None:
        self.delete("shell")
        width = max(event.width, 1)
        height = max(event.height, 1)
        self._draw_rounded_rect(1, 1, width - 1, height - 1, min(height // 2, 23), tags="shell", fill=GLASS_COLORS["entry"], outline=GLASS_COLORS["border"], width=1)
        self.tag_lower("shell")
        self.itemconfigure(self._window, width=max(1, width - 32), height=max(1, height - 18))

    def _draw_rounded_rect(self, x1: int, y1: int, x2: int, y2: int, radius: int, **kwargs: object) -> int:
        points = [
            x1 + radius, y1, x2 - radius, y1, x2, y1, x2, y1 + radius,
            x2, y2 - radius, x2, y2, x2 - radius, y2, x1 + radius, y2,
            x1, y2, x1, y2 - radius, x1, y1 + radius, x1, y1,
        ]
        return self.create_polygon(points, smooth=True, splinesteps=ROUND_SPLINESTEPS, **kwargs)


def _hex_to_colorref(color: str) -> int:
    value = color.lstrip("#")
    red = int(value[0:2], 16)
    green = int(value[2:4], 16)
    blue = int(value[4:6], 16)
    return red | (green << 8) | (blue << 16)


def dwm_set_window_attribute(hwnd: int, attribute: int, value: int | str) -> None:
    import ctypes

    raw_value = _hex_to_colorref(value) if isinstance(value, str) else value
    data = ctypes.c_int(raw_value)
    ctypes.windll.dwmapi.DwmSetWindowAttribute(
        hwnd,
        attribute,
        ctypes.byref(data),
        ctypes.sizeof(data),
    )


def apply_dark_title_bar(window: tk.Misc, hwnd: int) -> None:
    """Match the native Windows title bar to the tray console theme."""
    try:
        if window.tk.call("tk", "windowingsystem") != "win32":
            return
        dwm_set_window_attribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, 1)
        dwm_set_window_attribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE_BEFORE_20H1, 1)
        dwm_set_window_attribute(hwnd, DWMWA_CAPTION_COLOR, "#162231")
        dwm_set_window_attribute(hwnd, DWMWA_BORDER_COLOR, "#344454")
        dwm_set_window_attribute(hwnd, DWMWA_TEXT_COLOR, "#F8FAFC")
    except Exception:
        return


def apply_window_backdrop(window: tk.Misc) -> None:
    """Best-effort Windows Mica backdrop; regular Tk styling is the fallback."""
    try:
        if window.tk.call("tk", "windowingsystem") != "win32":
            return

        import ctypes

        window.update_idletasks()
        hwnd = ctypes.windll.user32.GetParent(window.winfo_id())
        backdrop_type = ctypes.c_int(2)
        ctypes.windll.dwmapi.DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            ctypes.byref(backdrop_type),
            ctypes.sizeof(backdrop_type),
        )
        apply_dark_title_bar(window, hwnd)
    except Exception:
        return


def apply_window_alpha(window: tk.Misc) -> None:
    """Keep text readable; Mica/backdrop handles the glass impression."""
    try:
        window.attributes("-alpha", WINDOW_ALPHA)
    except tk.TclError:
        return


class UiLogHandler(logging.Handler):
    def __init__(self, log: Callable[[str, str], None]) -> None:
        super().__init__()
        self.log = log

    def emit(self, record: logging.LogRecord) -> None:
        if record.levelno >= logging.ERROR:
            level = "error"
        elif record.levelno >= logging.WARNING:
            level = "warn"
        else:
            level = "info"
        self.log(record.getMessage(), level)


class BackgroundSyncRuntime:
    def __init__(self, device_id: str, log: Callable[[str, str], None] | None = None) -> None:
        self.device_id = device_id
        self.log = log or (lambda message, level="info": None)
        self._stop_requested = threading.Event()
        self._thread: threading.Thread | None = None

    def start(self, port: int, peers: list[str]) -> None:
        if self._thread and self._thread.is_alive():
            self.stop()
        self._stop_requested = threading.Event()
        self._thread = threading.Thread(
            target=self._run_thread,
            args=(port, peers, self._stop_requested),
            name="lan-clipboard-sync-ui-runtime",
            daemon=True,
        )
        self._thread.start()

    def stop(self) -> None:
        self._stop_requested.set()
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=3)

    def set_device_id(self, device_id: str) -> None:
        self.device_id = device_id

    def _run_thread(self, port: int, peers: list[str], stop_event: threading.Event) -> None:
        try:
            asyncio.run(self._run_app(port, peers, stop_event))
        except Exception as exc:
            self.log(f"同步核心运行失败：{exc}", "error")

    def _make_ui_logger(self) -> logging.Logger:
        logger = logging.getLogger(f"lan_clipboard_sync.ui.runtime.{id(self)}")
        logger.handlers.clear()
        logger.setLevel(logging.INFO)
        logger.propagate = False
        logger.addHandler(UiLogHandler(self.log))
        return logger

    async def _run_app(self, port: int, peers: list[str], stop_event: threading.Event) -> None:
        logger = self._make_ui_logger()
        app = ClipboardSyncApp(
            device_id=self.device_id,
            host="0.0.0.0",
            port=port,
            peers=peers,
            poll_interval=0.1,
            reconnect_delay=2.0,
            logger=logger,
        )
        await app.start()
        self.log(f"同步核心已监听端口 {port}")
        try:
            while not stop_event.is_set():
                await asyncio.sleep(0.1)
        finally:
            await app.stop()
            self.log("同步核心已停止")


class SyncDesktopUi:
    def __init__(self, root: tk.Tk, controller: SyncUiController, initial_port: int, initial_peers: list[str]) -> None:
        self.root = root
        self.controller = controller
        self.initial_port = initial_port
        self.initial_peers = initial_peers
        self.compact_window: tk.Toplevel | None = None
        self._compact_mode_active = False
        self._closing = False
        self._pending_view: str | None = None
        self._view_switch_after_id: str | None = None
        self._visible_view: str | None = None
        self._view_widgets: dict[str, list[tk.Widget]] = {}
        self._view_refs: dict[str, dict[str, object]] = {}
        self.peer_var = tk.StringVar(value=self.initial_peers[0] if self.initial_peers else "")
        self.port_var = tk.StringVar(value=str(self.initial_port))
        self.device_id_var = tk.StringVar(value=self.controller.device_id)
        self.nav_buttons: dict[str, tk.Button] = {}
        self.status_label: ttk.Label | None = None
        self.peer_list: tk.Listbox | None = None
        self.log_list: tk.Listbox | None = None
        self.primary_sync_button: ttk.Button | None = None
        self.pause_button: ttk.Button | None = None
        self.port_card_items: dict[str, int] = {}
        self.metric_labels: dict[str, ttk.Label] = {}

        for peer in initial_peers:
            self.controller.add_peer(peer)
        self.controller.apply_settings(port_text=str(initial_port), device_id_text=self.device_id_var.get())

        self.root.title("剪贴板同步")
        self.root.geometry("1120x680")
        self.root.minsize(980, 620)
        self.root.configure(bg=GLASS_COLORS["tray_bg"])
        apply_window_backdrop(self.root)
        apply_window_alpha(self.root)
        self.root.bind("<Unmap>", self._handle_main_window_unmap, add="+")

        self._configure_style()
        self._build_shell()
        self._show_view("overview")

    def _configure_style(self) -> None:
        style = ttk.Style(self.root)
        style.theme_use("clam")
        colors = GLASS_COLORS
        style.configure("GlassRoot.TFrame", background=colors["tray_bg"])
        style.configure(
            "GlassSidebar.TFrame",
            background=colors["sidebar"],
            bordercolor=colors["border"],
            relief="solid",
            borderwidth=1,
        )
        style.configure(
            "GlassPanel.TFrame",
            background=colors["glass_panel"],
            bordercolor=colors["border"],
            relief="solid",
            borderwidth=1,
        )
        style.configure("GlassTitle.TLabel", background=colors["tray_bg"], foreground=colors["ink"], font=GLASS_FONTS["title"])
        style.configure("GlassMuted.TLabel", background=colors["tray_bg"], foreground=colors["muted"], font=GLASS_FONTS["subtitle"])
        style.configure("GlassPanelTitle.TLabel", background=colors["glass_panel"], foreground=colors["ink"], font=GLASS_FONTS["panel_title"])
        style.configure("GlassPanelText.TLabel", background=colors["glass_panel"], foreground=colors["ink"], font=GLASS_FONTS["body"])
        style.configure("GlassPanelMuted.TLabel", background=colors["glass_panel"], foreground=colors["muted"], font=GLASS_FONTS["subtitle"])
        style.configure("GlassSidebarTitle.TLabel", background=colors["sidebar"], foreground=colors["ink"], font=GLASS_FONTS["brand"])
        style.configure("GlassSidebar.TLabel", background=colors["sidebar"], foreground=colors["muted"], font=GLASS_FONTS["subtitle"])
        style.configure("GlassMetric.TLabel", background=colors["glass_panel"], foreground=colors["ink"], font=GLASS_FONTS["metric"])
        style.configure("GlassMetricCaption.TLabel", background=colors["glass_panel"], foreground=colors["muted"], font=GLASS_FONTS["metric_caption"])
        style.configure(
            "Glass.TEntry",
            fieldbackground=colors["entry"],
            foreground=colors["ink"],
            insertcolor=colors["green"],
            bordercolor=colors["border"],
            lightcolor=colors["border"],
            darkcolor=colors["border"],
            padding=ENTRY_PADDING,
        )
        style.configure(
            "TButton",
            padding=BUTTON_PADDING,
            font=GLASS_FONTS["button"],
            background=colors["tray_panel"],
            foreground=colors["ink"],
            bordercolor=colors["tray_border"],
            lightcolor=colors["tray_border"],
            darkcolor=colors["tray_border"],
            relief="flat",
            borderwidth=1,
        )
        style.map(
            "TButton",
            background=[("active", colors["tray_border"])],
            foreground=[("active", colors["ink"])],
            bordercolor=[("active", colors["green"])],
        )
        style.configure("Primary.TButton", background=colors["tray_panel"], foreground=colors["white"], font=GLASS_FONTS["button"], bordercolor=colors["green"], padding=BUTTON_PADDING)
        style.map("Primary.TButton", background=[("active", colors["tray_border"])], foreground=[("active", colors["white"])], bordercolor=[("active", colors["green"])])
        style.configure("Danger.TButton", background=colors["tray_panel"], foreground=colors["red"], font=GLASS_FONTS["button"], bordercolor=colors["red"], padding=BUTTON_PADDING)
        style.map("Danger.TButton", background=[("active", colors["tray_border"])], foreground=[("active", colors["red"])], bordercolor=[("active", colors["red"])])

        style.configure("Root.TFrame", background=colors["tray_bg"])
        style.configure("Panel.TFrame", background=colors["glass_panel"], bordercolor=colors["border"], relief="solid", borderwidth=1)
        style.configure("Muted.TLabel", background=colors["tray_bg"], foreground=colors["muted"], font=GLASS_FONTS["subtitle"])

    def _build_shell(self) -> None:
        shell = ttk.Frame(self.root, style="GlassRoot.TFrame", padding=14)
        shell.pack(fill="both", expand=True)
        shell.columnconfigure(1, weight=1)
        shell.rowconfigure(0, weight=1)

        sidebar = ttk.Frame(shell, style="GlassSidebar.TFrame", padding=16)
        sidebar.grid(row=0, column=0, sticky="nsw")
        sidebar.columnconfigure(0, minsize=188)

        ttk.Label(sidebar, text="剪贴板同步", style="GlassSidebarTitle.TLabel").pack(anchor="w")
        ttk.Label(sidebar, text="局域网剪贴板同步工具", style="GlassSidebar.TLabel").pack(anchor="w", pady=(4, 18))

        self.nav_buttons["overview"] = self._make_nav_button(sidebar, "overview", "总览")
        self.nav_buttons["devices"] = self._make_nav_button(sidebar, "devices", "设备连接")
        self.nav_buttons["history"] = self._make_nav_button(sidebar, "history", "剪贴历史")
        self.nav_buttons["settings"] = self._make_nav_button(sidebar, "settings", "设置")
        for button in self.nav_buttons.values():
            button.pack(anchor="w", fill="x", pady=4)

        self._build_port_status_card(sidebar)

        self.main = ttk.Frame(shell, style="GlassRoot.TFrame")
        self.main.grid(row=0, column=1, sticky="nsew", padx=(14, 0))
        self.main.columnconfigure(0, weight=1)
        self.main.rowconfigure(2, weight=1)

    def _rounded_panel(self, parent: tk.Widget, padding: int | tuple[int, int] = 12) -> tuple[tk.Canvas, tk.Frame]:
        pad_x, pad_y = padding if isinstance(padding, tuple) else (padding, padding)
        canvas = tk.Canvas(parent, width=1, height=1, bg=GLASS_COLORS["tray_bg"], highlightthickness=0, borderwidth=0)
        panel = tk.Frame(canvas, bg=GLASS_COLORS["glass_panel"])
        window = canvas.create_window(pad_x, pad_y, anchor="nw", window=panel)

        def redraw(event: tk.Event) -> None:
            canvas.delete("rounded_panel")
            self._draw_rounded_rect(
                canvas,
                1,
                1,
                max(1, event.width - 1),
                max(1, event.height - 1),
                CARD_RADIUS,
                tags="rounded_panel",
                fill=GLASS_COLORS["glass_panel"],
                outline=GLASS_COLORS["border"],
                width=1,
            )
            canvas.tag_lower("rounded_panel")
            canvas.itemconfigure(
                window,
                width=max(1, event.width - pad_x * 2),
                height=max(1, event.height - pad_y * 2),
            )

        canvas.bind("<Configure>", redraw)
        return canvas, panel

    def _build_port_status_card(self, parent: ttk.Frame) -> None:
        colors = GLASS_COLORS
        self.port_card_canvas = tk.Canvas(
            parent,
            width=188,
            height=154,
            bg=colors["sidebar"],
            highlightthickness=0,
            borderwidth=0,
        )
        self.port_card_canvas.pack(anchor="center", fill="x", side="bottom", pady=(18, 0))
        self._draw_rounded_rect(
            self.port_card_canvas,
            2,
            2,
            186,
            152,
            CARD_RADIUS,
            fill=colors["glass_panel"],
            outline=colors["border"],
            width=1,
        )
        self.port_card_items["label"] = self.port_card_canvas.create_text(
            24,
            29,
            text="本机端口",
            anchor="w",
            fill=colors["muted"],
            font=GLASS_FONTS["port_label"],
        )
        self.port_card_items["port"] = self.port_card_canvas.create_text(
            24,
            69,
            text="8765",
            anchor="w",
            fill=colors["white"],
            font=GLASS_FONTS["port_value"],
        )
        self.port_card_items["hint"] = self.port_card_canvas.create_text(
            24,
            103,
            text="等待局域网设备连接",
            anchor="w",
            fill=colors["muted"],
            font=GLASS_FONTS["port_hint"],
        )
        self.port_card_items["status"] = self.port_card_canvas.create_text(
            24,
            131,
            text="未启动",
            anchor="w",
            fill=colors["red"],
            font=GLASS_FONTS["port_status"],
        )

    def _draw_rounded_rect(
        self,
        canvas: tk.Canvas,
        x1: int,
        y1: int,
        x2: int,
        y2: int,
        radius: int,
        **kwargs: object,
    ) -> int:
        points = [
            x1 + radius,
            y1,
            x2 - radius,
            y1,
            x2,
            y1,
            x2,
            y1 + radius,
            x2,
            y2 - radius,
            x2,
            y2,
            x2 - radius,
            y2,
            x1 + radius,
            y2,
            x1,
            y2,
            x1,
            y2 - radius,
            x1,
            y1 + radius,
            x1,
            y1,
        ]
        return canvas.create_polygon(points, smooth=True, splinesteps=ROUND_SPLINESTEPS, **kwargs)

    def _make_nav_button(self, parent: tk.Widget, view: str, label: str) -> tk.Button:
        button = tk.Button(
            parent,
            text=label,
            command=lambda selected=view: self._request_view_switch(selected),
            cursor="hand2",
            anchor="w",
            padx=12,
            pady=8,
            borderwidth=0,
            relief="flat",
            font=GLASS_FONTS["nav"],
            takefocus=True,
        )
        button.bind("<Enter>", lambda _event, item=button: self._style_single_nav_button(item, hover=True))
        button.bind("<Leave>", lambda _event: self._style_nav_buttons())
        return button

    def _style_nav_buttons(self) -> None:
        active_view = self.controller.state.active_view
        for view, button in self.nav_buttons.items():
            self._style_single_nav_button(button, active=view == active_view)

    def _style_single_nav_button(self, button: tk.Button, *, active: bool = False, hover: bool = False) -> None:
        colors = GLASS_COLORS
        if active:
            background = colors["blue_soft"]
            foreground = colors["ink"]
            highlight = colors["blue"]
        elif hover:
            background = colors["sidebar_hover"]
            foreground = colors["ink"]
            highlight = colors["border"]
        else:
            background = colors["sidebar"]
            foreground = colors["muted"]
            highlight = colors["sidebar"]
        button.configure(
            bg=background,
            fg=foreground,
            activebackground=colors["blue_soft"],
            activeforeground=colors["ink"],
            highlightthickness=1,
            highlightbackground=highlight,
            highlightcolor=highlight,
        )

    def _request_view_switch(self, view: str) -> None:
        if self._visible_view == view and self.controller.state.active_view == view:
            return
        if not self.controller.set_view(view):
            return
        self._style_nav_buttons()
        self._pending_view = view
        if self._view_switch_after_id is not None:
            try:
                self.root.after_cancel(self._view_switch_after_id)
            except tk.TclError:
                pass
        try:
            self._view_switch_after_id = self.root.after_idle(self._flush_pending_view)
        except tk.TclError:
            self._view_switch_after_id = None
            self._show_view(view)

    def _flush_pending_view(self) -> None:
        view = self._pending_view
        self._pending_view = None
        self._view_switch_after_id = None
        if view is not None:
            self._show_view(view)

    def _cancel_pending_view_switch(self) -> None:
        if self._view_switch_after_id is None:
            return
        try:
            self.root.after_cancel(self._view_switch_after_id)
        except tk.TclError:
            pass
        self._view_switch_after_id = None
        self._pending_view = None

    def _show_view(self, view: str) -> None:
        if self.controller.state.active_view == view and self.main.winfo_children():
            if self._visible_view == view:
                return
        elif not self.controller.set_view(view):
            return
        self._cancel_pending_view_switch()
        self._hide_visible_view()
        self._reset_main_rows()
        if self._view_cache_alive(view):
            self._restore_view_refs(view)
            self._apply_main_row_layout(view)
            for child in self._view_widgets[view]:
                child.grid()
        else:
            existing_children = set(self.main.winfo_children())
            self._reset_view_refs()
            {
                "overview": self._build_overview_view,
                "devices": self._build_devices_view,
                "history": self._build_history_view,
                "settings": self._build_settings_view,
            }[view]()
            self._view_widgets[view] = [child for child in self.main.winfo_children() if child not in existing_children]
            self._store_view_refs(view)
        self._visible_view = view
        self._style_nav_buttons()
        self._refresh()

    def _hide_visible_view(self) -> None:
        if self._visible_view is None:
            return
        for child in self._view_widgets.get(self._visible_view, []):
            if child.winfo_exists():
                child.grid_remove()

    def _reset_main_rows(self) -> None:
        for row in range(6):
            self.main.rowconfigure(row, weight=0, minsize=0)

    def _reset_view_refs(self) -> None:
        self.status_label = None
        self.peer_list = None
        self.log_list = None
        self.primary_sync_button = None
        self.pause_button = None
        self.metric_labels = {}

    def _view_cache_alive(self, view: str) -> bool:
        widgets = self._view_widgets.get(view)
        return bool(widgets) and all(widget.winfo_exists() for widget in widgets)

    def _store_view_refs(self, view: str) -> None:
        self._view_refs[view] = {
            "status_label": self.status_label,
            "peer_list": self.peer_list,
            "log_list": self.log_list,
            "primary_sync_button": self.primary_sync_button,
            "pause_button": self.pause_button,
            "metric_labels": dict(self.metric_labels),
        }

    def _restore_view_refs(self, view: str) -> None:
        refs = self._view_refs.get(view, {})
        self.status_label = refs.get("status_label")
        self.peer_list = refs.get("peer_list")
        self.log_list = refs.get("log_list")
        self.primary_sync_button = refs.get("primary_sync_button")
        self.pause_button = refs.get("pause_button")
        self.metric_labels = refs.get("metric_labels", {})

    def _apply_main_row_layout(self, view: str) -> None:
        if view == "overview":
            self.main.rowconfigure(2, weight=1)
        elif view in {"devices", "history"}:
            self.main.rowconfigure(1, weight=1)

    def _header(self, title: str, subtitle: str) -> ttk.Frame:
        top = ttk.Frame(self.main, style="GlassRoot.TFrame")
        top.grid(row=0, column=0, sticky="ew")
        top.columnconfigure(0, weight=1)
        ttk.Label(top, text=title, style="GlassTitle.TLabel").grid(row=0, column=0, sticky="w")
        self.status_label = ttk.Label(top, text=subtitle, style="GlassMuted.TLabel")
        self.status_label.grid(row=1, column=0, sticky="w", pady=(4, 0))
        return top

    def _build_overview_view(self) -> None:
        # overview-reference-layout-lock: keep the screenshot-matched dashboard layout stable.
        top = ttk.Frame(self.main, style="GlassRoot.TFrame")
        top.grid(row=0, column=0, sticky="ew")
        top.columnconfigure(0, weight=1)
        ttk.Label(top, text="同步状态", style="GlassTitle.TLabel").grid(row=0, column=0, sticky="w")
        self.status_label = ttk.Label(top, text="正在监听本机剪贴板，收到远端文本后自动写入。", style="GlassMuted.TLabel")
        self.status_label.grid(row=1, column=0, sticky="w", pady=(4, 0))
        actions = ttk.Frame(top, style="GlassRoot.TFrame")
        actions.grid(row=0, column=1, rowspan=2, sticky="e")
        self.primary_sync_button = RoundedButton(actions, text="开始同步", style="Primary.TButton", command=self._toggle_primary_sync)
        self.primary_sync_button.pack(side="left", padx=(0, 8))
        self.pause_button = RoundedButton(actions, text="暂停", command=self._toggle_pause)
        self.pause_button.pack(side="left")

        self._build_overview_metrics()

        self.main.rowconfigure(2, weight=1)
        body = ttk.Frame(self.main, style="GlassRoot.TFrame")
        body.grid(row=2, column=0, sticky="nsew", pady=(12, 0))
        body.columnconfigure(0, weight=1)
        body.columnconfigure(1, weight=1)
        body.rowconfigure(0, minsize=178, weight=0)
        body.rowconfigure(1, weight=1)
        self.peer_list = self._build_overview_device_section(body)
        self._build_manual_connect_panel(body)
        self.log_list = self._build_recent_clipboard_events(body)

    def _build_overview_metrics(self) -> None:
        strip = ttk.Frame(self.main, style="GlassRoot.TFrame")
        strip.grid(row=1, column=0, sticky="ew", pady=(20, 0))
        for col in range(4):
            strip.columnconfigure(col, weight=1)
        self.metric_labels = {}
        self._metric_card(strip, 0, "online", "0", "在线设备")
        self._metric_card(strip, 1, "latency", "0 ms", "最近同步延迟")
        self._metric_card(strip, 2, "syncs", "0", "今日同步次数")
        self._metric_card(strip, 3, "duplicates", "0", "循环拦截")

    def _metric_card(self, parent: ttk.Frame, column: int, key: str, value: str, caption: str) -> None:
        card_shell, card = self._rounded_panel(parent, padding=(18, 14))
        card_shell.configure(height=96)
        card_shell.grid(row=0, column=column, sticky="ew", padx=(0, 10) if column < 3 else 0)
        card.columnconfigure(0, weight=1)
        label = ttk.Label(card, text=value, style="GlassMetric.TLabel")
        label.pack(anchor="center")
        ttk.Label(card, text=caption, style="GlassPanelMuted.TLabel").pack(anchor="center", pady=(8, 0))
        self.metric_labels[key] = label

    def _build_overview_device_section(self, parent: ttk.Frame) -> tk.Listbox:
        return self._list_panel(parent, "已连接设备", 0)

    def _build_manual_connect_panel(self, parent: ttk.Frame) -> None:
        panel_shell, panel = self._rounded_panel(parent, padding=20)
        panel_shell.configure(height=178)
        panel_shell.grid(row=0, column=1, sticky="nsew", padx=(10, 0))
        panel.columnconfigure(0, weight=1)
        ttk.Label(panel, text="手动连接", style="GlassPanelTitle.TLabel").grid(row=0, column=0, sticky="w")
        ttk.Label(panel, text="输入对方 IP", style="GlassPanelMuted.TLabel").grid(row=1, column=0, sticky="w", pady=(14, 8))
        form = tk.Frame(panel, bg=GLASS_COLORS["glass_panel"])
        form.grid(row=2, column=0, sticky="ew")
        form.columnconfigure(0, weight=1)
        RoundedEntry(form, textvariable=self.peer_var, font=(FONT_UI, 15)).grid(row=0, column=0, sticky="ew")
        RoundedButton(form, text="连接", style="Primary.TButton", command=self._add_peer).grid(row=0, column=1, sticky="e", padx=(12, 0))

    def _build_recent_clipboard_events(self, parent: ttk.Frame) -> tk.Listbox:
        return self._list_panel(parent, "连接和剪贴日志", 1, monospace=True, columnspan=2)

    def _build_status_strip(self) -> None:
        strip = ttk.Frame(self.main, style="GlassPanel.TFrame", padding=12)
        strip.grid(row=1, column=0, sticky="ew", pady=(12, 0))
        for col in range(3):
            strip.columnconfigure(col, weight=1)
        self.metric_labels = {}
        self._strip_item(strip, 0, "state", "未启动", "状态")
        self._strip_item(strip, 1, "online", "0", "已连接设备")
        self._strip_item(strip, 2, "latency", "0 ms", "最近延迟")

    def _strip_item(self, parent: ttk.Frame, column: int, key: str, value: str, caption: str) -> None:
        frame = ttk.Frame(parent, style="GlassPanel.TFrame", padding=(10, 6))
        frame.grid(row=0, column=column, sticky="ew", padx=(0, 8) if column < 2 else 0)
        frame.columnconfigure(0, weight=1)
        label = ttk.Label(frame, text=value, style="GlassMetric.TLabel")
        label.pack(anchor="center")
        ttk.Label(frame, text=caption, style="GlassMetricCaption.TLabel").pack(anchor="center")
        self.metric_labels[key] = label

    def _build_primary_sync_action(self) -> None:
        panel = ttk.Frame(self.main, style="GlassPanel.TFrame", padding=14)
        panel.grid(row=2, column=0, sticky="ew", pady=12)
        panel.columnconfigure(0, weight=1)
        ttk.Label(panel, text="剪贴板同步", style="GlassPanelTitle.TLabel").grid(row=0, column=0, sticky="w")
        ttk.Label(
            panel,
            text="添加同一局域网内的设备后，点击主按钮开始同步文本和图片剪贴板。",
            style="GlassPanelMuted.TLabel",
        ).grid(row=1, column=0, sticky="w", pady=(4, 0))
        self.primary_sync_button = ttk.Button(panel, text="开始同步", style="Primary.TButton", command=self._toggle_primary_sync)
        self.primary_sync_button.grid(row=0, column=1, rowspan=2, sticky="e", padx=(14, 0))

    def _build_device_cards(self, parent: ttk.Frame) -> tk.Listbox:
        return self._list_panel(parent, "设备卡片", 0)

    def _build_activity_stream(self, parent: ttk.Frame) -> tk.Listbox:
        return self._list_panel(parent, "同步流", 1, monospace=True)

    def _build_devices_view(self) -> None:
        self._header("设备连接", "用对方 IP 连接局域网内的电脑，添加后回到总览开始同步。")
        self.main.rowconfigure(1, weight=1)
        stack = ttk.Frame(self.main, style="GlassRoot.TFrame")
        stack.grid(row=1, column=0, sticky="nsew", pady=(14, 0))
        stack.columnconfigure(0, weight=1)
        stack.rowconfigure(2, weight=1)
        stack.configure(style="GlassRoot.TFrame")
        # devices-stack: compact vertical workbench for pairing, manual connect, and list.

        self._build_devices_quick_pair_panel(stack)
        self._build_devices_connect_panel(stack)
        self.peer_list = self._build_devices_list_panel(stack)
        self.log_list = None

    def _build_devices_quick_pair_panel(self, parent: ttk.Frame) -> None:
        panel_shell, panel = self._rounded_panel(parent, padding=(18, 14))
        panel_shell.configure(height=108)
        panel_shell.grid(row=0, column=0, sticky="ew")
        panel.columnconfigure(0, weight=1)
        ttk.Label(panel, text="快速配对", style="GlassPanelTitle.TLabel").grid(row=0, column=0, sticky="w")
        ttk.Label(
            panel,
            text="当前版本使用手动 IP 连接；自动发现和扫码配对会在后续版本开启。",
            style="GlassPanelMuted.TLabel",
        ).grid(row=1, column=0, sticky="w", pady=(6, 0))
        actions = tk.Frame(panel, bg=GLASS_COLORS["glass_panel"])
        actions.grid(row=0, column=1, rowspan=2, sticky="e", padx=(16, 0))
        RoundedButton(actions, text="自动发现", command=lambda: self._coming_soon("自动发现"), canvas_bg=GLASS_COLORS["glass_panel"]).pack(side="left", padx=(0, 10))
        RoundedButton(actions, text="扫码配对", command=lambda: self._coming_soon("扫码配对"), canvas_bg=GLASS_COLORS["glass_panel"]).pack(side="left")

    def _build_devices_connect_panel(self, parent: ttk.Frame) -> None:
        panel_shell, panel = self._rounded_panel(parent, padding=(18, 14))
        panel_shell.configure(height=118)
        panel_shell.grid(row=1, column=0, sticky="ew", pady=(12, 0))
        panel.columnconfigure(0, weight=1)
        ttk.Label(panel, text="添加设备", style="GlassPanelTitle.TLabel").grid(row=0, column=0, sticky="w")
        ttk.Label(panel, text="输入对方 IP", style="GlassPanelMuted.TLabel").grid(row=0, column=1, sticky="e")

        form = tk.Frame(panel, bg=GLASS_COLORS["glass_panel"])
        form.grid(row=1, column=0, columnspan=2, sticky="ew", pady=(12, 0))
        form.columnconfigure(0, weight=1)
        RoundedEntry(form, textvariable=self.peer_var, font=(FONT_UI, 14)).grid(row=0, column=0, sticky="ew")
        RoundedButton(form, text="添加设备", style="Primary.TButton", command=self._add_peer, canvas_bg=GLASS_COLORS["glass_panel"]).grid(row=0, column=1, sticky="e", padx=(12, 0))
        RoundedButton(form, text="移除选中", command=self._remove_selected_peer, canvas_bg=GLASS_COLORS["glass_panel"]).grid(row=0, column=2, sticky="e", padx=(10, 0))
        RoundedButton(form, text="清空列表", style="Danger.TButton", command=self._clear_peers, canvas_bg=GLASS_COLORS["glass_panel"]).grid(row=0, column=3, sticky="e", padx=(10, 0))

    def _build_devices_list_panel(self, parent: ttk.Frame) -> tk.Listbox:
        panel_shell, panel = self._rounded_panel(parent, padding=14)
        panel_shell.grid(row=2, column=0, sticky="nsew", pady=(12, 0))
        panel.columnconfigure(0, weight=1)
        ttk.Label(panel, text="设备列表", style="GlassPanelTitle.TLabel").pack(anchor="w")
        widget = tk.Listbox(
            panel,
            borderwidth=0,
            highlightthickness=0,
            selectbackground=GLASS_COLORS["tray_border"],
            selectforeground=GLASS_COLORS["white"],
            background=GLASS_COLORS["entry"],
            foreground=GLASS_COLORS["ink"],
            activestyle="none",
            exportselection=False,
            font=GLASS_FONTS["body"],
        )
        widget.pack(fill="both", expand=True, pady=(10, 0))
        return widget

    def _build_history_view(self) -> None:
        # history-reference-layout-lock: keep the screenshot-matched history layout stable.
        top = self._header("剪贴历史", "查看最近的剪贴同步记录和连接反馈。")
        actions = ttk.Frame(top, style="GlassRoot.TFrame")
        actions.grid(row=0, column=1, rowspan=2, sticky="ne")
        RoundedButton(actions, text="清空历史", command=self._clear_logs, canvas_bg=GLASS_COLORS["tray_bg"]).pack(side="left", padx=(0, 10))
        RoundedButton(actions, text="复制历史", style="Primary.TButton", command=self._copy_logs, canvas_bg=GLASS_COLORS["tray_bg"]).pack(side="left")
        self.main.rowconfigure(1, weight=1)
        self.log_list = self._list_panel(self.main, "同步记录", 1, monospace=True)
        self.peer_list = None

    def _build_settings_view(self) -> None:
        self._header("设置", "调整设备 ID、端口和日常运行选项。")
        panel_shell, panel = self._rounded_panel(self.main, padding=22)
        panel_shell.grid(row=1, column=0, sticky="new", pady=(14, 0))
        panel.columnconfigure(1, weight=1)
        self.device_id_var.set(self.controller.device_id)
        self.port_var.set(str(self.controller.state.listen_port))
        ttk.Label(panel, text="设备 ID", style="GlassPanelText.TLabel").grid(row=0, column=0, sticky="w", pady=(0, 12), padx=(0, 16))
        RoundedEntry(panel, textvariable=self.device_id_var, font=(FONT_UI, 14)).grid(row=0, column=1, sticky="ew", pady=(0, 12))
        ttk.Label(panel, text="监听端口", style="GlassPanelText.TLabel").grid(row=1, column=0, sticky="w", pady=(0, 12), padx=(0, 16))
        RoundedEntry(panel, textvariable=self.port_var, font=(FONT_UI, 14)).grid(row=1, column=1, sticky="ew", pady=(0, 12))
        RoundedButton(panel, text="保存设置", style="Primary.TButton", command=self._save_settings, canvas_bg=GLASS_COLORS["glass_panel"]).grid(row=2, column=1, sticky="w", pady=(4, 0))
        ttk.Label(panel, text="运行中修改设备 ID 或端口需要先“断开全部”。", style="GlassPanelMuted.TLabel").grid(row=3, column=1, sticky="w", pady=(14, 0))
        self.peer_list = None
        self.log_list = None

    def _metric(self, parent: ttk.Frame, column: int, key: str, value: str, caption: str) -> None:
        frame = ttk.Frame(parent, style="GlassPanel.TFrame", padding=12)
        frame.grid(row=0, column=column, sticky="ew", padx=4)
        frame.columnconfigure(0, weight=1)
        label = ttk.Label(frame, text=value, style="GlassMetric.TLabel")
        label.pack(anchor="center")
        ttk.Label(frame, text=caption, style="GlassMetricCaption.TLabel").pack(anchor="center")
        self.metric_labels[key] = label

    def _list_panel(
        self,
        parent: ttk.Frame,
        title: str,
        column_or_row: int,
        *,
        monospace: bool = False,
        columnspan: int = 1,
    ) -> tk.Listbox:
        panel_shell, panel = self._rounded_panel(parent, padding=12)
        if parent is self.main:
            panel_shell.grid(row=column_or_row, column=0, sticky="nsew", pady=(14, 0))
            self.main.rowconfigure(column_or_row, weight=1)
        elif columnspan > 1:
            panel_shell.grid(row=1, column=0, columnspan=columnspan, sticky="nsew", pady=(12, 0))
        else:
            panel_shell.grid(row=0, column=column_or_row, sticky="nsew", padx=(0, 7) if column_or_row == 0 else (7, 0))
        ttk.Label(panel, text=title, style="GlassPanelTitle.TLabel").pack(anchor="w")
        font = GLASS_FONTS["mono"] if monospace else GLASS_FONTS["body"]
        widget = tk.Listbox(
            panel,
            borderwidth=0,
            highlightthickness=0,
            highlightbackground=GLASS_COLORS["border"],
            highlightcolor=GLASS_COLORS["blue"],
            selectbackground=GLASS_COLORS["tray_border"],
            selectforeground=GLASS_COLORS["white"],
            background=GLASS_COLORS["entry"],
            foreground=GLASS_COLORS["ink"],
            activestyle="none",
            exportselection=False,
            font=font,
        )
        widget.pack(fill="both", expand=True, pady=(10, 0))
        return widget

    def _start(self) -> None:
        if not self.controller.apply_settings(port_text=self.port_var.get(), device_id_text=self.device_id_var.get()):
            self._refresh()
            return
        peer = self.peer_var.get().strip()
        if peer and not self.controller.add_peer(peer):
            self._refresh()
            return
        self.controller.start(port=self.controller.state.listen_port, peers=[])
        self._refresh()

    def _toggle_primary_sync(self) -> None:
        if self.controller.state.running:
            self._stop()
        else:
            self._start()

    def _toggle_pause(self) -> None:
        if self.controller.state.paused:
            self.controller.resume()
        else:
            self.controller.pause()
        self._refresh()

    def _stop(self) -> None:
        self.controller.stop()
        self._refresh()

    def _add_peer(self) -> None:
        peer = self.peer_var.get().strip()
        if peer:
            self.controller.add_peer(peer)
        self._refresh()

    def _remove_selected_peer(self) -> None:
        if not self.peer_list:
            return
        selection = self.peer_list.curselection()
        if not selection:
            self.controller.state.add_log("请先选择要移除的设备", level="error")
            self._refresh()
            return
        index = selection[0]
        if index < len(self.controller.state.peers):
            self.controller.remove_peer(self.controller.state.peers[index].address)
        self._refresh()

    def _clear_peers(self) -> None:
        self.controller.clear_peers()
        self._refresh()

    def _clear_logs(self) -> None:
        self.controller.clear_logs()
        self._refresh()

    def _coming_soon(self, feature: str) -> None:
        self.controller.state.add_log(f"{feature}会在后续版本开启；当前请手动输入设备 IP。")
        self._refresh()

    def _copy_logs(self) -> None:
        text = "\n".join(f"[{entry.level}] {entry.message}" for entry in self.controller.state.logs)
        self.root.clipboard_clear()
        self.root.clipboard_append(text)
        self.controller.state.add_log("剪贴历史已复制到剪贴板")
        self._refresh()

    def _save_settings(self) -> None:
        self.controller.apply_settings(port_text=self.port_var.get(), device_id_text=self.device_id_var.get())
        self._refresh()

    def add_runtime_log(self, message: str, level: str = "info") -> None:
        def commit() -> None:
            if self._closing:
                return
            self._apply_runtime_state_hint(message)
            self.controller.state.add_log(message, level=level)
            self._refresh()

        try:
            self.root.after(0, commit)
        except tk.TclError:
            self.controller.state.add_log(message, level=level)

    def _apply_runtime_state_hint(self, message: str) -> None:
        if message.startswith("设备已识别："):
            rest = message.removeprefix("设备已识别：")
            marker = "（设备 ID："
            if marker in rest:
                address, raw_device_id = rest.split(marker, 1)
                device_id = raw_device_id.rstrip("）").strip()
                address = address.strip()
                if address and device_id:
                    self.controller.state.upsert_peer(PeerStatus(name=device_id, address=address, state="online"))
                    return
        if message.startswith("已连接设备："):
            address = message.split("：", 1)[1].strip()
            self.controller.state.upsert_peer(PeerStatus(name=_peer_log_name(address), address=address, state="online"))
            return
        if message.startswith("远端设备已连接："):
            address = message.split("：", 1)[1].strip()
            self.controller.state.upsert_peer(PeerStatus(name=_peer_log_name(address), address=address, state="online"))
            return
        for prefix in ("设备暂不可用：", "设备连接已断开：", "远端设备已断开："):
            if message.startswith(prefix):
                address = message.removeprefix(prefix).split("（", 1)[0].strip()
                self.controller.state.upsert_peer(PeerStatus(name=_peer_log_name(address), address=address, state="reconnecting"))
                return
        if "本机复制" in message or "已写入剪贴板" in message:
            self.controller.state.sync_count += 1

    def _handle_main_window_unmap(self, event: tk.Event) -> None:
        if event.widget is not self.root or self._closing:
            return
        self.root.after(120, self._enter_compact_mode)

    def _enter_compact_mode(self) -> None:
        if self._closing:
            return
        try:
            if self.root.state() != "iconic":
                return
        except tk.TclError:
            return
        self._compact_mode_active = True
        self.root.withdraw()
        self._open_compact()

    def _open_compact(self) -> None:
        if self.compact_window and self.compact_window.winfo_exists():
            self.compact_window.deiconify()
            self.compact_window.lift()
            return
        self.compact_window = tk.Toplevel(self.root)
        self.compact_window.title("托盘小浮窗")
        self.compact_window.geometry("320x254")
        self.compact_window.resizable(False, False)
        self.compact_window.configure(bg=GLASS_COLORS["tray_bg"])
        apply_window_backdrop(self.compact_window)
        apply_window_alpha(self.compact_window)
        self.compact_window.protocol("WM_DELETE_WINDOW", self._close_compact_window)
        self._build_compact_window_shell(self.compact_window)
        self._refresh_compact()

    def _build_compact_window_shell(self, window: tk.Toplevel) -> None:
        colors = GLASS_COLORS
        compact = tk.Frame(window, bg=colors["tray_bg"], padx=18, pady=18)
        compact.pack(fill="both", expand=True)
        header = tk.Frame(compact, bg=colors["tray_bg"])
        header.pack(fill="x")
        tk.Label(header, text="剪贴板同步", bg=colors["tray_bg"], fg=colors["white"], font=(FONT_UI, 15, "bold")).pack(side="left")
        status_group = tk.Frame(header, bg=colors["tray_bg"])
        status_group.pack(side="right")
        self.compact_status_dot = tk.Canvas(status_group, width=10, height=10, bg=colors["tray_bg"], highlightthickness=0)
        self.compact_status_dot.pack(side="left", padx=(0, 6))
        self.compact_status_dot.create_oval(1, 1, 9, 9, fill=colors["green"], outline=colors["green"])
        self.compact_status_text = tk.Label(status_group, text="在线", bg=colors["tray_bg"], fg=colors["green"], font=(FONT_UI, 10, "bold"))
        self.compact_status_text.pack(side="left")

        separator = tk.Frame(compact, bg=colors["tray_border"], height=1)
        separator.pack(fill="x", pady=(16, 10))
        self.compact_peer_value = self._compact_metric_row(compact, "已连接设备")
        self.compact_latency_value = self._compact_metric_row(compact, "最近延迟")
        self.compact_port_value = self._compact_metric_row(compact, "监听端口")

        buttons = tk.Frame(compact, bg=colors["tray_bg"])
        buttons.pack(fill="x", pady=(14, 0))
        self.compact_pause_button = tk.Button(
            buttons,
            text="暂停",
            command=self._toggle_pause,
            bg=colors["tray_panel"],
            fg=colors["white"],
            activebackground=colors["tray_border"],
            activeforeground=colors["white"],
            relief="solid",
            borderwidth=1,
            font=(FONT_UI, 11, "bold"),
        )
        self.compact_pause_button.pack(side="left", fill="x", expand=True, padx=(0, 8), ipady=8)
        tk.Button(
            buttons,
            text="打开面板",
            command=self._open_main_from_compact,
            bg=colors["tray_panel"],
            fg=colors["white"],
            activebackground=colors["tray_border"],
            activeforeground=colors["white"],
            relief="solid",
            borderwidth=1,
            font=(FONT_UI, 11, "bold"),
        ).pack(side="left", fill="x", expand=True, padx=(8, 0), ipady=8)

    def _compact_metric_row(self, parent: tk.Frame, label: str) -> tk.Label:
        colors = GLASS_COLORS
        row = tk.Frame(parent, bg=colors["tray_bg"])
        row.pack(fill="x")
        tk.Label(row, text=label, bg=colors["tray_bg"], fg=colors["tray_muted"], font=(FONT_UI, 10)).pack(side="left", pady=8)
        value = tk.Label(row, text="", bg=colors["tray_bg"], fg=colors["white"], font=(FONT_UI, 10, "bold"))
        value.pack(side="right", pady=8)
        tk.Frame(parent, bg=colors["tray_border"], height=1).pack(fill="x")
        return value

    def _open_main_from_compact(self) -> None:
        self._compact_mode_active = False
        self.root.deiconify()
        self.root.state("normal")
        self.root.lift()
        self.root.focus_force()
        if self.compact_window and self.compact_window.winfo_exists():
            self.compact_window.withdraw()

    def _close_compact_window(self) -> None:
        if self.compact_window and self.compact_window.winfo_exists():
            self.compact_window.destroy()
        self.compact_window = None
        if self._compact_mode_active and not self._closing:
            self._compact_mode_active = False
            self.root.deiconify()
            self.root.state("normal")
            self.root.lift()

    def _refresh(self) -> None:
        state = self.controller.state
        if self.status_label is not None and self.status_label.winfo_exists():
            self.status_label.configure(text=f"{state.status_text} · 设备 ID：{self.controller.device_id}")
        self._refresh_port_status_card(state)
        if self.primary_sync_button is not None and self.primary_sync_button.winfo_exists():
            self.primary_sync_button.configure(
                text="停止同步" if state.running else "开始同步",
                style="Danger.TButton" if state.running else "Primary.TButton",
            )
        if self.pause_button is not None and self.pause_button.winfo_exists():
            self.pause_button.configure(text="恢复" if state.paused else "暂停")
        metric_values = {
            "state": status_display(state),
            "online": str(state.connected_count),
            "latency": f"{state.latest_latency_ms} ms",
            "syncs": str(state.sync_count),
            "duplicates": str(state.duplicate_blocks),
        }
        for key, text in metric_values.items():
            label = self.metric_labels.get(key)
            if label is not None and label.winfo_exists():
                label.configure(text=text)
        if getattr(self, "peer_list", None):
            self._fill_peers(state)
        if getattr(self, "log_list", None):
            self._fill_logs(state)
        self._refresh_compact()

    def _refresh_port_status_card(self, state: SyncUiState) -> None:
        if not hasattr(self, "port_card_canvas"):
            return
        if state.paused:
            status_text = "已暂停"
            status_color = GLASS_COLORS["yellow"]
        elif state.running:
            status_text = "运行中"
            status_color = GLASS_COLORS["green"]
        else:
            status_text = "未启动"
            status_color = GLASS_COLORS["red"]

        hint = f"已连接 {state.connected_count} 台设备" if state.connected_count else "等待局域网设备连接"
        self.port_card_canvas.itemconfigure(self.port_card_items["port"], text=str(state.listen_port))
        self.port_card_canvas.itemconfigure(self.port_card_items["hint"], text=hint)
        self.port_card_canvas.itemconfigure(self.port_card_items["status"], text=status_text, fill=status_color)

    def _fill_peers(self, state: SyncUiState) -> None:
        if not self.peer_list:
            return
        self.peer_list.delete(0, tk.END)
        if not state.peers:
            self.peer_list.insert(tk.END, "暂无连接设备")
            self.peer_list.insert(tk.END, "输入对方 IP 后点击“添加设备”")
            return
        for peer in state.peers:
            self.peer_list.insert(tk.END, f"{peer.name}    {peer.state}    最近同步：暂无    {_display_peer_address(peer.address)}")

    def _fill_logs(self, state: SyncUiState) -> None:
        if not self.log_list:
            return
        self.log_list.delete(0, tk.END)
        if not state.logs:
            self.log_list.insert(tk.END, "开始同步后会显示连接和剪贴板日志")
            return
        for entry in state.logs[:40]:
            timestamp = datetime.fromtimestamp(entry.timestamp).strftime("%H:%M:%S")
            self.log_list.insert(tk.END, f"{timestamp}  [{entry.level}] {entry.message}")

    def _refresh_compact(self) -> None:
        if not self.compact_window or not self.compact_window.winfo_exists():
            return
        state = self.controller.state
        text = "暂停" if state.paused else ("在线" if state.running else "离线")
        color = GLASS_COLORS["yellow"] if state.paused else (GLASS_COLORS["green"] if state.running else GLASS_COLORS["red"])
        if hasattr(self, "compact_status_text"):
            self.compact_status_text.configure(text=text, fg=color)
        if hasattr(self, "compact_status_dot"):
            self.compact_status_dot.delete("all")
            self.compact_status_dot.create_oval(1, 1, 9, 9, fill=color, outline=color)
        self.compact_peer_value.configure(text=str(state.connected_count))
        self.compact_latency_value.configure(text=f"{state.latest_latency_ms} ms")
        self.compact_port_value.configure(text=str(state.listen_port))
        self.compact_pause_button.configure(text="恢复" if state.paused else "暂停")


def run_ui(args: argparse.Namespace) -> None:
    root = tk.Tk()
    runtime = BackgroundSyncRuntime(device_id=args.device_id)
    controller = SyncUiController(device_id=args.device_id, runtime=runtime)
    ui = SyncDesktopUi(root, controller, args.port, args.peer)
    runtime.log = ui.add_runtime_log
    root.protocol("WM_DELETE_WINDOW", lambda: _close(root, controller, ui))
    root.mainloop()


def _close(root: tk.Tk, controller: SyncUiController, ui: SyncDesktopUi | None = None) -> None:
    if ui is not None:
        ui._closing = True
    controller.stop()
    root.destroy()


def _display_peer_address(address: str) -> str:
    return address.removeprefix("ws://").rstrip("/")


def _peer_log_name(address: str) -> str:
    return _display_peer_address(address)


def status_display(state: SyncUiState) -> str:
    if state.status_key == "disconnected":
        return "未连接设备"
    return state.status_text
