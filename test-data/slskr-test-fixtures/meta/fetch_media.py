#!/usr/bin/env python3
"""Install bounded, hash-pinned optional media; preserve invalid caches."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tempfile
import time
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener

ROOT = Path(__file__).resolve().parent.parent
CHUNK = 64 * 1024
MAX_ASSET = 128 * 1024 * 1024
MAX_TOTAL = 256 * 1024 * 1024
DEADLINE_SECONDS = 180


def https_url(url: str) -> str:
    parsed = urlsplit(url)
    if (parsed.scheme != 'https' or not parsed.hostname or parsed.username
            or parsed.password or parsed.fragment):
        raise ValueError('Media URLs must use HTTPS without credentials or fragments')
    return url


class HttpsRedirects(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return super().redirect_request(req, fp, code, msg, headers, https_url(newurl))


def destination(root: Path, relative: str) -> Path:
    if (not isinstance(relative, str) or not re.fullmatch(r'[A-Za-z0-9_./-]+', relative)
            or any(part in ('', '.', '..') for part in relative.split('/'))):
        raise ValueError('Invalid media path')
    path = root
    for part in relative.split('/'):
        path = path / part
        if path.is_symlink():
            raise ValueError('Media paths must not contain symlinks')
    return path


def declarations(root: Path) -> list[dict]:
    with (root / 'meta' / 'manifest.json').open('rb') as source:
        raw = source.read(128 * 1024 + 1)
    if len(raw) > 128 * 1024:
        raise ValueError('Media manifest exceeds 128 KiB')
    manifest = json.loads(raw)
    downloads = []
    seen = {entry['path'] for entry in manifest.get('files', [])}
    total = 0
    for asset in manifest.get('assets', []):
        for entry in asset.get('download_via_script', []):
            destination(root, entry['path'])
            https_url(entry['url'])
            size = entry['bytes']
            if (type(size) is not int or not 0 < size <= MAX_ASSET
                    or not isinstance(entry['sha256'], str)
                    or not re.fullmatch(r'[0-9a-f]{64}', entry['sha256'])
                    or entry['path'] in seen):
                raise ValueError('Invalid or duplicate pinned media declaration')
            seen.add(entry['path'])
            total += size
            downloads.append(entry)
            if len(downloads) > 16 or total > MAX_TOTAL:
                raise ValueError('Media download set exceeds count or byte budget')
    return downloads


def verify(path: Path, entry: dict) -> None:
    if not path.is_file() or path.stat().st_size != entry['bytes']:
        raise ValueError(f"Media byte count mismatch: {entry['path']}")
    digest = hashlib.sha256()
    with path.open('rb') as source:
        remaining = entry['bytes']
        while remaining:
            data = source.read(min(CHUNK, remaining))
            if not data:
                raise ValueError('Media cache changed during validation')
            digest.update(data)
            remaining -= len(data)
        if source.read(1):
            raise ValueError('Media cache changed during validation')
    if digest.hexdigest() != entry['sha256']:
        raise ValueError(f"Media SHA-256 mismatch: {entry['path']}")


def download(root: Path, entry: dict, opener=None) -> None:
    output = destination(root, entry['path'])
    if output.exists():
        verify(output, entry)
        print(f"OK: verified cache: {entry['path']}")
        return
    output.parent.mkdir(parents=True, exist_ok=True)
    destination(root, entry['path'])
    opener = opener or build_opener(HttpsRedirects())
    deadline = time.monotonic() + DEADLINE_SECONDS
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=output.parent, prefix='.media-', delete=False) as sink:
            temporary = Path(sink.name)
            request = Request(https_url(entry['url']), headers={'User-Agent': 'slskr-fixtures/2'})
            with opener.open(request, timeout=15) as response:
                https_url(response.geturl())
                remaining = entry['bytes']
                digest = hashlib.sha256()
                while remaining:
                    if time.monotonic() >= deadline:
                        raise TimeoutError('Media download exceeded its deadline')
                    data = response.read(min(CHUNK, remaining))
                    if not data:
                        raise ValueError('Media download is truncated')
                    if len(data) > min(CHUNK, remaining):
                        raise ValueError('Media download exceeds its pinned size')
                    sink.write(data)
                    digest.update(data)
                    remaining -= len(data)
                if time.monotonic() >= deadline or response.read(1):
                    raise ValueError('Media download exceeds its size or deadline')
                if digest.hexdigest() != entry['sha256']:
                    raise ValueError('Media download SHA-256 mismatch')
            sink.flush()
            os.fsync(sink.fileno())
        destination(root, entry['path'])
        if output.exists():
            raise ValueError('Media destination appeared during download')
        temporary.chmod(0o644)
        os.replace(temporary, output)
        print(f"OK: installed verified media: {entry['path']}")
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main() -> int:
    for entry in declarations(ROOT):
        download(ROOT, entry)
    print('OK: pinned media verified; static manifest hashes remain unchanged.')
    return 0


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError) as error:
        print(f'ERROR: {error}', file=sys.stderr)
        raise SystemExit(1)
