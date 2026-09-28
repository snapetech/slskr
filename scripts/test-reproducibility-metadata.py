#!/usr/bin/env python3
"""Validate source-bound local and hosted reproducibility evidence."""

import copy
import hashlib
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SCRIPTS = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("reproducibility", SCRIPTS / "collect-reproducibility-metadata.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class Reproducibility(unittest.TestCase):
    def setUp(self):
        work = tempfile.TemporaryDirectory(prefix="slskr-provenance-")
        self.addCleanup(work.cleanup)
        self.root = Path(work.name)
        for relative in MODULE.PROVENANCE_FILES + MODULE.CONFIGURATION_FILES:
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(relative + "\n")
        self.artifact = self.root / "target/fixture.json"
        self.artifact.parent.mkdir()
        self.artifact.write_text('{"result":"passed"}\n')
        self.source = "a" * 40
        self.github = {
            "GITHUB_ACTIONS": "true", "GITHUB_SERVER_URL": "https://github.com",
            "GITHUB_REPOSITORY": "example/slskr", "GITHUB_RUN_ID": "123",
            "GITHUB_RUN_ATTEMPT": "2", "GITHUB_JOB": "rust", "GITHUB_SHA": self.source,
            "GITHUB_TOKEN": "not-for-metadata",
        }

    def collect(self, environment=None, status="", artifact_name="ci-reproducibility-rust"):
        with patch.object(MODULE, "ROOT", self.root), \
                patch.object(MODULE, "command_version", side_effect=lambda tool: tool + " fixture-version"), \
                patch.object(MODULE, "git_output", side_effect=lambda *args: self.source if args[0] == "rev-parse" else status), \
                patch.dict(os.environ, environment or {}, clear=True):
            return MODULE.collect(["target/fixture.json"], artifact_name)

    def test_local_metadata_hashes_required_files_and_artifacts(self):
        metadata = self.collect()
        self.assertEqual(MODULE.validate(metadata), [])
        self.assertIsNone(metadata["ci"])
        self.assertFalse(metadata["repository"]["worktreeDirty"])
        self.assertEqual(metadata["artifacts"][0]["sha256"], hashlib.sha256(self.artifact.read_bytes()).hexdigest())
        self.assertEqual(len(metadata["configurationFiles"]), 3)

    def test_hosted_metadata_binds_run_artifact_and_checkout_without_secrets(self):
        metadata = self.collect(self.github)
        self.assertEqual(MODULE.validate(metadata), [])
        self.assertEqual(metadata["ci"]["runUrl"], "https://github.com/example/slskr/actions/runs/123")
        self.assertEqual(metadata["ci"]["artifactName"], "ci-reproducibility-rust")
        self.assertNotIn("not-for-metadata", str(metadata))

    def test_ci_local_checks_do_not_invent_a_retained_artifact_name(self):
        metadata = self.collect(self.github, artifact_name=None)
        self.assertEqual(MODULE.validate(metadata), [])
        self.assertNotIn("artifactName", metadata["ci"])

    def test_dirty_worktree_is_reported(self):
        metadata = self.collect(status=" M changed.rs\n?? other.rs")
        self.assertTrue(metadata["repository"]["worktreeDirty"])
        self.assertEqual(metadata["repository"]["worktreeStatusEntries"], 2)

    def test_unknown_worktree_state_is_not_reported_as_clean(self):
        metadata = self.collect(status=None)
        self.assertIsNone(metadata["repository"]["worktreeDirty"])
        self.assertTrue(MODULE.validate(metadata))

    def test_checkout_and_hosted_source_must_match(self):
        metadata = self.collect({**self.github, "GITHUB_SHA": "b" * 40})
        self.assertIn("ci.sourceCommit differs from the checked-out source", MODULE.validate(metadata))

    def test_missing_required_lock_or_config_is_rejected(self):
        for field in ("lockFiles", "configurationFiles"):
            metadata = self.collect()
            metadata[field].pop()
            self.assertTrue(MODULE.validate(metadata))

    def test_missing_artifact_and_invalid_digest_are_rejected(self):
        for changes in ({"present": False}, {"sha256": "bad"}, {"sha256": int("1" * 64)}, {"bytes": True}):
            metadata = self.collect()
            metadata["artifacts"][0].update(changes)
            self.assertTrue(MODULE.validate(metadata))

    def test_duplicate_file_paths_are_rejected(self):
        metadata = self.collect()
        metadata["lockFiles"].append(copy.deepcopy(metadata["lockFiles"][0]))
        self.assertTrue(MODULE.validate(metadata))

    def test_malformed_ci_context_is_rejected(self):
        for key, value in (("runId", "0"), ("runAttempt", "bad"), ("job", ""),
                           ("artifactName", None), ("repository", "missing-owner"),
                           ("serverUrl", "https://[invalid"), ("serverUrl", "http://github.com"),
                           ("serverUrl", "https://user:password@github.com"),
                           ("runUrl", "https://other.example/run/123")):
            with self.subTest(key=key, value=value):
                metadata = self.collect(self.github)
                metadata["ci"][key] = value
                self.assertTrue(MODULE.validate(metadata))

    def test_invalid_tool_timestamp_schema_and_repository_are_rejected(self):
        for key, value in (("generatedAt", "invalidZ"), ("schemaVersion", True), ("repository", [])):
            metadata = self.collect()
            metadata[key] = value
            self.assertTrue(MODULE.validate(metadata))
        metadata = self.collect()
        metadata["toolchain"]["cargo"] = " "
        self.assertTrue(MODULE.validate(metadata))

    def test_legacy_schema_one_metadata_remains_readable(self):
        metadata = self.collect()
        metadata["schemaVersion"] = 1
        del metadata["configurationFiles"]
        del metadata["environment"]
        self.assertEqual(MODULE.validate(metadata), [])


if __name__ == "__main__":
    unittest.main()
