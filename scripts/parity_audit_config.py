"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

from typing import Any


def config_entries(report: dict[str, Any]) -> list[dict[str, Any]]:
    status_map = {"implemented": "complete", "partial": "partial", "missing": "missing"}
    return [
        {
            "id": f"config:{row['path']}",
            "workstream": "configuration",
            "featureFamily": row["path"].split(".", 1)[0],
            "targets": row["targets"],
            "surface": "configuration-leaf",
            "subject": row["path"],
            "status": status_map[row["overall"]],
            "coverage": {
                "yaml": row["yaml"],
                "environment": row["environment"],
                "commandLine": row["commandLine"],
                "runtime": row["runtime"],
                "lifecycleValidationDifferential": (
                    "complete" if row["overall"] == "implemented" else "open"
                ),
            },
            "evidence": row["runtimeEvidence"],
        }
        for row in report["comparison"]["leafStatus"]
    ]
