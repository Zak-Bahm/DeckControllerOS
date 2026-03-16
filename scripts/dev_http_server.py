#!/usr/bin/env python3
"""Serve staged payload files and accept Deck log uploads."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import os
import re


def _sanitize(s: str) -> str:
    out = re.sub(r"[^A-Za-z0-9._-]+", "_", s.strip())
    return out[:80] or "unknown"


class DevRequestHandler(SimpleHTTPRequestHandler):
    logs_dir: Path
    log_endpoint: str

    def do_POST(self) -> None:  # noqa: N802 (base class API)
        if self.path != self.log_endpoint:
            self.send_error(404, "Unknown endpoint")
            return

        content_length = self.headers.get("Content-Length")
        if content_length is None:
            self.send_error(411, "Content-Length required")
            return

        try:
            n_bytes = int(content_length)
        except ValueError:
            self.send_error(400, "Invalid Content-Length")
            return

        payload = self.rfile.read(n_bytes)
        now = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        remote_ip = self.client_address[0] if self.client_address else "unknown"

        # Extract metadata from the payload body (authoritative source).
        # The payload starts with key=value lines followed by a blank line.
        body_meta: dict[str, str] = {}
        try:
            text = payload.decode("utf-8", errors="replace")
            for line in text.splitlines():
                if not line:
                    break
                if "=" in line:
                    k, v = line.split("=", 1)
                    body_meta[k.strip()] = v.strip()
        except Exception:
            pass

        # Use body metadata, fall back to HTTP headers, then defaults.
        host = _sanitize(
            body_meta.get("deck_host")
            or self.headers.get("X-Deck-Host")
            or "deck"
        )
        status = _sanitize(
            body_meta.get("command_exit_code")
            or self.headers.get("X-Command-Exit-Code")
            or "unknown"
        )
        command = (
            body_meta.get("command")
            or self.headers.get("X-Command")
            or ""
        )

        filename = f"{now}_{host}_exit{status}.log"
        out_path = self.logs_dir / filename
        i = 1
        while out_path.exists():
            out_path = self.logs_dir / f"{now}_{host}_exit{status}_{i}.log"
            i += 1

        meta = [
            f"received_utc={now}",
            f"remote_ip={remote_ip}",
        ]
        out_path.write_bytes("\n".join(meta).encode("utf-8") + b"\n" + payload)

        self.send_response(200)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.end_headers()
        self.wfile.write(f"stored {out_path.name}\n".encode("utf-8"))

    def log_message(self, fmt: str, *args: object) -> None:
        timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        message = fmt % args
        print(f"[{timestamp}] {self.client_address[0]} {message}")


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Serve dev payload and collect logs from controlleros-dev-run."
    )
    parser.add_argument("--dir", required=True, help="Directory to serve static payload from")
    parser.add_argument(
        "--logs-dir", required=True, help="Directory where POSTed logs are stored"
    )
    parser.add_argument("--bind", default="0.0.0.0", help="Bind address")
    parser.add_argument("--port", type=int, default=8000, help="TCP port")
    parser.add_argument("--log-endpoint", default="/logs", help="POST endpoint path")
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    serve_dir = Path(args.dir).resolve()
    logs_dir = Path(args.logs_dir).resolve()

    if not serve_dir.is_dir():
        raise SystemExit(f"error: serve directory does not exist: {serve_dir}")

    logs_dir.mkdir(parents=True, exist_ok=True)

    endpoint = args.log_endpoint
    if not endpoint.startswith("/"):
        endpoint = "/" + endpoint

    handler_cls = type("Handler", (DevRequestHandler,), {})
    handler_cls.logs_dir = logs_dir
    handler_cls.log_endpoint = endpoint

    os.chdir(serve_dir)
    server = ThreadingHTTPServer((args.bind, args.port), handler_cls)
    print(f"Serving payload from {serve_dir}")
    print(f"Listening on http://{args.bind}:{args.port}")
    print(f"Log endpoint: {endpoint}")
    print(f"Logs dir: {logs_dir}")
    server.serve_forever()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
