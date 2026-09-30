#!/usr/bin/env python3
"""Offline regressions for the optional, pinned fixture downloader."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('fetch_media', Path(__file__).resolve().parents[1] / 'test-data/slskr-test-fixtures/meta/fetch_media.py')
fetch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fetch)


class Response(io.BytesIO):
    def read(self, size=-1):
        assert 0 < size <= fetch.CHUNK, 'unbounded network read'
        return super().read(size)

    def geturl(self):
        return 'https://example.org/media'


class Opener:
    def __init__(self, data):
        self.data = data

    def open(self, request, timeout):
        assert timeout == 15
        return Response(self.data)


class MediaTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / 'meta').mkdir()
        self.data = b'encoded-content-unit-stub' * 3000
        self.entry = {'path': 'movie/clip.mp4', 'url': 'https://example.org/media',
                      'bytes': len(self.data), 'sha256': hashlib.sha256(self.data).hexdigest()}

    def manifest(self, entries):
        (self.root / 'meta/manifest.json').write_text(json.dumps({'assets': [{'download_via_script': entries}]}))
        return fetch.declarations(self.root)

    def test_install_and_verified_cache(self):
        self.assertEqual(self.manifest([self.entry]), [self.entry])
        fetch.download(self.root, self.entry, Opener(self.data))
        self.assertEqual((self.root / self.entry['path']).read_bytes(), self.data)
        fetch.download(self.root, self.entry, object())
        self.assertEqual(list((self.root / 'movie').glob('.media-*')), [])

    def test_truncated_oversize_and_wrong_hash_cleanup(self):
        for data in [self.data[:-1], self.data + b'x', b'x' * len(self.data)]:
            with self.subTest(size=len(data)), self.assertRaises(ValueError):
                fetch.download(self.root, self.entry, Opener(data))
            self.assertFalse((self.root / self.entry['path']).exists())
            self.assertEqual(list((self.root / 'movie').glob('.media-*')), [])

    def test_invalid_cache_is_preserved(self):
        path = self.root / self.entry['path']
        path.parent.mkdir()
        for data in [b'old', b'x' * len(self.data)]:
            path.write_bytes(data)
            with self.assertRaises(ValueError):
                fetch.download(self.root, self.entry, object())
            self.assertEqual(path.read_bytes(), data)

    def test_path_and_symlink_rejection(self):
        for name in ['/absolute', '../escape', 'a/../b', 'a//b', 'a\\b', '.', 'a/./b']:
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.manifest([{**self.entry, 'path': name}])
        (self.root / 'movie').symlink_to(self.root / 'meta', target_is_directory=True)
        with self.assertRaises(ValueError):
            self.manifest([self.entry])

    def test_url_and_pin_rejection(self):
        for url in ['http://example.org/media', 'file:///tmp/media', 'https://user:password@example.org/media']:
            with self.subTest(url=url), self.assertRaises(ValueError):
                self.manifest([{**self.entry, 'url': url}])
        for value in [0, True, fetch.MAX_ASSET + 1]:
            with self.assertRaises(ValueError):
                self.manifest([{**self.entry, 'bytes': value}])
        with self.assertRaises(ValueError):
            self.manifest([{**self.entry, 'sha256': 'observed-hash'}])
        with self.assertRaises(ValueError):
            fetch.HttpsRedirects().redirect_request(None, None, 302, '', {}, 'http://example.org/media')

    def test_duplicate_static_collision_and_budget_rejection(self):
        with self.assertRaises(ValueError):
            self.manifest([self.entry, self.entry])
        (self.root / 'meta/manifest.json').write_text(json.dumps({'files': [{'path': self.entry['path']}], 'assets': [{'download_via_script': [self.entry]}]}))
        with self.assertRaises(ValueError):
            fetch.declarations(self.root)
        with self.assertRaises(ValueError):
            self.manifest([{**self.entry, 'path': f'movie/{i}.mp4'} for i in range(17)])
        with self.assertRaises(ValueError):
            self.manifest([{**self.entry, 'path': f'movie/{i}.mp4', 'bytes': fetch.MAX_ASSET} for i in range(3)])

    def test_deadline_and_network_failure_cleanup(self):
        with patch.object(fetch.time, 'monotonic', side_effect=[0, fetch.DEADLINE_SECONDS + 1]):
            with self.assertRaises(TimeoutError):
                fetch.download(self.root, self.entry, Opener(self.data))
        with patch.object(Opener, 'open', side_effect=TimeoutError('socket timeout')):
            with self.assertRaises(TimeoutError):
                fetch.download(self.root, self.entry, Opener(self.data))
        self.assertFalse((self.root / self.entry['path']).exists())
        self.assertEqual(list((self.root / 'movie').glob('.media-*')), [])

    def test_empty_and_oversize_manifest(self):
        self.assertEqual(self.manifest([]), [])
        (self.root / 'meta/manifest.json').write_bytes(b' ' * (128 * 1024 + 1))
        with self.assertRaises(ValueError):
            fetch.declarations(self.root)


if __name__ == '__main__':
    unittest.main()
