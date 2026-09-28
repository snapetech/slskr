#!/usr/bin/env python3
"""Execute the actual workflow verifier against bounded checksum fixtures."""
import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / '.github/workflows/publish-chocolatey.yml'
NAME = 'slskr-v0.2.40-x86_64-pc-windows-msvc.zip'
PAYLOAD = b'isolated checksum regression fixture\n'
SHA = hashlib.sha256(PAYLOAD).hexdigest()
# Hosted Linux PowerShell cold startup exceeded the old ten-second bound.
# Retain a deadline and subprocess.run's kill/wait cleanup on expiration.
POWERSHELL_TIMEOUT_SECONDS = 60


def verifier():
    text = WORKFLOW.read_text()
    section = text.split('      - name: Verify release asset checksum\n', 1)[1]
    section = section.split('      - name: Resolve package version\n', 1)[0]
    return textwrap.dedent(section.split('        run: |\n', 1)[1])


@unittest.skipUnless(shutil.which('pwsh'), 'requires PowerShell; clean Windows runner supplies it')
class ChecksumTests(unittest.TestCase):
    def run_fixture(self, rows, succeeds):
        with tempfile.TemporaryDirectory(prefix='slskr-chocolatey-checksum-') as directory:
            root = Path(directory)
            assets = root / 'release-assets'
            assets.mkdir()
            (assets / NAME).write_bytes(PAYLOAD)
            (assets / 'SHA256SUMS.txt').write_text(rows)
            script = root / 'verify.ps1'
            script.write_text(verifier())
            result = subprocess.run(['pwsh', '-NoLogo', '-NoProfile', '-File', str(script)],
                                    cwd=root, capture_output=True, text=True,
                                    timeout=POWERSHELL_TIMEOUT_SECONDS)
            if succeeds:
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn('Verified SHA256', result.stdout)
            else:
                self.assertNotEqual(result.returncode, 0)

    def test_plain_basename(self):
        self.run_fixture(f'{SHA}  {NAME}\n', True)

    def test_published_dot_prefix(self):
        self.run_fixture(f'{SHA}  ./{NAME}\n', True)

    def test_binary_mode_dot_prefix(self):
        self.run_fixture(f'{SHA} *./{NAME}\n', True)

    def test_missing_entry(self):
        self.run_fixture(f'{SHA}  unrelated.zip\n', False)

    def test_duplicate_aliases(self):
        self.run_fixture(f'{SHA}  {NAME}\n{SHA}  ./{NAME}\n', False)

    def test_wrong_hash(self):
        self.run_fixture(f'{"0" * 64}  ./{NAME}\n', False)

    def test_other_directory_is_rejected(self):
        self.run_fixture(f'{SHA}  elsewhere/{NAME}\n', False)

    def test_suffix_is_rejected(self):
        self.run_fixture(f'{SHA}  ./{NAME}.other\n', False)


if __name__ == '__main__':
    unittest.main()
