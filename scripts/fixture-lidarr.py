#!/usr/bin/env python3
"""Deterministic local Lidarr API fixture for frozen runtime differentials."""

from __future__ import annotations

import argparse
import json
import threading
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


WANTED = {
    "totalRecords": 3,
    "records": [
        {
            "id": 1,
            "title": "Fixture Album One",
            "foreignAlbumId": "release-one",
            "artist": {
                "id": 11,
                "artistName": "Fixture Artist One",
                "foreignArtistId": "artist-one",
            },
        },
        {
            "id": 2,
            "title": "Fixture Album Two",
            "foreignAlbumId": "release-two",
            "artist": {
                "id": 12,
                "artistName": "Fixture Artist Two",
                "foreignArtistId": "artist-two",
            },
        },
        {
            "id": 3,
            "title": "Fixture Album Three",
            "foreignAlbumId": "release-three",
            "artist": {
                "id": 13,
                "artistName": "Fixture Artist Three",
                "foreignArtistId": "artist-three",
            },
        },
    ],
}

MANUAL_IMPORT = [
    {
        "id": 101,
        "path": "/lidarr/Fixture Album/01 - Fixture.flac",
        "name": "01 - Fixture.flac",
        "artist": {"id": 11, "artistName": "Fixture Artist One"},
        "album": {"id": 21, "title": "Fixture Album"},
        "albumReleaseId": 31,
        "tracks": [{"id": 41, "title": "Fixture Track"}],
        "quality": {"quality": {"id": 7, "name": "FLAC"}},
        "releaseGroup": "fixture",
        "downloadId": "download-1",
        "indexerFlags": 0,
        "rejections": [],
        "additionalFile": False,
        "replaceExistingFiles": False,
        "disableReleaseSwitching": False,
    },
    {
        "id": 102,
        "path": "/lidarr/Fixture Album/02 - Ambiguous.flac",
        "name": "02 - Ambiguous.flac",
        "artist": {"id": 11, "artistName": "Fixture Artist One"},
        "album": {"id": 21, "title": "Fixture Album"},
        "albumReleaseId": 31,
        "tracks": [{"id": 42, "title": "Ambiguous Track"}],
        "quality": {"quality": {"id": 7, "name": "FLAC"}},
        "rejections": [{"reason": "Ambiguous release"}],
        "additionalFile": False,
        "replaceExistingFiles": False,
        "disableReleaseSwitching": False,
    },
]


class State:
    def __init__(self) -> None:
        self.lock = threading.Lock()
        self.requests: list[dict[str, object]] = []

    def reset(self) -> None:
        with self.lock:
            self.requests.clear()

    def record(self, handler: BaseHTTPRequestHandler, body: bytes) -> None:
        parsed = urllib.parse.urlsplit(handler.path)
        query = urllib.parse.parse_qs(parsed.query, keep_blank_values=True)
        decoded_body: object = body.decode("utf-8", errors="replace")
        if body:
            try:
                decoded_body = json.loads(body)
            except json.JSONDecodeError:
                pass
        with self.lock:
            self.requests.append(
                {
                    "method": handler.command,
                    "path": parsed.path,
                    "query": {key: values for key, values in sorted(query.items())},
                    "apiKey": handler.headers.get("X-Api-Key", ""),
                    "body": decoded_body,
                }
            )

    def snapshot(self) -> dict[str, object]:
        with self.lock:
            return {"requests": list(self.requests)}


def handler_type(state: State) -> type[BaseHTTPRequestHandler]:
    class Handler(BaseHTTPRequestHandler):
        server_version = "LidarrFixture/1"

        def log_message(self, *_args: object) -> None:
            return

        def send_json(self, status: int, value: object) -> None:
            body = json.dumps(value, separators=(",", ":")).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def read_body(self) -> bytes:
            if self.headers.get("Transfer-Encoding", "").lower() != "chunked":
                return self.rfile.read(int(self.headers.get("Content-Length", "0")))
            chunks: list[bytes] = []
            while True:
                size_line = self.rfile.readline().split(b";", 1)[0].strip()
                size = int(size_line, 16)
                if size == 0:
                    self.rfile.readline()
                    break
                chunks.append(self.rfile.read(size))
                self.rfile.read(2)
            return b"".join(chunks)

        def do_GET(self) -> None:  # noqa: N802
            parsed = urllib.parse.urlsplit(self.path)
            if parsed.path == "/__status":
                self.send_json(200, state.snapshot())
                return
            state.record(self, b"")
            if parsed.path == "/api/v1/system/status":
                self.send_json(200, {"appName": "Lidarr", "version": "2.1.0"})
            elif parsed.path == "/api/v1/wanted/missing":
                self.send_json(200, WANTED)
            elif parsed.path == "/api/v1/manualimport":
                self.send_json(200, MANUAL_IMPORT)
            else:
                self.send_json(404, {"error": "not found"})

        def do_POST(self) -> None:  # noqa: N802
            body = self.read_body()
            parsed = urllib.parse.urlsplit(self.path)
            if parsed.path == "/__reset":
                state.reset()
                self.send_json(200, {"reset": True})
                return
            state.record(self, body)
            if parsed.path == "/api/v1/command":
                self.send_json(
                    200,
                    {"id": 42, "name": "ManualImport", "status": "queued"},
                )
            else:
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
