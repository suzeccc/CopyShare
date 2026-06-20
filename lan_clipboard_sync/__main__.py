from __future__ import annotations

import argparse
import asyncio
import logging
import socket
import uuid

from .app import ClipboardSyncApp


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="lan-clipboard-sync",
        description="通过 WebSocket 在局域网内同步文本和图片剪贴板。",
    )
    parser.add_argument("--host", default="0.0.0.0", help="服务监听地址")
    parser.add_argument("--port", type=int, default=8765, help="服务 WebSocket 端口")
    parser.add_argument(
        "--peer",
        action="append",
        default=[],
        help="设备 IP、host:port 或 ws:// URL；可重复传入",
    )
    parser.add_argument(
        "--poll-interval",
        type=float,
        default=0.1,
        help="剪贴板轮询间隔，单位秒",
    )
    parser.add_argument(
        "--reconnect-delay",
        type=float,
        default=2.0,
        help="对端设备断开后的重连等待秒数",
    )
    parser.add_argument(
        "--device-id",
        default=f"{socket.gethostname()}-{uuid.uuid4().hex[:8]}",
        help="同步消息里的当前设备 ID",
    )
    parser.add_argument("--verbose", action="store_true", help="启用调试日志")
    return parser


async def _run(args: argparse.Namespace) -> None:
    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(asctime)s %(levelname)s %(message)s",
    )
    app = ClipboardSyncApp(
        device_id=args.device_id,
        host=args.host,
        port=args.port,
        peers=args.peer,
        poll_interval=args.poll_interval,
        reconnect_delay=args.reconnect_delay,
    )
    await app.run()


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        asyncio.run(_run(args))
    except KeyboardInterrupt:
        return 0
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
