#!/usr/bin/env python3
"""Regression checks for honest browser evidence and owned-process cleanup."""
import importlib.util
import os
from pathlib import Path
import signal
import sys
import tempfile
import unittest
from unittest.mock import patch
import subprocess

spec = importlib.util.spec_from_file_location('nightly', Path(__file__).with_name('run-react-nightly-audit.py'))
nightly = importlib.util.module_from_spec(spec)
spec.loader.exec_module(nightly)


class NightlyTests(unittest.TestCase):
    def audit(self):
        return {'evidenceMode': 'mock', 'scenario': 'success', 'endpointSweepCount': 0,
                'errors': [], 'routes': [{'viewport': viewport, 'apiResponses': [{'status': 200}]}
                                        for viewport in ('desktop', 'mobile')]}

    def test_observed_routes(self):
        self.assertEqual(nightly.validate_audit(self.audit(), 'success')['renderChecks'], 2)

    def test_rejects_false_or_incomplete_evidence(self):
        for key, value in [('evidenceMode', 'live'), ('scenario', 'wrong'),
                           ('endpointSweepCount', 1), ('endpointSweepCount', False),
                           ('errors', ['failure']), ('routes', [])]:
            with self.subTest(key=key, value=value):
                audit = self.audit()
                audit[key] = value
                with self.assertRaises(RuntimeError):
                    nightly.validate_audit(audit, 'success')
        audit = self.audit()
        audit['routes'].pop()
        with self.assertRaises(RuntimeError):
            nightly.validate_audit(audit, 'success')
        audit = self.audit()
        for route in audit['routes']:
            route['apiResponses'] = []
        with self.assertRaises(RuntimeError):
            nightly.validate_audit(audit, 'success')

    def test_clears_inherited_live_sweep_and_restrictions(self):
        base = {'PATH': '/bin', 'SLSKR_REACT_WEB_AUDIT_TOKEN': 'private',
                'SLSKR_REACT_WEB_AUDIT_BACKEND_URL': 'https://example.invalid',
                'SLSKR_REACT_WEB_AUDIT_ENDPOINT_SWEEP': '["synthetic"]',
                'SLSKR_REACT_WEB_AUDIT_ROUTES': '/', 'HEADLESS': 'false'}
        result = nightly.scenario_environment(base, 'success', Path('/tmp/report'))
        for key in base:
            if key.startswith('SLSKR_REACT_WEB_AUDIT_'):
                self.assertNotIn(key, result)
        self.assertEqual(result['PATH'], '/bin')
        self.assertEqual(result['HEADLESS'], 'true')
        self.assertIn('SLSKR_REACT_WEB_AUDIT_ROUTES', nightly.scenario_environment(base, nightly.SCENARIOS[1], Path('/tmp/report')))

    def test_timeout_kills_and_reaps_owned_group(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pid_file = root / 'pid'
            code = 'import os,time; from pathlib import Path; Path("pid").write_text(str(os.getpid())); time.sleep(30)'
            with self.assertRaises(subprocess.TimeoutExpired):
                nightly.run_owned([sys.executable, '-c', code], root, os.environ.copy(), root / 'log', timeout=.3)
            pid = int(pid_file.read_text())
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)

    def test_guarded_timeout_stops_external_service(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            code = 'import os,time; from pathlib import Path; Path("' + directory + '/pid").write_text(str(os.getpid())); time.sleep(30)'
            guard = str(Path(__file__).resolve().with_name('with-process-memory-guard.sh'))
            with self.assertRaises(subprocess.TimeoutExpired):
                nightly.run_owned([guard, sys.executable, '-c', code], root,
                                  os.environ.copy(), root / 'log', timeout=1)
            pid = int((root / 'pid').read_text())
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)

    def test_failed_exit_cleans_group(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(nightly.os, 'killpg', wraps=os.killpg) as kill:
                with self.assertRaises(RuntimeError):
                    nightly.run_owned([sys.executable, '-c', 'raise SystemExit(7)'], root, os.environ.copy(), root / 'log')
                self.assertEqual([call.args[1] for call in kill.call_args_list], [signal.SIGTERM, signal.SIGKILL])


if __name__ == '__main__':
    unittest.main()
