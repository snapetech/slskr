#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check-web-bundle-budget.py")
SPEC = importlib.util.spec_from_file_location("check_web_bundle_budget", SCRIPT)
assert SPEC and SPEC.loader
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class WebBundleBudgetTests(unittest.TestCase):
    @staticmethod
    def write_entry(
        directory: Path,
        references: list[str],
        *,
        asset_prefix: str = "./assets",
    ) -> None:
        (directory / "index.html").write_text(
            "\n".join(
                [
                    f'<script type="module" src="{asset_prefix}/entry.js"></script>',
                    *[
                        f'<link rel="modulepreload" href="{asset_prefix}/{name}">'
                        for name in references
                    ],
                ]
            ),
            encoding="utf-8",
        )

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
            self.assertEqual(report["initialJsBytes"], 2048)
            self.assertEqual(
                report["gzipBytes"],
                sum(module.gzip_size(path) for path in sorted(assets.glob("*.js"))),
            )

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

    def test_initial_js_uses_entry_and_modulepreload_assets(self) -> None:
        with tempfile.TemporaryDirectory(prefix="slskr-web-budget-") as directory:
            build = Path(directory)
            assets = build / "assets"
            assets.mkdir()
            (assets / "entry.js").write_bytes(b"entry" * 100)
            (assets / "runtime.js").write_bytes(b"runtime" * 100)
            (assets / "lazy.js").write_bytes(b"lazy" * 100)
            (assets / "styles.css").write_bytes(b"body {}")
            self.write_entry(build, ["runtime.js"])

            report = module.inspect_build(
                build,
                initial_js_max_kib=1,
                gzip_max_kib=10,
            )

            self.assertEqual(report["initialJsAssets"], ["entry.js", "runtime.js"])
            self.assertEqual(report["initialJsBytes"], 1200)
            self.assertEqual(
                report["gzipBytes"],
                sum(module.gzip_size(path) for path in sorted(assets.glob("*.js"))),
            )
            self.assertEqual(report["aggregateViolations"][0]["check"], "initial-js")
            self.assertFalse(report["passed"])

    def test_dashboard_root_relative_entry_is_supported(self) -> None:
        with tempfile.TemporaryDirectory(prefix="slskr-dashboard-budget-") as directory:
            build = Path(directory)
            assets = build / "assets"
            assets.mkdir()
            (assets / "entry.js").write_bytes(b"entry" * 100)
            (assets / "api.js").write_bytes(b"api" * 100)
            self.write_entry(build, ["api.js"], asset_prefix="/assets")

            report = module.inspect_build(build)

            self.assertEqual(report["initialJsAssets"], ["entry.js", "api.js"])
            self.assertTrue(report["passed"])

    def test_gzip_budget_is_deterministic_and_separate_from_asset_budgets(self) -> None:
        with tempfile.TemporaryDirectory(prefix="slskr-web-budget-") as directory:
            build = Path(directory)
            assets = build / "assets"
            assets.mkdir()
            (assets / "entry.js").write_bytes(bytes(range(256)) * 1024)
            self.write_entry(build, [])

            first = module.inspect_build(build, gzip_max_kib=1)
            second = module.inspect_build(build, gzip_max_kib=1)

            self.assertEqual(first["gzipBytes"], second["gzipBytes"])
            self.assertEqual(first["gzipCompressionLevel"], 9)
            self.assertEqual(first["aggregateViolations"][0]["check"], "gzip-js")
            self.assertFalse(first["violations"])

    def test_report_can_be_serialized_with_aggregate_fields(self) -> None:
        with tempfile.TemporaryDirectory(prefix="slskr-web-budget-") as directory:
            assets = Path(directory) / "assets"
            assets.mkdir()
            (assets / "entry.js").write_bytes(b"entry")

            report = module.inspect_build(Path(directory))

            encoded = json.dumps(report, sort_keys=True)
            self.assertIn('"initialJsBudgetKiB": 1150', encoded)
            self.assertIn('"gzipBudgetKiB": 600', encoded)


if __name__ == "__main__":
    unittest.main()
