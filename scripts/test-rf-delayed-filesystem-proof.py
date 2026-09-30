#!/usr/bin/env python3
"""Check isolated mount cleanup, including failed normal unmount."""
import importlib.util
from pathlib import Path
import signal
import subprocess
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('slowfs', Path(__file__).with_name('run-rf-delayed-filesystem-proof.py'))
slowfs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(slowfs)


class CleanupTests(unittest.TestCase):
    def test_no_process_is_safe(self):
        slowfs.stop_owned(None)

    def test_owned_group_gets_graceful_then_hard_cleanup(self):
        process = Mock(pid=1234)
        with patch.object(slowfs.os, 'killpg') as kill:
            slowfs.stop_owned(process)
        self.assertEqual([call.args[1] for call in kill.call_args_list], [signal.SIGTERM, signal.SIGKILL])
        process.wait.assert_called_once_with(timeout=5)

    def test_timeout_still_reaps_process(self):
        process = Mock(pid=1234)
        process.wait.side_effect = [subprocess.TimeoutExpired('fixture', 5), 0]
        with patch.object(slowfs.os, 'killpg'):
            slowfs.stop_owned(process)
        self.assertEqual(process.wait.call_count, 2)

    def test_failed_unmount_stops_fixture_and_detaches_owned_mount(self):
        mount = Path('/tmp/owned-mount')
        with patch.object(slowfs.os.path, 'ismount', return_value=True), \
             patch.object(slowfs, 'stop_owned') as stop, \
             patch.object(slowfs.subprocess, 'run') as run:
            run.side_effect = [subprocess.CalledProcessError(1, 'fusermount3'), Mock(returncode=0)]
            with self.assertRaises(subprocess.CalledProcessError):
                slowfs.unmount_owned(mount, 'owned-process')
            stop.assert_called_once_with('owned-process')
            self.assertEqual(run.call_args.args[0], ['fusermount3', '-uz', str(mount)])

    def test_removed_mount_does_not_attempt_detach(self):
        with patch.object(slowfs.os.path, 'ismount', side_effect=[True, False]), \
             patch.object(slowfs, 'stop_owned') as stop, \
             patch.object(slowfs.subprocess, 'run') as run:
            slowfs.unmount_owned(Path('/tmp/owned-mount'), 'owned-process')
            self.assertEqual(run.call_count, 1)
            stop.assert_called_once()


if __name__ == '__main__':
    unittest.main()
