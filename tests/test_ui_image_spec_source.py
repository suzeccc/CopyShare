import unittest
from pathlib import Path


class UiImageSpecSourceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.source = Path("lan_clipboard_sync/ui/tk_app.py").read_text(encoding="utf-8")
        self.preview = Path(".superpowers/brainstorm/ui-stable/index.html").read_text(encoding="utf-8")

    def test_overview_matches_reference_dashboard_sections(self) -> None:
        for marker in (
            "_build_overview_metrics",
            "_build_overview_device_section",
            "_build_manual_connect_panel",
            "_build_recent_clipboard_events",
        ):
            self.assertIn(marker, self.source)

        self.assertNotIn("self._build_tray_bridge_panel(body)", self.source)
        self.assertNotIn("self.stop_button", self.source)
        self.assertNotIn('>断开全部</button>', self.preview)
        self.assertIn('id="stopSync" type="button">停止同步</button>', self.preview)
        self.assertIn('self._list_panel(parent, "连接和剪贴日志", 1, monospace=True, columnspan=2)', self.source)
        self.assertIn("body.rowconfigure(0, minsize=178, weight=0)", self.source)
        self.assertIn("highlightthickness=0", self.source)

        for text in (
            "同步状态",
            "在线设备",
            "最近同步延迟",
            "今日同步次数",
            "循环拦截",
            "已连接设备",
            "手动连接",
            "输入对方 IP",
            "连接",
            "连接和剪贴日志",
            "暂无连接设备",
            "开始同步后会显示连接和剪贴板日志",
        ):
            self.assertIn(text, self.source)

        for old_copy in (
            "对方 IP 或 ws:// 地址",
            "连接 peer",
            "默认会补全为 ws://192.168.1.20:8765/",
        ):
            self.assertNotIn(old_copy, self.source)

    def test_tray_floating_window_matches_reference_controls(self) -> None:
        for marker in (
            "_build_compact_window_shell",
            "_compact_metric_row",
            "_open_main_from_compact",
            "compact_pause_button",
            "compact_status_text",
        ):
            self.assertIn(marker, self.source)

        for text in ("托盘小浮窗", "剪贴板同步", "已连接设备", "最近延迟", "监听端口", "暂停", "打开面板"):
            self.assertIn(text, self.source)

    def test_preview_contains_reference_dashboard_and_tray(self) -> None:
        for marker in (
            "function overviewMetricsHtml",
            "function manualConnectHtml",
            "function recentEventsHtml",
            "function compactFloatingHtml",
        ):
            self.assertIn(marker, self.preview)

        self.assertNotIn("function trayBridgeHtml", self.preview)
        self.assertNotIn("${trayBridgeHtml()}", self.preview)
        self.assertIn("recent-events-card", self.preview)
        self.assertIn("overview-card", self.preview)
        self.assertIn("grid-column: 1 / -1", self.preview)

        for text in ("同步状态", "输入对方 IP", "连接", "托盘小浮窗", "打开面板", "已连接设备", "连接和剪贴日志"):
            self.assertIn(text, self.preview)

        for old_copy in (
            "对方 IP 或 ws:// 地址",
            "连接 peer",
            "默认会补全为 ws://192.168.1.20:8765/",
        ):
            self.assertNotIn(old_copy, self.preview)

    def test_sidebar_port_card_matches_latest_reference(self) -> None:
        for marker in (
            "_build_port_status_card",
            "_draw_rounded_rect",
            "port_card_canvas",
            "_refresh_port_status_card",
        ):
            self.assertIn(marker, self.source)

        for old_marker in (
            "status_dot_bg",
            "port_status_dot_outer",
            "port_status_dot_inner",
        ):
            self.assertNotIn(old_marker, self.source)

        self.assertNotIn("self.port_label = ttk.Label(sidebar", self.source)
        self.assertIn('text="本机端口"', self.source)
        self.assertIn('text="等待局域网设备连接"', self.source)
        self.assertIn('text="未启动"', self.source)
        self.assertIn('font=GLASS_FONTS["port_value"]', self.source)
        self.assertIn('font=GLASS_FONTS["port_status"]', self.source)

        self.assertIn(".side-card .status", self.preview)
        self.assertNotIn(".side-card .status:before", self.preview)
        self.assertNotIn("box-shadow: 0 0 0 8px rgba(34, 197, 94, .14)", self.preview)
        self.assertIn("#sidePort", self.preview)
        self.assertIn("font-size: 29px", self.preview)

    def test_device_connection_page_uses_compact_workbench_layout(self) -> None:
        for marker in (
            "_build_devices_quick_pair_panel",
            "_build_devices_connect_panel",
            "_build_devices_list_panel",
            "devices-stack",
            "device-form-row",
            "device-list-card",
        ):
            self.assertIn(marker, self.source + self.preview)

        for text in (
            "快速配对",
            "当前版本使用手动 IP 连接",
            "添加设备",
            "输入对方 IP",
            "移除选中",
            "清空列表",
            "设备列表",
        ):
            self.assertIn(text, self.source)

        for old_layout in (
            'assist = ttk.Frame(self.main, style="GlassPanel.TFrame"',
            'form = ttk.Frame(self.main, style="GlassPanel.TFrame"',
            'self.peer_list = self._list_panel(self.main, "设备列表", 3)',
        ):
            self.assertNotIn(old_layout, self.source)

        self.assertIn('RoundedEntry(form, textvariable=self.peer_var, font=(FONT_UI, 14))', self.source)
        self.assertIn('RoundedButton(form, text="添加设备"', self.source)
        self.assertIn('RoundedButton(form, text="移除选中"', self.source)
        self.assertIn('RoundedButton(form, text="清空列表"', self.source)

    def test_settings_page_allows_editing_device_id(self) -> None:
        for fragment in (
            'self.device_id_var = tk.StringVar(value=self.controller.device_id)',
            'RoundedEntry(panel, textvariable=self.device_id_var',
            'self.controller.apply_settings(port_text=self.port_var.get(), device_id_text=self.device_id_var.get())',
            'RoundedButton(panel, text="保存设置", style="Primary.TButton", command=self._save_settings',
            '调整设备 ID、端口和日常运行选项。',
        ):
            self.assertIn(fragment, self.source)

        self.assertNotIn('ttk.Label(panel, text=self.controller.device_id', self.source)

    def test_view_switching_does_not_refresh_destroyed_metric_widgets(self) -> None:
        for fragment in (
            "if self.controller.state.active_view == view and self.main.winfo_children():",
            "self.metric_labels = {}",
            'label.winfo_exists()',
        ):
            self.assertIn(fragment, self.source)

    def test_view_switching_reuses_cached_view_widgets(self) -> None:
        for fragment in (
            "self._view_widgets",
            "self._view_refs",
            "def _restore_view_refs",
            "grid_remove()",
            "self._store_view_refs(view)",
        ):
            self.assertIn(fragment, self.source)

        self.assertNotIn("for child in self.main.winfo_children():\n            child.destroy()", self.source)

    def test_overview_and_history_layouts_stay_reference_locked(self) -> None:
        for marker in (
            "overview-reference-layout-lock",
            "history-reference-layout-lock",
        ):
            self.assertIn(marker, self.source)

        for overview_fragment in (
            'self._build_overview_metrics()',
            'body.grid(row=2, column=0, sticky="nsew", pady=(12, 0))',
            'body.rowconfigure(0, minsize=178, weight=0)',
            'self.peer_list = self._build_overview_device_section(body)',
            'self._build_manual_connect_panel(body)',
            'self.log_list = self._build_recent_clipboard_events(body)',
            'self.primary_sync_button = RoundedButton(actions, text="开始同步"',
            'self.pause_button = RoundedButton(actions, text="暂停"',
            'panel_shell.configure(height=178)',
            'RoundedEntry(form, textvariable=self.peer_var, font=(FONT_UI, 15))',
            'RoundedButton(form, text="连接"',
            'self._list_panel(parent, "连接和剪贴日志", 1, monospace=True, columnspan=2)',
        ):
            self.assertIn(overview_fragment, self.source)

        for history_fragment in (
            'top = self._header("剪贴历史", "查看最近的剪贴同步记录和连接反馈。")',
            'actions = ttk.Frame(top, style="GlassRoot.TFrame")',
            'actions.grid(row=0, column=1, rowspan=2, sticky="ne")',
            'RoundedButton(actions, text="清空历史", command=self._clear_logs',
            'RoundedButton(actions, text="复制历史", style="Primary.TButton", command=self._copy_logs',
            'self.main.rowconfigure(1, weight=1)',
            'self.log_list = self._list_panel(self.main, "同步记录", 1, monospace=True)',
            'panel_shell.grid(row=column_or_row, column=0, sticky="nsew", pady=(14, 0))',
        ):
            self.assertIn(history_fragment, self.source)

        for old_history_fragment in (
            'actions.grid(row=1, column=0, sticky="ew", pady=12)',
            'ttk.Button(actions, text="清空历史", command=self._clear_logs).pack(side="left")',
            'self.log_list = self._list_panel(self.main, "同步记录", 2, monospace=True)',
        ):
            self.assertNotIn(old_history_fragment, self.source)


if __name__ == "__main__":
    unittest.main()
