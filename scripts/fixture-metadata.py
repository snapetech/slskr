#!/usr/bin/env python3
"""Deterministic MusicBrainz/AcoustID HTTP fixture for live integration tests."""

from __future__ import annotations

import argparse
import json
import threading
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


RELEASE = {
    "id": "release-1",
    "title": "Fixture Release",
    "date": "2024-01-02",
    "country": "CA",
    "artist-credit": [{"name": "Fixture Artist", "artist": {"id": "artist-1", "name": "Fixture Artist"}}],
    "media": [{
        "format": "Digital Media",
        "tracks": [
            {"position": 1, "title": "Fixture Track", "length": 180000, "recording": {"id": "recording-1", "title": "Fixture Track", "artist-credit": [{"name": "Fixture Artist", "artist": {"id": "artist-1"}}]}},
        ],
    }],
}

RECORDING = {
    "id": "recording-1",
    "title": "Fixture Track",
    "length": 180000,
    "isrcs": ["CA-FIX-24-00001"],
    "artist-credit": [{"name": "Fixture Artist", "artist": {"id": "artist-1"}}],
}

ARTIST = {"id": "artist-1", "name": "Fixture Artist", "sort-name": "Artist, Fixture"}
RELEASE_GROUP = {
    "id": "group-1",
    "title": "Fixture Release",
    "primary-type": "Album",
    "first-release-date": "2024-01-02",
}


class State:
    def __init__(self) -> None:
        self.lock = threading.Lock()
        self.requests: list[dict[str, object]] = []

    def record(self, handler: BaseHTTPRequestHandler) -> None:
        parsed = urllib.parse.urlsplit(handler.path)
        with self.lock:
            self.requests.append({
                "path": parsed.path,
                "query": urllib.parse.parse_qs(parsed.query, keep_blank_values=True),
                "userAgent": handler.headers.get("User-Agent", ""),
            })

    def reset(self) -> None:
        with self.lock:
            self.requests.clear()

    def snapshot(self) -> dict[str, object]:
        with self.lock:
            return {"requests": list(self.requests)}


def response_for(path: str, query: dict[str, list[str]]) -> object:
    if path.endswith("/release/release-1"):
        return RELEASE
    if path.endswith("/recording/recording-1"):
        return RECORDING
    if "/artist/" in path:
        return {**ARTIST, "id": path.rsplit("/", 1)[-1]}
    if path.endswith("/release-group"):
        return {"release-groups": [RELEASE_GROUP], "count": 1}
    if path.endswith("/release") and query.get("release-group") == ["group-1"]:
        return {"releases": [RELEASE], "count": 1}
    if path.endswith("/recording"):
        return {"recordings": [RECORDING], "count": 1}
    if path.endswith("/ws/2") or path == "/":
        return {"fixture": True}
    return {"error": "not found"}


def handler_type(state: State) -> type[BaseHTTPRequestHandler]:
    class Handler(BaseHTTPRequestHandler):
        server_version = "MetadataFixture/1"

        def log_message(self, *_args: object) -> None:
            return

        def send_json(self, status: int, value: object) -> None:
            body = json.dumps(value, separators=(",", ":")).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self) -> None:  # noqa: N802
            parsed = urllib.parse.urlsplit(self.path)
            if parsed.path == "/__status":
                self.send_json(200, state.snapshot())
                return
            state.record(self)
            value = response_for(parsed.path, urllib.parse.parse_qs(parsed.query, keep_blank_values=True))
            self.send_json(404 if isinstance(value, dict) and value.get("error") else 200, value)

        def do_POST(self) -> None:  # noqa: N802
            parsed = urllib.parse.urlsplit(self.path)
            if parsed.path == "/__reset":
                state.reset()
                self.send_json(200, {"reset": True})
                return
            if parsed.path.endswith("/lookup"):
                state.record(self)
                self.send_json(200, {
                    "status": "ok",
                    "results": [{
                        "id": "acoustid-1",
                        "score": 0.97,
                        "recordings": [{
                            "id": "recording-1",
                            "title": "Fixture Track",
                            "artists": [{"name": "Fixture Artist"}],
                        }],
                    }],
                })
                return
            self.send_json(404, {"error": "not found"})

    return Handler


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, required=True)
    args = parser.parse_args()
    server = ThreadingHTTPServer(("127.0.0.1", args.port), handler_type(State()))
    server.serve_forever()


if __name__ == "__main__":
    main()
