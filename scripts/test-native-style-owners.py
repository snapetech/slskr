#!/usr/bin/env python3
"""Bounded checks for ordered native Web stylesheet materialization."""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('build_native_styles', SCRIPTS / 'build-native-styles.py')
assert SPEC and SPEC.loader
BUILDER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILDER)


class NativeStyleOwnerTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix='slskr-native-style-owner-')
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.owners = self.root / 'style_parts'
        self.owners.mkdir()
        (self.owners / 'a.css').write_bytes(b'.a { color: red; }\n')
        (self.owners / 'z.css').write_bytes(b'.a { color: blue; }\n')
        self.manifest(['style_parts/z.css', 'style_parts/a.css'])

    def manifest(self, names):
        (self.root / 'styles.manifest.json').write_text(json.dumps(names))

    def test_manifest_order_preserves_the_cascade(self):
        self.assertEqual(BUILDER.stylesheet_bytes(self.root),
                         b'.a { color: blue; }\n.a { color: red; }\n')

    def test_empty_manifest_is_rejected(self):
        self.manifest([])
        with self.assertRaisesRegex(ValueError, 'nonempty'):
            BUILDER.stylesheet_bytes(self.root)

    def test_duplicate_owners_are_rejected(self):
        self.manifest(['style_parts/a.css', 'style_parts/a.css', 'style_parts/z.css'])
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            BUILDER.stylesheet_bytes(self.root)

    def test_unlisted_owners_are_rejected(self):
        self.manifest(['style_parts/a.css'])
        with self.assertRaisesRegex(ValueError, 'every owner'):
            BUILDER.stylesheet_bytes(self.root)

    def test_noncanonical_and_escaping_paths_are_rejected(self):
        for name in ['../outside.css', '/outside.css', 'style_parts/../outside.css',
                     'style_parts//a.css', 'style_parts\\a.css', 17]:
            self.manifest([name])
            with self.subTest(name=name), self.assertRaises(ValueError):
                BUILDER.stylesheet_bytes(self.root)

    def test_repository_owners_are_bounded_and_materialize_one_asset(self):
        styles = BUILDER.stylesheet_bytes(BUILDER.STATIC_ROOT)
        self.assertIn(b'.slskr-modal-backdrop', styles)
        self.assertIn(b'@media (max-width: 760px)', styles)
        for owner in (BUILDER.STATIC_ROOT / 'style_parts').glob('*.css'):
            self.assertLessEqual(len(owner.read_bytes().splitlines()), 1000, owner.name)
        self.assertFalse((BUILDER.STATIC_ROOT / 'styles.css').exists())
        self.assertIn('href="/styles.css"', (BUILDER.STATIC_ROOT / 'index.html').read_text())


if __name__ == '__main__':
    unittest.main()
