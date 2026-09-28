#!/usr/bin/env python3
"""Build the frozen, externally observable parity work manifest.

The manifest deliberately distinguishes inventory/presence from behavioral
proof. A route or WebUI call that exists but lacks its complete differential
matrix remains ``needs-proof``.
"""

from __future__ import annotations

import argparse
import collections
import csv
import json
import re
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any
from urllib.parse import unquote

from parity_protocol_inventory import protocol_entries, protocol_units
from parity_not_applicable import (
    FILE_LIFECYCLE_CASES,
    PERSISTENCE_CASES,
    file_lifecycle_not_applicable_cases,
    operator_not_applicable_cases,
    persistence_not_applicable_cases,
    security_not_applicable_cases,
)
from parity_audit_api import (
    api_entries,
    controller_api_ledger,
    security_authorization_ledger,
)

from parity_audit_completeness import (
    strict_universal_failures,
    summarize,
)

from parity_audit_config import (
    config_entries,
)

from parity_audit_files import (
    file_lifecycle_entries,
    file_lifecycle_ledger,
    file_write_domains,
)

from parity_audit_frozen_transport import (
    frozen_slskdn_expected_failure_checks,
    frozen_slskdn_transport_not_applicable_contracts,
    validate_live_interop_mapping_contracts,
    validate_universal_transport_scope_contracts,
)

from parity_audit_live_catalog import (
    live_interop_features,
    live_interop_not_applicable_reason,
    validate_live_interop_scope_contracts,
)

from parity_audit_live_evidence import (
    live_interop_entries,
    live_interop_ledger,
)

from parity_audit_operator import (
    operator_entries,
    operator_families,
    operator_packaging_ledger,
)

from parity_audit_persistence import (
    database_domains,
    persistence_entries,
    persistence_lifecycle_ledger,
)

from parity_audit_process import (
    bounded_slskr_test_command,
    feature_family,
    fresh_json_evidence_paths,
    guarded_process_command,
    run_json,
    run_logged,
)

from parity_audit_protocol import (
    protocol_behaviors_ledger,
)

from parity_audit_security import (
    security_component_entries,
    security_components,
    security_control_ledger,
)

from parity_audit_shared import (
    CONTROLLER_API_DIFFERENTIAL_TEST_PREFIX,
    CONTROLLER_API_TEST_FEATURES,
    EXPECTED,
    FILE_LIFECYCLE_DIFFERENTIAL_TEST_PREFIX,
    LIVE_INTEROP_EXPECTED_FAILURE_DETAIL_TOKENS,
    LIVE_INTEROP_LOCAL_CONTROLLER_FEATURES,
    LIVE_INTEROP_PROOF_REQUIREMENTS,
    LIVE_INTEROP_REQUIRED_DETAIL_TOKENS,
    PERSISTENCE_DIFFERENTIAL_TEST_PREFIX,
    PROTOCOL_DIFFERENTIAL_TEST_PREFIX,
    REACT_WEB_UI_CASE_COUNT,
    REACT_WEB_UI_ROUTE_COUNT,
    SECURITY_AUTHORIZATION_TEST,
    SECURITY_CONTROL_CASES,
    SECURITY_CONTROL_DIFFERENTIAL_TEST_PREFIX,
    UNIVERSAL_BIDIRECTIONAL_TRANSPORTS,
    UNIVERSAL_BIDIRECTIONAL_TRANSPORT_TARGETS,
    UNIVERSAL_LIFECYCLE_CHECK,
    UNIVERSAL_LIFECYCLE_SCENARIOS,
    UNIVERSAL_TRANSPORT_LIFECYCLE_DETAIL_TOKENS,
    UNIVERSAL_TRANSPORT_LIFECYCLE_REQUIREMENTS,
    UNIVERSAL_TRANSPORT_TARGETS,
    UNIVERSAL_UI_SCENARIOS,
    UNIVERSAL_UI_WORKFLOWS,
    UNMATERIALIZED_WORKSTREAMS,
)

from parity_audit_transport import (
    transport_capability_evidence_failures,
    universal_transport_failures,
)

from parity_audit_ui_evidence import (
    universal_target_ui_comparison_failures,
    universal_ui_scenario_failures,
)

from parity_audit_webui import (
    webui_entries,
    webui_workflow_ledger,
)



def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--slskd-root", type=Path, required=True)
    parser.add_argument("--slskdn-root", type=Path, required=True)
    parser.add_argument("--slskr-root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--check-frozen", action="store_true")
    parser.add_argument("--require-complete", action="store_true")
    parser.add_argument(
        "--live-interop-evidence",
        type=Path,
        action="append",
        help=(
            "Opt in to one or more explicit all-green credentialed live-interop "
            "TSVs. Only exact mapped feature/direction cases are promoted."
        ),
    )
    parser.add_argument(
        "--operator-evidence",
        type=Path,
        help=(
            "Opt in to explicit operator-packaging artifact evidence. Only "
            "exact target/family/case rows with pass=true are promoted."
        ),
    )
    parser.add_argument(
        "--transport-evidence",
        type=Path,
        help=(
            "Explicit fresh live JSON for --strict-universal. It must prove "
            "every supported transport in both directions and the full "
            "lifecycle matrix."
        ),
    )
    parser.add_argument(
        "--transport-capability-evidence",
        type=Path,
        help=(
            "Fresh live source-bound capability JSON for target transport "
            "directions that are explicitly not applicable."
        ),
    )
    parser.add_argument(
        "--skip-security-differential",
        action="store_true",
        help=(
                        "Skip running the exhaustive security-authorization bounded "
                        "differential runner (crates/slskr linked proof slice). All "
            "security-authorization cases fall back to needs-proof. Use "
            "only for fast, evidence-incomplete dry runs."
        ),
    )
    parser.add_argument(
        "--skip-security-control-differential",
        action="store_true",
        help=(
            "Skip the explicit security-controls differential tests. Security "
            "component cases fall back to needs-proof. Use only for fast, "
            "evidence-incomplete dry runs."
        ),
    )
    parser.add_argument(
        "--skip-controller-api-differential",
        action="store_true",
        help=(
            "Skip running the controller API bounded differential runner "
            "(crates/slskr). All controller-api cases fall back to "
            "needs-proof. Use only for fast, evidence-incomplete dry runs."
        ),
    )
    parser.add_argument(
        "--skip-persistence-differential",
        action="store_true",
        help=(
            "Skip running the persistence bounded differential runner "
            "(crates/slskr). All persistence-lifecycle cases fall "
            "back to needs-proof. Use only for fast, evidence-incomplete "
            "dry runs."
        ),
    )
    parser.add_argument(
        "--skip-file-differential",
        action="store_true",
        help=(
            "Skip running the file-lifecycle bounded differential runner. "
            "File-lifecycle cases fall back to needs-proof. Use only for "
            "fast, evidence-incomplete dry runs."
        ),
    )
    parser.add_argument(
        "--skip-protocol-differential",
        action="store_true",
        help=(
            "Skip running the protocol bounded differential runner plus "
            "the slskr-protocol/slskr-client Cargo slices. All "
            "protocol-behaviors cases fall back to "
            "needs-proof. Use only for fast, "
            "evidence-incomplete dry runs."
        ),
    )
    parser.add_argument(
        "--reuse-evidence",
        action="store_true",
        help=(
            "Reuse retained passing differential/WebUI evidence from "
            "/tmp/slskr-parity-evidence and target/react-webui-audit without "
            "starting Cargo or browser proof processes."
        ),
    )
    parser.add_argument(
        "--rust-ui-evidence",
        type=Path,
        help=(
            "Explicit Rust UI audit JSON for --strict-universal. It must be a "
            "fresh live-backend audit covering all 15 routes at both viewports."
        ),
    )
    parser.add_argument(
        "--react-ui-evidence",
        type=Path,
        help=(
            "Explicit React UI audit JSON for --strict-universal. It must be a "
            f"fresh live-backend audit covering all {REACT_WEB_UI_ROUTE_COUNT} routes at both viewports."
        ),
    )
    parser.add_argument(
        "--react-ui-scenario-evidence",
        type=Path,
        action="append",
        help=(
            "Additional fresh live React UI scenario JSON artifacts. Together "
            "with --react-ui-evidence they must cover every required state."
        ),
    )
    parser.add_argument(
        "--rust-ui-scenario-evidence",
        type=Path,
        action="append",
        help=(
            "Additional fresh live Rust UI scenario JSON artifacts. Together "
            "with --rust-ui-evidence they must cover every required state."
        ),
    )
    parser.add_argument(
        "--target-ui-comparison-evidence",
        type=Path,
        help=(
            "Fresh live side-by-side workflow/action/response comparison JSON "
            "for both frozen target profiles."
        ),
    )
    parser.add_argument(
        "--strict-universal",
        action="store_true",
        help=(
            "Require the universal drop-in replacement contract: fresh evidence, "
            "all live interop directions, operator evidence, and live-backend React/Rust UI proof."
        ),
    )
    args = parser.parse_args()

    if args.strict_universal and args.reuse_evidence:
        parser.error("--strict-universal cannot be combined with --reuse-evidence")

    root = args.slskr_root.resolve()
    slskd_root = args.slskd_root.resolve()
    slskdn_root = args.slskdn_root.resolve()
    validate_live_interop_mapping_contracts(root)
    validate_live_interop_scope_contracts(slskd_root, slskdn_root)
    transport_target_requirements = validate_universal_transport_scope_contracts(
        slskd_root,
        slskdn_root,
    )
    transport_capability_contracts = frozen_slskdn_transport_not_applicable_contracts(
        slskdn_root
    )
    expected_failure_checks = frozen_slskdn_expected_failure_checks(slskdn_root)
    config_command = [
        sys.executable,
        "scripts/audit-upstream-config-surface.py",
        "--slskd-root",
        str(slskd_root),
        "--slskdn-root",
        str(slskdn_root),
        "--slskr-root",
        str(root),
        "--json",
    ]
    if args.check_frozen:
        config_command.append("--check-frozen")

    config = run_json(config_command, root)
    slskd_api = run_json(
        ["node", "scripts/audit-slskdn-controller-routes.mjs", "--slskdn-root", str(slskd_root), "--json"],
        root,
    )
    slskdn_api = run_json(
        ["node", "scripts/audit-slskdn-controller-routes.mjs", "--slskdn-root", str(slskdn_root), "--json"],
        root,
    )
    security_ledger = (
        None
        if args.skip_security_differential
        else security_authorization_ledger(root, args.reuse_evidence)
    )
    security_control_ledger_rows = (
        None
        if args.skip_security_control_differential
        else security_control_ledger(root, args.reuse_evidence)
    )
    controller_ledger = (
        None
        if args.skip_controller_api_differential
        else controller_api_ledger(
            root, slskd_root, slskdn_root, args.reuse_evidence
        )
    )
    persistence_ledger = (
        None
        if args.skip_persistence_differential
        else persistence_lifecycle_ledger(root, args.reuse_evidence)
    )
    file_ledger = (
        None
        if args.skip_file_differential
        else file_lifecycle_ledger(root, args.reuse_evidence)
    )
    protocol_ledger = (
        None
        if args.skip_protocol_differential
        else protocol_behaviors_ledger(root, args.reuse_evidence)
    )
    live_ledger = (
        None
        if args.live_interop_evidence is None
        else live_interop_ledger(
            args.live_interop_evidence,
            expected_failure_checks,
        )
    )
    operator_ledger = (
        None
        if args.operator_evidence is None
        else operator_packaging_ledger(args.operator_evidence)
    )
    webui = run_json(
        [
            "node",
            "scripts/audit-upstream-webui-endpoints.mjs",
            "--slskd-root",
            str(slskd_root),
            "--slskdn-root",
            str(slskdn_root),
            "--slskr-web-root",
            str(root),
            "--json",
        ],
        root,
    )
    webui_workflow_evidence = webui_workflow_ledger(
        root, webui, args.reuse_evidence
    )
    slskd_database_domains = database_domains(slskd_root)
    slskdn_database_domains = database_domains(slskdn_root)
    slskd_file_write_domains = file_write_domains(slskd_root)
    slskdn_file_write_domains = file_write_domains(slskdn_root)
    slskd_security_components = security_components(slskd_root)
    slskdn_security_components = security_components(slskdn_root)
    slskd_operator_families = operator_families(slskd_root)
    slskdn_operator_families = operator_families(slskdn_root)
    # slskd 10.0.2 identifies Soulseek.NET commit
    # 94fba7d4056796af067e6d7b2a8628099723cd26 in its NuGet metadata. Its
    # MessageCode.cs is byte-identical to the frozen vendored runtime copy.
    slskd_protocol_units = protocol_units(slskdn_root, include_slskdn_extensions=False)
    slskdn_protocol_units = protocol_units(slskdn_root, include_slskdn_extensions=True)
    interop_features = live_interop_features()

    actual = {
        "config": config["comparison"]["unionCount"],
        "slskd-api": len(slskd_api),
        "slskdn-api": len(slskdn_api),
        "webui-call-union": webui["comparison"]["targetUnionCount"],
        "slskd-database-domains": len(slskd_database_domains),
        "slskdn-database-domains": len(slskdn_database_domains),
        "slskd-file-writer-domains": len(slskd_file_write_domains),
        "slskdn-file-writer-domains": len(slskdn_file_write_domains),
        "slskd-security-components": len(slskd_security_components),
        "slskdn-security-components": len(slskdn_security_components),
        "slskd-operator-families": len(slskd_operator_families),
        "slskdn-operator-families": len(slskdn_operator_families),
        "slskd-protocol-units": len(slskd_protocol_units),
        "slskdn-protocol-units": len(slskdn_protocol_units),
        "live-interop-target-features": len(interop_features),
    }
    if args.check_frozen and actual != EXPECTED:
        raise SystemExit(f"frozen parity inventory changed: expected {EXPECTED!r}, got {actual!r}")

    entries = [
        *config_entries(config),
        *api_entries("slskd", slskd_api, security_ledger, controller_ledger),
        *api_entries("slskdn", slskdn_api, security_ledger, controller_ledger),
        *webui_entries(webui, webui_workflow_evidence),
        *persistence_entries(
            "slskd", slskd_database_domains, persistence_ledger, slskd_root
        ),
        *persistence_entries(
            "slskdn", slskdn_database_domains, persistence_ledger, slskdn_root
        ),
        *file_lifecycle_entries(
            "slskd", slskd_file_write_domains, file_ledger, slskd_root
        ),
        *file_lifecycle_entries(
            "slskdn", slskdn_file_write_domains, file_ledger, slskdn_root
        ),
        *security_component_entries(
            "slskd", slskd_security_components, security_control_ledger_rows, slskd_root
        ),
        *security_component_entries(
            "slskdn", slskdn_security_components, security_control_ledger_rows, slskdn_root
        ),
        *operator_entries(
            "slskd", slskd_operator_families, operator_ledger, slskd_root
        ),
        *operator_entries(
            "slskdn", slskdn_operator_families, operator_ledger, slskdn_root
        ),
        *protocol_entries("slskd", slskd_protocol_units, protocol_ledger, slskdn_root),
        *protocol_entries("slskdn", slskdn_protocol_units, protocol_ledger, slskdn_root),
        *live_interop_entries(
            interop_features,
            live_ledger,
            args.live_interop_evidence,
            expected_failure_checks,
        ),
    ]
    # A live interop row can be structurally not-applicable when the frozen
    # target owns the behavior locally (controller, protocol, persistence, or
    # security) and the bounded runner has no meaningful peer transaction for
    # that dimension.  That classification is only usable for strict
    # certification after this same invocation has produced a complete fresh
    # typed ledger for every other workstream.  Reused artifacts intentionally
    # never receive this promotion.
    typed_differential_proof = (
        not args.reuse_evidence
        and all(
            entry["status"] == "complete"
            for entry in entries
            if entry["workstream"] != "live-interop"
        )
    )
    for entry in entries:
        if entry["workstream"] != "live-interop":
            continue
        coverage = entry.get("coverage", {})
        if (
            coverage.get("liveBehavioralProof") == "not-applicable"
            and entry["status"] == "complete"
        ):
            coverage["typedDifferentialProof"] = (
                "complete" if typed_differential_proof else "open"
            )
            if typed_differential_proof:
                entry["proofMode"] = "fresh-typed-differential"
    manifest = {
        "schemaVersion": 1,
        "goal": "frozen externally observable 1:1 parity and bidirectional interoperability",
        "certification": {
            "mode": "universal-replacement" if args.strict_universal else "frozen-ledger",
            "evidenceMode": "reused" if args.reuse_evidence else "fresh",
            "liveInteropEvidence": [str(path) for path in args.live_interop_evidence or []],
            "operatorEvidence": str(args.operator_evidence) if args.operator_evidence else None,
            "transportEvidence": str(args.transport_evidence) if args.transport_evidence else None,
            "transportCapabilityEvidence": (
                str(args.transport_capability_evidence)
                if args.transport_capability_evidence
                else None
            ),
            "reactUiEvidence": str(args.react_ui_evidence) if args.react_ui_evidence else None,
            "rustUiEvidence": str(args.rust_ui_evidence) if args.rust_ui_evidence else None,
            "targetUiComparisonEvidence": (
                str(args.target_ui_comparison_evidence)
                if args.target_ui_comparison_evidence
                else None
            ),
        },
        "frozenTargets": {
            "slskd": config["slskd"]["revision"],
            "slskdn": config["slskdn"]["revision"],
            "slskNetRuntime": "af73ff3f84fda7ba890bb5aea3adf712e5400cf6",
        },
        "summary": summarize(entries),
        "unmaterializedWorkstreams": UNMATERIALIZED_WORKSTREAMS,
        "entries": entries,
    }

    if args.strict_universal:
        strict_failures = strict_universal_failures(
            entries,
            reuse_evidence=args.reuse_evidence,
            live_interop_evidence=args.live_interop_evidence,
            operator_evidence=args.operator_evidence,
            transport_evidence=args.transport_evidence,
            react_ui_evidence=args.react_ui_evidence,
            rust_ui_evidence=args.rust_ui_evidence,
            react_ui_scenario_evidence=args.react_ui_scenario_evidence,
            rust_ui_scenario_evidence=args.rust_ui_scenario_evidence,
            target_ui_comparison_evidence=args.target_ui_comparison_evidence,
            transport_target_requirements=transport_target_requirements,
            transport_capability_evidence=args.transport_capability_evidence,
            transport_capability_contracts=transport_capability_contracts,
        )
        if strict_failures:
            print(
                "universal replacement gate failed:",
                file=sys.stderr,
            )
            for failure in strict_failures:
                print(f"- {failure}", file=sys.stderr)
            raise SystemExit(1)
        manifest["summary"]["goalAchieved"] = True

    if args.json:
        print(json.dumps(manifest, indent=2))
    else:
        summary = manifest["summary"]
        print(
            "parity manifest: "
            f"materialized={summary['materializedEntryCount']} "
            f"complete={summary['statusCounts']['complete']} "
            f"partial={summary['statusCounts']['partial']} "
            f"missing={summary['statusCounts']['missing']} "
            f"needs-proof={summary['statusCounts']['needs-proof']} "
            f"denominator-missing={summary['unmaterializedWorkstreamCount']} "
            f"proof-case-closure={summary['proofCaseClosurePercentage']:.2f}%"
        )
        for name, counts in summary["workstreams"].items():
            print(
                f"  {name}: total={counts['total']} complete={counts['complete']} "
                f"partial={counts['partial']} missing={counts['missing']} "
                f"needs-proof={counts['needs-proof']}"
            )

    if args.require_complete:
        incomplete = [entry for entry in entries if entry["status"] != "complete"]
        if incomplete or UNMATERIALIZED_WORKSTREAMS:
            print(
                "literal parity check failed: "
                f"{len(incomplete)} materialized entries are incomplete and "
                f"{len(UNMATERIALIZED_WORKSTREAMS)} workstream denominators are missing",
                file=sys.stderr,
            )
            raise SystemExit(1)


if __name__ == "__main__":
    main()
