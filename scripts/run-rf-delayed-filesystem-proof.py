#!/usr/bin/env python3
"""Exercise transfer durability on an isolated, actually mounted slow FUSE filesystem."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time

TEST = 'focused_controller_tests::delayed_filesystem::mounted_sync_delay_preserves_queue_and_crash_recovery'


def stop_owned(process):
    if process is None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait(timeout=5)
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass


def unmount_owned(mount, filesystem):
    try:
        if os.path.ismount(mount):
            subprocess.run(['fusermount3', '-u', str(mount)], timeout=5, check=True)
    finally:
        stop_owned(filesystem)
        # Detach a busy failed mount only inside this fixture's owned temp root.
        if os.path.ismount(mount):
            subprocess.run(['fusermount3', '-uz', str(mount)], timeout=5, check=True)


def interrupt(signum, _frame):
    raise InterruptedError(f'filesystem proof interrupted by signal {signum}')


def main():
    signal.signal(signal.SIGTERM, interrupt)
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=root / 'target/rf-delayed-filesystem-proof.json')
    args = parser.parse_args()
    if not Path('/dev/fuse').exists():
        parser.error('Linux /dev/fuse and libfuse3 development tooling are required')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    source = root / 'scripts/fixtures/rf-delayed-filesystem.c'
    binary = root / 'target/rf-delayed-filesystem'
    flags = subprocess.check_output(['pkg-config', '--cflags', '--libs', 'fuse3'], text=True).split()
    subprocess.run(['cc', '-Wall', '-Wextra', '-Werror', '-O2', str(source), '-o', str(binary), *flags], check=True)
    record = {'schemaVersion': 1, 'evidenceMode': 'controlled delayed mounted FUSE filesystem; not hardware performance benchmark',
              'sourceCommit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
              'worktreeDirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root, text=True)),
              'startedUtc': datetime.now(timezone.utc).isoformat(),
              'harnessSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'filesystemSourceSha256': hashlib.sha256(source.read_bytes()).hexdigest(),
              'testSourceSha256': hashlib.sha256((root / 'crates/slskr/src/focused_controller_tests/delayed_filesystem.rs').read_bytes()).hexdigest(),
              'filesystemBinarySha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'libfuseVersion': subprocess.check_output(['pkg-config', '--modversion', 'fuse3'], text=True).strip(),
              'success': False}
    filesystem = test = None
    try:
        with tempfile.TemporaryDirectory(prefix='slskr-rf-delayed-filesystem-') as directory:
            temporary = Path(directory)
            backing, control, mount = (temporary / name for name in ('backing', 'control', 'mount'))
            for path in (backing, control, mount):
                path.mkdir()
            with args.output.with_suffix('.fuse.log').open('wb') as log:
                filesystem = subprocess.Popen([str(binary), str(backing), str(control), str(mount)],
                                              stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
                try:
                    deadline = time.monotonic() + 5
                    while not os.path.ismount(mount):
                        if filesystem.poll() is not None or time.monotonic() >= deadline:
                            raise RuntimeError('FUSE mount failed; see filesystem log')
                        time.sleep(.02)
                    record['actualFuseMountObserved'] = True
                    (control / 'enabled').touch()
                    with (mount / 'transfer-events.tsv').open('wb') as output:
                        output.write(b'probe')
                        output.flush()
                        started = time.monotonic()
                        os.fsync(output.fileno())
                        elapsed = time.monotonic() - started
                    if elapsed < .45 or not (control / 'observed').exists():
                        raise RuntimeError('mounted filesystem did not delay actual fsync')
                    record['probeFsyncSeconds'] = round(elapsed, 6)
                    (control / 'enabled').unlink()
                    (control / 'observed').unlink()
                    (mount / 'transfer-events.tsv').unlink()
                    environment = os.environ.copy()
                    environment.pop('SLSKR_RF047_DELAYED_CHILD', None)
                    environment.update(SLSKR_RF047_MOUNT=str(mount), SLSKR_RF047_CONTROL=str(control))
                    with args.output.with_suffix('.test.log').open('wb') as test_log:
                        # Cargo uses the pinned toolchain and repo configuration directly.
                        test = subprocess.Popen(['cargo', 'test', '--locked', '-p', 'slskr', '--lib', TEST,
                                                 '--', '--ignored', '--exact', '--nocapture'],
                                                cwd=root, env=environment, stdout=test_log,
                                                stderr=subprocess.STDOUT, start_new_session=True)
                        code = test.wait(timeout=300)
                    if code:
                        raise RuntimeError(f'delayed-storage regression exited {code}; see test log')
                    text = args.output.with_suffix('.test.log').read_text()
                    if '1 passed; 0 failed' not in text or '24 transfers and 48 ordered events recovered' not in text:
                        raise RuntimeError('test did not report the complete mounted-storage proof')
                    record.update(queueWritableDuringMountedFsync=True, killedWriterCycles=3,
                                  recoveredTransfers=24, recoveredOrderedEvents=48, success=True)
                finally:
                    stop_owned(test)
                    unmount_owned(mount, filesystem)
                    record['filesystemProcessReaped'] = filesystem.poll() is not None
    except Exception as error:
        record['success'] = False
        record['error'] = str(error)
        raise
    finally:
        stop_owned(test)
        stop_owned(filesystem)
        args.output.write_text(json.dumps(record, indent=2) + '\n')
    print(json.dumps(record, indent=2))


if __name__ == '__main__':
    main()
