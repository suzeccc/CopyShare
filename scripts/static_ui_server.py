from __future__ import annotations

import argparse
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import time


class UiHandler(BaseHTTPRequestHandler):
    root: Path

    def do_GET(self) -> None:
        if self.path not in {"/", "/index.html"}:
            self.send_error(HTTPStatus.NOT_FOUND)
            return

        html = (self.root / "index.html").read_bytes()
        self.send_response(HTTPStatus.OK)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(html)))
        self.end_headers()
        self.wfile.write(html)

    def do_POST(self) -> None:
        if self.path != "/event":
            self.send_error(HTTPStatus.NOT_FOUND)
            return

        length = int(self.headers.get("content-length", "0"))
        body = self.rfile.read(length).decode("utf-8", errors="replace")
        state_dir = self.root / "state"
        state_dir.mkdir(parents=True, exist_ok=True)
        with (state_dir / "events").open("a", encoding="utf-8") as stream:
            stream.write(body + "\n")

        payload = b'{"ok":true}'
        self.send_response(HTTPStatus.OK)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, format: str, *args: object) -> None:
        log_path = self.root / "state" / "server.log"
        with log_path.open("a", encoding="utf-8") as stream:
            stream.write("[%s] %s\n" % (time.strftime("%Y-%m-%dT%H:%M:%S"), format % args))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=61234)
    args = parser.parse_args()

    root = Path(args.root).resolve()
    state_dir = root / "state"
    state_dir.mkdir(parents=True, exist_ok=True)

    handler = type("BoundUiHandler", (UiHandler,), {"root": root})
    server = ThreadingHTTPServer((args.host, args.port), handler)
    info = {
        "type": "server-started",
        "host": args.host,
        "port": args.port,
        "url": f"http://localhost:{args.port}",
        "screen_file": str(root / "index.html"),
        "state_dir": str(state_dir),
    }
    (state_dir / "server-info").write_text(json.dumps(info, ensure_ascii=False) + "\n", encoding="utf-8")
    (state_dir / "server.log").write_text(json.dumps(info, ensure_ascii=False) + "\n", encoding="utf-8")
    server.serve_forever()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
