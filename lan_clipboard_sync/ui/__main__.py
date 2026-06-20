from __future__ import annotations

import argparse


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="lan-clipboard-sync-ui",
        description="LAN Clipboard Sync 桌面控制台。",
    )
    parser.add_argument("--device-id", default="ui-device", help="当前设备 ID")
    parser.add_argument("--port", type=int, default=8765, help="本机监听端口")
    parser.add_argument(
        "--peer",
        action="append",
        default=[],
        help="设备 IP、host:port 或 ws:// URL；可重复传入",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    from .tk_app import run_ui

    run_ui(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
