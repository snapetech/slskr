#!/usr/bin/env python3
"""Keep synthetic endpoint diagnostics out of rendered UI workflow evidence."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from parity_audit_webui import validate_ui_workflow_audit, webui_workflow_ledger


class WorkflowEvidence(unittest.TestCase):
    def setUp(self):
        self.work = tempfile.TemporaryDirectory(prefix="slskr-workflow-evidence-")
        self.addCleanup(self.work.cleanup)
        self.root = Path(self.work.name)
        self.report = {"slskd": {"endpoints": ["GET /jobs"]}, "slskdn": {"endpoints": []}}
        self.scenarios = ("success", "rendered-loading-and-empty",
                          "rendered-validation-and-server-error", "authorization-reconnect-and-restart")

    def audit(self, scenario):
        status = {"success": 200, "rendered-loading-and-empty": 200,
                  "rendered-validation-and-server-error": 422,
                  "authorization-reconnect-and-restart": 401}[scenario]
        return {"endpointSweepCount": 0, "errors": [], "routes": [{"apiResponses": [
            {"method": "GET", "path": "/api/v0/jobs", "status": status},
            {"method": "GET", "path": "/api/v0/not-a-target", "status": status},
        ]}]}

    def write_audits(self):
        for scenario in self.scenarios:
            directory = self.root / "target/react-webui-audit"
            if scenario != "success":
                directory /= f"scenarios/{scenario}"
            directory.mkdir(parents=True)
            (directory / "audit.json").write_text(json.dumps(self.audit(scenario)))

    def test_zero_sweep_evidence_credits_only_observed_target_requests(self):
        self.write_audits()
        ledger = webui_workflow_ledger(self.root, self.report, True)
        self.assertEqual(len(ledger), 4)
        for case in ledger.values():
            self.assertEqual(case, {"GET /jobs": True})

    def test_synthetic_sweep_is_rejected(self):
        self.write_audits()
        path = self.root / "target/react-webui-audit/audit.json"
        audit = self.audit("success")
        audit["endpointSweepCount"] = 417
        path.write_text(json.dumps(audit))
        with self.assertRaisesRegex(RuntimeError, "zero synthetic"):
            webui_workflow_ledger(self.root, self.report, True)

    def test_unmarked_legacy_evidence_is_rejected(self):
        with self.assertRaisesRegex(RuntimeError, "regenerate"):
            validate_ui_workflow_audit({"errors": [], "routes": []})

    def test_noninteger_and_negative_sweep_counts_are_rejected(self):
        for count in (False, True, -1, "0", 0.0, None):
            with self.subTest(count=count), self.assertRaises(RuntimeError):
                validate_ui_workflow_audit({"endpointSweepCount": count})

    def test_errors_are_not_converted_into_workflow_proof(self):
        self.write_audits()
        path = self.root / "target/react-webui-audit/audit.json"
        audit = self.audit("success")
        audit["errors"] = ["broken page"]
        path.write_text(json.dumps(audit))
        with self.assertRaisesRegex(RuntimeError, "contains errors"):
            webui_workflow_ledger(self.root, self.report, True)

    def test_fresh_ledger_removes_inherited_synthetic_sweep_settings(self):
        seen = []

        def run(command, **kwargs):
            environment = kwargs.get("env")
            if environment is not None:
                seen.append(environment)
                directory = Path(environment["SLSKR_REACT_WEB_AUDIT_DIR"])
                directory.mkdir(parents=True)
                scenario = environment["SLSKR_REACT_WEB_AUDIT_SCENARIO"]
                (directory / "audit.json").write_text(json.dumps(self.audit(scenario)))
            return subprocess.CompletedProcess(command, 0)

        with patch.dict(os.environ, {"SLSKR_REACT_WEB_AUDIT_ENDPOINT_SWEEP": "poison"}), \
                patch("parity_audit_webui.subprocess.run", side_effect=run):
            ledger = webui_workflow_ledger(self.root, self.report, False)
        self.assertEqual(len(seen), 4)
        self.assertTrue(all("SLSKR_REACT_WEB_AUDIT_ENDPOINT_SWEEP" not in env for env in seen))
        self.assertTrue(all(case == {"GET /jobs": True} for case in ledger.values()))


if __name__ == "__main__":
    unittest.main()
