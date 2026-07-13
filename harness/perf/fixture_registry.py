#!/usr/bin/env python3
"""Serve checked-in registry fixtures with runtime bottle URLs."""

from __future__ import annotations

import argparse
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


class FixtureHandler(SimpleHTTPRequestHandler):
    root: Path
    base_url: str

    def translate_path(self, path: str) -> str:
        relative = path.split("?", 1)[0].lstrip("/")
        if relative == "formula.json" or (
            relative.startswith(("formula/", "bottles/"))
            and (relative.endswith(".json") or relative.startswith("bottles/"))
        ):
            return str(self.root / relative)
        return str(self.root / ".missing")

    def send_head(self):
        path = Path(self.translate_path(self.path))
        if not path.is_file():
            self.send_error(404, "Not found")
            return None
        if path.suffix != ".json":
            return super().send_head()
        data = path.read_bytes().replace(b"http://127.0.0.1:0", self.base_url.encode())
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        return data

    def do_GET(self):
        result = self.send_head()
        if isinstance(result, bytes):
            self.wfile.write(result)
        elif result:
            try:
                self.copyfile(result, self.wfile)
            finally:
                result.close()

    def do_HEAD(self):
        result = self.send_head()
        if result and not isinstance(result, bytes):
            result.close()


def main() -> None:
    parser = argparse.ArgumentParser(description="Serve Coldbrew registry fixtures.")
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--port-file", type=Path, required=True)
    args = parser.parse_args()

    server = ThreadingHTTPServer(("127.0.0.1", 0), FixtureHandler)
    FixtureHandler.root = args.root.resolve()
    FixtureHandler.base_url = f"http://127.0.0.1:{server.server_port}"
    args.port_file.write_text(str(server.server_port))
    server.serve_forever()


if __name__ == "__main__":
    main()
