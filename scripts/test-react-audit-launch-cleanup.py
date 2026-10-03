#!/usr/bin/env python3
"""Prove a failed browser launch closes the actual audit's temporary server."""

from pathlib import Path
import json
import os
import shutil
import subprocess
import tempfile
import unittest


REPO = Path(__file__).resolve().parent.parent


class LaunchCleanup(unittest.TestCase):
    def test_failed_launch_exits_without_a_stale_http_listener(self):
        with tempfile.TemporaryDirectory(prefix="slskr-audit-launch-") as directory:
            root = Path(directory)
            scripts = root / "web/scripts"
            scripts.mkdir(parents=True)
            for name in ("audit-react-webui.mjs", "audit-response-contracts.mjs"):
                shutil.copyfile(REPO / "web/scripts" / name, scripts / name)
            build = root / "web/build"
            build.mkdir()
            (build / "index.html").write_text("<html><head></head><body></body></html>")
            axe_script = root / "web/node_modules/axe-core/axe.min.js"
            axe_script.parent.mkdir(parents=True)
            axe_script.write_text("/* fixture axe script */\n")
            playwright = root / "web/node_modules/@playwright/test"
            playwright.mkdir(parents=True)
            (playwright / "package.json").write_text(json.dumps({
                "type": "module", "exports": "./index.js",
            }))
            (playwright / "index.js").write_text(
                "export const chromium = { launch: async () => { "
                "throw new Error('fixture browser launch failure'); } };\n"
            )
            environment = os.environ.copy()
            for key in list(environment):
                if key.startswith("SLSKR_REACT_WEB_AUDIT_"):
                    del environment[key]
            result = subprocess.run(
                ["node", str(scripts / "audit-react-webui.mjs")],
                cwd=root, env=environment, capture_output=True, text=True, timeout=10,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("fixture browser launch failure", result.stderr)
            self.assertFalse((root / "target/react-webui-audit/audit.json").exists())


if __name__ == "__main__":
    unittest.main()
