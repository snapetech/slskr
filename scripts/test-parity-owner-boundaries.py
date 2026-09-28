#!/usr/bin/env python3
"""Keep source-bound parity guards attached to the active Rust owners."""

from __future__ import annotations

import ast
import importlib.util
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent


def load_script(name: str):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), SCRIPTS / name)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


MANIFEST = load_script('audit-parity-manifest.py')
CONFIG = load_script('audit-upstream-config-surface.py')


class ParityOwnerBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix='slskr-parity-owner-')
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.source = self.root / 'crates/slskr/src'
        self.source.mkdir(parents=True)
        (self.root / 'scripts').mkdir()
        (self.root / 'docs').mkdir()
        (self.root / 'docs/slskr.config.example.toml').write_text('[core]\nenabled = true\n')
        (self.source / 'cli.rs').write_text('')
        (self.source / 'lib.rs').write_text('mod hash_backfill_runtime;\n')
        (self.source / 'hash_backfill_runtime.rs').write_text(
            (SCRIPTS.parent / 'crates/slskr/src/hash_backfill_runtime.rs').read_text()
        )
        self.runner = self.root / 'scripts/run-slskdn-cross-client-interop.sh'
        self.runner.write_text((SCRIPTS / self.runner.name).read_text())

    def test_audit_owners_are_bounded_and_keep_the_entry_point(self):
        registry = SCRIPTS / 'audit-parity-manifest.py'
        self.assertLessEqual(len(registry.read_bytes().splitlines()), 700)
        functions = [node.name for node in ast.parse(registry.read_bytes()).body
                     if isinstance(node, ast.FunctionDef)]
        self.assertEqual(functions, ['main'])
        owners = sorted(SCRIPTS.glob('parity_audit_*.py'))
        self.assertEqual(len(owners), 16)
        for owner in owners:
            self.assertLessEqual(len(owner.read_bytes().splitlines()), 1000, owner.name)
        self.assertEqual(MANIFEST.run_json.__module__, 'parity_audit_process')
        self.assertEqual(MANIFEST.validate_live_interop_mapping_contracts.__module__,
                         'parity_audit_frozen_transport')

    def test_backfill_guard_accepts_registered_owner(self):
        MANIFEST.validate_live_interop_mapping_contracts(self.root)

    def test_backfill_guard_rejects_disconnected_owner(self):
        (self.source / 'lib.rs').write_text('')
        with self.assertRaisesRegex(ValueError, 'owner is absent'):
            MANIFEST.validate_live_interop_mapping_contracts(self.root)

    def test_backfill_guard_rejects_old_registry_decoy(self):
        owner = self.source / 'hash_backfill_runtime.rs'
        (self.source / 'lib.rs').write_text('mod hash_backfill_runtime;\n' + owner.read_text())
        owner.unlink()
        with self.assertRaisesRegex(ValueError, 'FLAC-header implementation'):
            MANIFEST.validate_live_interop_mapping_contracts(self.root)

    def test_backfill_guard_rejects_missing_transfer_token_check(self):
        owner = self.source / 'hash_backfill_runtime.rs'
        owner.write_text(owner.read_text().replace('backfill transfer token did not match', 'removed'))
        with self.assertRaisesRegex(ValueError, 'FLAC-header implementation'):
            MANIFEST.validate_live_interop_mapping_contracts(self.root)

    def test_backfill_guard_rejects_missing_remote_route(self):
        self.runner.write_text(self.runner.read_text().replace('/api/v0/backfill/file', '/removed'))
        with self.assertRaisesRegex(ValueError, 'remote route/hash assertion'):
            MANIFEST.validate_live_interop_mapping_contracts(self.root)

    def test_configuration_inventory_reads_owners_and_excludes_test_decoys(self):
        owner = self.source / 'config_parts'
        owner.mkdir()
        (owner / 'peer_transport.rs').write_text('const KEY: &str = "SLSKR_OWNED_CONFIG";')
        tests = self.source / 'config_tests'
        tests.mkdir()
        (tests / 'peer_transport.rs').write_text('let decoy = "SLSKR_TEST_DECOY";')
        (self.source / 'controller_tests.rs').write_text('let decoy = "SLSKR_TEST_DECOY";')
        inventory = CONFIG.slskr_inventory(self.root)
        self.assertIn('SLSKR_OWNED_CONFIG', inventory['environmentVariables'])
        self.assertNotIn('SLSKR_TEST_DECOY', inventory['environmentVariables'])
        (owner / 'peer_transport.rs').unlink()
        self.assertNotIn('SLSKR_OWNED_CONFIG', CONFIG.slskr_inventory(self.root)['environmentVariables'])


if __name__ == '__main__':
    unittest.main()
