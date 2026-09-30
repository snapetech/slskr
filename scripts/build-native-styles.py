#!/usr/bin/env python3
"""Bundle ordered native Web style owners into the existing single CSS asset."""

from __future__ import annotations

import argparse
import json
from pathlib import Path, PurePosixPath

STATIC_ROOT = Path(__file__).resolve().parent.parent / 'crates/slskr-web/static'


def stylesheet_bytes(static_root: Path) -> bytes:
    manifest = json.loads((static_root / 'styles.manifest.json').read_text(encoding='utf-8'))
    if not isinstance(manifest, list) or not manifest:
        raise ValueError('native stylesheet manifest must be a nonempty list')
    paths = []
    for name in manifest:
        if not isinstance(name, str):
            raise ValueError('native stylesheet owner names must be strings')
        path = PurePosixPath(name)
        if len(path.parts) != 2 or path.parts[0] != 'style_parts' or path.suffix != '.css':
            raise ValueError(f'invalid native stylesheet owner: {name!r}')
        if str(path) != name or '\\' in name:
            raise ValueError(f'noncanonical native stylesheet owner: {name!r}')
        paths.append(static_root / name)
    if len(set(paths)) != len(paths):
        raise ValueError('native stylesheet manifest contains duplicate owners')
    available = set((static_root / 'style_parts').glob('*.css'))
    if set(paths) != available:
        raise ValueError('native stylesheet manifest must list every owner exactly once')
    return b''.join(path.read_bytes() for path in paths)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-root', type=Path, default=STATIC_ROOT)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    styles = stylesheet_bytes(args.source_root)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(styles)


if __name__ == '__main__':
    main()
