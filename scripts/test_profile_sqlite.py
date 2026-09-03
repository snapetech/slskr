#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("profile-sqlite.py")
SPEC = importlib.util.spec_from_file_location("profile_sqlite", SCRIPT)
assert SPEC and SPEC.loader
profile_sqlite = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = profile_sqlite
SPEC.loader.exec_module(profile_sqlite)


class ProfileSqliteTests(unittest.TestCase):
    def test_query_parser_rejects_mutation_and_multiple_statements(self) -> None:
        with self.assertRaises(ValueError):
            profile_sqlite.parse_query("write=UPDATE messages SET read = 1")
        with self.assertRaises(ValueError):
            profile_sqlite.parse_query("read=SELECT 1; DELETE FROM messages")

    def test_profile_records_plan_and_real_result_shape(self) -> None:
        with tempfile.TemporaryDirectory(prefix="slskr-sqlite-profile-") as directory:
            database = Path(directory) / "profile.db"
            connection = sqlite3.connect(database)
            connection.execute("CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT)")
            connection.executemany(
                "INSERT INTO records (value) VALUES (?)",
                [(f"value-{index}",) for index in range(20)],
            )
            connection.execute("CREATE INDEX idx_records_value ON records(value)")
            connection.commit()
            connection.close()

            artifact = profile_sqlite.profile_database(
                database,
                [
                    profile_sqlite.QueryCase(
                        "by_value",
                        "SELECT id, value FROM records WHERE value = 'value-3'",
                    )
                ],
                warmup_iterations=1,
                measured_iterations=3,
            )

            case = artifact["cases"]["by_value"]
            self.assertEqual(artifact["benchmark"], "slskr-sqlite")
            self.assertEqual(case["rows"], {"first": 1, "minimum": 1, "maximum": 1})
            self.assertEqual(len(case["plan"]), 1)
            self.assertIn("idx_records_value", case["plan"][0]["detail"])
            self.assertEqual(set(case["latencyMs"]), {"minimum", "median", "p95", "maximum"})

    def test_cli_writes_json_artifact(self) -> None:
        with tempfile.TemporaryDirectory(prefix="slskr-sqlite-profile-cli-") as directory:
            directory = Path(directory)
            database = directory / "profile.db"
            output = directory / "artifact.json"
            connection = sqlite3.connect(database)
            connection.execute("CREATE TABLE records (id INTEGER PRIMARY KEY)")
            connection.commit()
            connection.close()

            subprocess.check_call(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--database",
                    str(database),
                    "--query",
                    "records=SELECT id FROM records",
                    "--warmup",
                    "0",
                    "--iterations",
                    "1",
                    "--output",
                    str(output),
                ]
            )
            artifact = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(artifact["cases"]["records"]["rows"]["first"], 0)


if __name__ == "__main__":
    unittest.main()
