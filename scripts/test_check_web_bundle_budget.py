#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check-web-bundle-budget.py")
SPEC = importlib.util.spec_from_file_location("check_web_bundle_budget", SCRIPT)
assert SPEC and SPEC.loader
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class WebBundleBudgetTests(unittest.TestCase):
    def test_default_and_named_budgets_are_reported(self) -> None:
        with tempfile.TemporaryDirectory(prefix="slskr-web-budget-") as directory:
            assets = Path(directory) / "assets"
            assets.mkdir()
            (assets / "System-hash.js").write_bytes(b"x" * 1024)
            (assets / "unknown-hash.js").write_bytes(b"x" * 1024)

            report = module.inspect_build(Path(directory))

            records = {record["asset"]: record for record in report["assets"]}
            self.assertEqual(records["System-hash.js"]["budgetKiB"], 360)
            self.assertEqual(records["unknown-hash.js"]["budgetKiB"], 700)
            self.assertTrue(report["passed"])

    def test_named_budget_violation_is_reported(self) -> None:
        with tempfile.TemporaryDirectory(prefix="slskr-web-budget-") as directory:
            assets = Path(directory) / "assets"
            assets.mkdir()
            (assets / "MediaCore-hash.js").write_bytes(b"x" * (451 * 1024))

            report = module.inspect_build(Path(directory))

            self.assertFalse(report["passed"])
            self.assertEqual(
                report["violations"][0]["asset"], "MediaCore-hash.js"
            )


if __name__ == "__main__":
    unittest.main()
