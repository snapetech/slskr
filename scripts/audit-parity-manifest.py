#!/usr/bin/env python3
"""Build the frozen, externally observable parity work manifest.

The manifest deliberately distinguishes inventory/presence from behavioral
proof. A route or WebUI call that exists but lacks its complete differential
matrix remains ``needs-proof``.
"""

from __future__ import annotations

import argparse
import collections
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


EXPECTED = {
    "config": 436,
    "slskd-api": 91,
    "slskdn-api": 678,
    "webui-call-union": 417,
    "slskd-database-domains": 11,
    "slskdn-database-domains": 61,
    "slskd-file-writer-domains": 9,
    "slskdn-file-writer-domains": 52,
    "slskd-security-components": 12,
    "slskdn-security-components": 121,
    "slskd-operator-families": 3,
    "slskdn-operator-families": 37,
    "slskd-protocol-units": 123,
    "slskdn-protocol-units": 170,
    "live-interop-target-features": 62,
}

UNMATERIALIZED_WORKSTREAMS: list[dict[str, str]] = []


def run_json(command: list[str], cwd: Path) -> Any:
    completed = subprocess.run(
        command,
        cwd=cwd,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return json.loads(completed.stdout)


def feature_family(subject: str) -> str:
    value = subject.split(" ", 1)[-1].strip("/")
    parts = [part for part in value.split("/") if part]
    while parts and (parts[0] == "api" or re.fullmatch(r"v(?:\d+|\{version\})", parts[0])):
        parts.pop(0)
    return parts[0].replace(":var", "parameter") if parts else "root"


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


def api_entries(target: str, rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    entries = []
    for row in rows:
        subject = f"{row['method']} {row['route']}"
        cases = [
            "route-presence",
            "nominal-status-headers-body",
            "malformed-path-query-or-body",
            "missing-empty-or-conflict-state",
            "runtime-failure-and-timeout",
        ]
        if row["method"] == "GET":
            cases.append("populated-dynamic-state")
        else:
            cases.extend(
                [
                    "mutation-side-effects-and-readback",
                    "restart-persistence-or-reset",
                    "concurrency-and-idempotency",
                ]
            )
        for case in cases:
            entries.append(
                {
                    "id": f"api:{target}:{row['method']}:{row['route']}:{case}",
                    "workstream": f"{target}-controller-api",
                    "featureFamily": feature_family(row["route"]),
                    "targets": [target],
                    "surface": "controller-route-case",
                    "subject": subject,
                    "case": case,
                    "status": "needs-proof",
                    "coverage": {
                        "routeInventory": "complete",
                        "behavioralDifferential": "open",
                    },
                    "evidence": row["controller"],
                }
            )

        for profile in (
            "anonymous",
            "basic-readonly",
            "basic-readwrite",
            "basic-administrator",
            "bearer-readonly",
            "bearer-readwrite",
            "bearer-administrator",
            "invalid-or-expired-credential",
            "missing-required-scope",
            "wrong-authentication-scheme",
        ):
            entries.append(
                {
                    "id": f"security:{target}:{row['method']}:{row['route']}:{profile}",
                    "workstream": "security-authorization",
                    "featureFamily": feature_family(row["route"]),
                    "targets": [target],
                    "surface": "controller-authorization-case",
                    "subject": subject,
                    "case": profile,
                    "status": "needs-proof",
                    "coverage": {
                        "authorizationMetadata": "complete",
                        "liveHttpDifferential": "open",
                        "expected": row["auth"],
                    },
                    "evidence": row["controller"],
                }
            )
    return entries


def webui_entries(report: dict[str, Any]) -> list[dict[str, Any]]:
    slskd = set(report["slskd"]["endpoints"])
    slskdn = set(report["slskdn"]["endpoints"])
    slskr = set(report["slskr"]["endpoints"])
    union = sorted(slskd | slskdn)
    entries = []
    for subject in union:
        targets = [
            target
            for target, values in (("slskd", slskd), ("slskdn", slskdn))
            if subject in values
        ]
        for case in (
            "call-presence",
            "rendered-success",
            "rendered-loading-and-empty",
            "rendered-validation-and-server-error",
            "authorization-reconnect-and-restart",
        ):
            call_present = subject in slskr
            entries.append(
                {
                    "id": f"webui:{subject}:{case}",
                    "workstream": "webui-workflows",
                    "featureFamily": feature_family(subject),
                    "targets": targets,
                    "surface": "webui-workflow-case",
                    "subject": subject,
                    "case": case,
                    "status": (
                        "complete"
                        if case == "call-presence" and call_present
                        else "missing"
                        if not call_present
                        else "needs-proof"
                    ),
                    "coverage": {
                        "callPresence": "complete" if call_present else "missing",
                        "renderedWorkflowDifferential": (
                            "not-applicable" if case == "call-presence" else "open"
                        ),
                    },
                    "evidence": report["slskr"]["sources"].get(subject, []),
                }
            )
    return entries


def database_domains(root: Path) -> dict[str, list[str]]:
    source_root = root / "src/slskd"
    domains: dict[str, set[str]] = collections.defaultdict(set)
    internal_tables = {
        "__HashDbMigrations",
        "Messages_fts",
        "filenames_config",
        "filenames_content",
        "filenames_data",
        "filenames_docsize",
        "filenames_idx",
        "version",
    }
    for source_path in sorted(source_root.rglob("*.cs")):
        source = source_path.read_text(encoding="utf-8-sig", errors="ignore")
        source = re.sub(r"/\*.*?\*/", "", source, flags=re.DOTALL)
        source = re.sub(r"//[^\n]*", "", source)
        relative = str(source_path.relative_to(source_root))
        for _entity_type, name in re.findall(r"DbSet<([^>]+)>\s+(\w+)", source):
            if name not in internal_tables:
                domains[name].add(relative)
        for name in re.findall(
            r"CREATE\s+(?:VIRTUAL\s+)?TABLE(?:\s+IF\s+NOT\s+EXISTS)?\s+"
            r"[\[\"']?([A-Za-z_][A-Za-z0-9_]*)",
            source,
            flags=re.IGNORECASE,
        ):
            if name not in internal_tables:
                domains[name].add(relative)
        for name in re.findall(r'\.ToTable\(\s*"([^"]+)"', source):
            if name not in internal_tables:
                domains[name].add(relative)
    return {name: sorted(paths) for name, paths in sorted(domains.items())}


def persistence_entries(target: str, domains: dict[str, list[str]]) -> list[dict[str, Any]]:
    entries = []
    for domain, sources in domains.items():
        family = sources[0].split("/", 1)[0].lower() if sources else domain.lower()
        for case in (
            "schema-create-and-migrate",
            "create-and-read-roundtrip",
            "update-delete-and-readback",
            "restart-rehydration",
            "transaction-and-concurrency-atomicity",
            "corrupt-state-and-upgrade-failure",
        ):
            entries.append(
                {
                    "id": f"persistence:{target}:{domain}:{case}",
                    "workstream": "persistence-lifecycle",
                    "featureFamily": family,
                    "targets": [target],
                    "surface": "database-lifecycle-case",
                    "subject": domain,
                    "case": case,
                    "status": "needs-proof",
                    "coverage": {
                        "frozenDatabaseInventory": "complete",
                        "behavioralDifferential": "open",
                    },
                    "evidence": sources,
                }
            )
    return entries


def file_write_domains(root: Path) -> list[str]:
    source_root = root / "src/slskd"
    patterns = (
        re.compile(
            r"(?:System\.IO\.)?File\."
            r"(?:WriteAllText(?:Async)?|WriteAllBytes(?:Async)?|Move|Replace|Create)\b"
        ),
        re.compile(
            r"new\s+(?:System\.IO\.)?FileStream\([^\n]+"
            r"FileMode\.(?:Create|CreateNew|Append|OpenOrCreate)\b"
        ),
        re.compile(r"new\s+(?:System\.IO\.)?StreamWriter\b"),
        re.compile(r"\b(?:AtomicFileWriter|SecureFileWriter)\."),
    )
    return [
        str(path.relative_to(source_root))
        for path in sorted(source_root.rglob("*.cs"))
        if any(pattern.search(path.read_text(encoding="utf-8-sig", errors="ignore")) for pattern in patterns)
    ]


def file_lifecycle_entries(target: str, domains: list[str]) -> list[dict[str, Any]]:
    entries = []
    for source in domains:
        subject = source.removesuffix(".cs")
        family = source.split("/", 1)[0].lower()
        for case in (
            "path-and-default-selection",
            "nominal-bytes-and-metadata",
            "existing-missing-and-overwrite",
            "permissions-symlink-and-path-confinement",
            "partial-cancel-and-cleanup",
            "restart-reload-retention-and-corruption",
        ):
            entries.append(
                {
                    "id": f"file-lifecycle:{target}:{subject}:{case}",
                    "workstream": "persistence-lifecycle",
                    "featureFamily": family,
                    "targets": [target],
                    "surface": "file-lifecycle-case",
                    "subject": subject,
                    "case": case,
                    "status": "needs-proof",
                    "coverage": {
                        "frozenFileWriterInventory": "complete",
                        "behavioralDifferential": "open",
                    },
                    "evidence": source,
                }
            )
    return entries


def security_components(root: Path) -> list[str]:
    source_root = root / "src/slskd"
    security_name = re.compile(
        r"Security|Auth|RateLimit|Csp|Csrf|Cors|Token|Certificate|Blacklist|"
        r"Blocklist|Ban|Permission|Policy",
        flags=re.IGNORECASE,
    )
    return [
        str(path.relative_to(source_root))
        for path in sorted(source_root.rglob("*.cs"))
        if security_name.search(str(path.relative_to(source_root)))
    ]


def security_component_entries(target: str, components: list[str]) -> list[dict[str, Any]]:
    entries = []
    for source in components:
        subject = source.removesuffix(".cs")
        family = source.split("/", 1)[0].lower()
        for case in (
            "activation-default-and-profile",
            "accepted-nominal-input",
            "rejected-malicious-and-boundary-input",
            "quota-time-lockout-and-concurrency",
            "secret-logging-and-privacy-output",
            "restart-rotation-and-recovery",
        ):
            entries.append(
                {
                    "id": f"security-component:{target}:{subject}:{case}",
                    "workstream": "security-controls",
                    "featureFamily": family,
                    "targets": [target],
                    "surface": "security-control-case",
                    "subject": subject,
                    "case": case,
                    "status": "needs-proof",
                    "coverage": {
                        "frozenSecurityComponentInventory": "complete",
                        "behavioralDifferentialOrNotApplicableProof": "open",
                    },
                    "evidence": source,
                }
            )
    return entries


def operator_families(root: Path) -> dict[str, list[str]]:
    families: dict[str, set[str]] = collections.defaultdict(set)

    dockerfile = root / "Dockerfile"
    if dockerfile.is_file():
        families["container-root"].add("Dockerfile")

    workflow_root = root / ".github/workflows"
    if workflow_root.is_dir():
        for path in sorted(workflow_root.glob("*.y*ml")):
            families[f"github-workflow-{path.stem}"].add(str(path.relative_to(root)))

    packaging_root = root / "packaging"
    if packaging_root.is_dir():
        for path in sorted(packaging_root.rglob("*")):
            if path.is_file():
                relative = path.relative_to(root)
                families[f"packaging-{relative.parts[1]}"].add(str(relative))

    systemd_root = root / "etc/systemd"
    if systemd_root.is_dir():
        for child in sorted(systemd_root.rglob("*")):
            if child.is_file():
                families["systemd-hardened"].add(str(child.relative_to(root)))

    nix_file = root / "flake.nix"
    if nix_file.is_file():
        families["nix-root"].add("flake.nix")

    vpn_root = root / "src/slskdN.VpnAgent"
    if vpn_root.is_dir():
        for child in sorted(vpn_root.rglob("*")):
            if child.is_file() and (
                child.name == "install.sh" or "systemd" in child.relative_to(vpn_root).parts
            ):
                families["vpn-agent"].add(str(child.relative_to(root)))

    return {family: sorted(paths) for family, paths in sorted(families.items())}


def operator_entries(target: str, families: dict[str, list[str]]) -> list[dict[str, Any]]:
    entries = []
    for family, sources in families.items():
        for case in (
            "build-render-and-artifact-contents",
            "fresh-install-and-upgrade",
            "start-stop-signal-and-restart",
            "configuration-user-permissions-and-secrets",
            "network-ports-storage-and-health",
            "failure-rollback-uninstall-and-logs",
        ):
            entries.append(
                {
                    "id": f"operator:{target}:{family}:{case}",
                    "workstream": "operator-packaging",
                    "featureFamily": family,
                    "targets": [target],
                    "surface": "operator-family-case",
                    "subject": family,
                    "case": case,
                    "status": "needs-proof",
                    "coverage": {
                        "frozenOperatorArtifactInventory": "complete",
                        "behavioralDifferentialOrNotApplicableProof": "open",
                    },
                    "evidence": sources,
                }
            )
    return entries


def enum_values(path: Path, enum_name: str) -> list[tuple[str, int]]:
    source = path.read_text(encoding="utf-8-sig")
    match = re.search(
        rf"(?:public|internal)\s+enum\s+{re.escape(enum_name)}\b[^{{]*{{",
        source,
        flags=re.DOTALL,
    )
    if match is None:
        raise ValueError(f"enum {enum_name} not found in {path}")
    start = match.end()
    depth = 1
    cursor = start
    while depth:
        depth += (source[cursor] == "{") - (source[cursor] == "}")
        cursor += 1
    body = source[start : cursor - 1]
    return [
        (name, int(value, 0))
        for name, value in re.findall(
            r"^\s*(\w+)\s*=\s*(0x[0-9A-Fa-f]+|[0-9]+),",
            body,
            flags=re.MULTILINE,
        )
        if name != "Unknown"
    ]


def static_string_constants(path: Path, class_name: str) -> list[tuple[str, str]]:
    source = path.read_text(encoding="utf-8-sig")
    match = re.search(
        rf"(?:public|internal)\s+static\s+class\s+{re.escape(class_name)}\b[^{{]*{{",
        source,
        flags=re.DOTALL,
    )
    if match is None:
        raise ValueError(f"static class {class_name} not found in {path}")
    start = match.end()
    depth = 1
    cursor = start
    while depth:
        depth += (source[cursor] == "{") - (source[cursor] == "}")
        cursor += 1
    body = source[start : cursor - 1]
    return re.findall(r'public const string\s+(\w+)\s*=\s*"([^"]+)"', body)


def protocol_units(root: Path, include_slskdn_extensions: bool) -> list[dict[str, Any]]:
    runtime_root = root / "vendor/slskNet.Runtime/src"
    message_codes = runtime_root / "Messaging/MessageCode.cs"
    units = []
    for family in ("Initialization", "Peer", "Distributed", "Server"):
        for name, value in enum_values(message_codes, family):
            units.append(
                {
                    "family": f"soulseek-{family.lower()}",
                    "name": name,
                    "value": value,
                    "source": str(message_codes.relative_to(root)),
                }
            )

    if not include_slskdn_extensions:
        return units

    enum_sources = (
        (
            "peer-capability",
            runtime_root / "PeerCapabilityMessageType.cs",
            "PeerCapabilityMessageType",
        ),
        (
            "mesh-sync",
            root / "src/slskd/Mesh/Messages/MeshMessages.cs",
            "MeshMessageType",
        ),
        (
            "virtual-soulfind-bridge",
            root / "src/slskd/VirtualSoulfind/Bridge/Protocol/SoulseekProtocolParser.cs",
            "MessageType",
        ),
    )
    for family, path, enum_name in enum_sources:
        for name, value in enum_values(path, enum_name):
            units.append(
                {
                    "family": family,
                    "name": name,
                    "value": value,
                    "source": str(path.relative_to(root)),
                }
            )

    constant_sources = (
        (
            "rendezvous-overlay",
            root / "src/slskd/DhtRendezvous/Messages/OverlayMessages.cs",
            "OverlayMessageType",
        ),
        (
            "mesh-overlay-control",
            root / "src/slskd/Mesh/Overlay/OverlayControlTypes.cs",
            "OverlayControlTypes",
        ),
    )
    for family, path, class_name in constant_sources:
        for name, value in static_string_constants(path, class_name):
            units.append(
                {
                    "family": family,
                    "name": name,
                    "value": value,
                    "source": str(path.relative_to(root)),
                }
            )

    services_root = root / "src/slskd/Mesh/ServiceFabric/Services"
    for path in sorted(services_root.glob("*.cs")):
        source = path.read_text(encoding="utf-8-sig")
        for value in re.findall(r'public string ServiceName\s*=>\s*"([^"]+)"', source):
            units.append(
                {
                    "family": "mesh-service",
                    "name": value,
                    "value": value,
                    "source": str(path.relative_to(root)),
                }
            )
    return units


def protocol_entries(target: str, units: list[dict[str, Any]]) -> list[dict[str, Any]]:
    entries = []
    for unit in units:
        subject = f"{unit['family']}:{unit['name']}:{unit['value']}"
        for case in (
            "exact-frame-and-encoding",
            "decode-dispatch-and-side-effects",
            "malformed-truncated-oversize-and-unknown",
            "timeout-cancel-reconnect-and-failure",
            "live-bidirectional-exchange",
        ):
            entries.append(
                {
                    "id": f"protocol:{target}:{subject}:{case}",
                    "workstream": "protocol-behaviors",
                    "featureFamily": unit["family"],
                    "targets": [target],
                    "surface": "protocol-unit-case",
                    "subject": subject,
                    "case": case,
                    "status": "needs-proof",
                    "coverage": {
                        "frozenProtocolInventory": "complete",
                        "behavioralDifferentialOrNotApplicableProof": "open",
                    },
                    "evidence": unit["source"],
                }
            )
    return entries


def live_interop_features() -> list[tuple[str, str]]:
    shared = (
        "server-session",
        "peer-endpoint",
        "listener-and-indirect-connect",
        "type1-obfuscation",
        "public-search",
        "room-search",
        "wishlist-search",
        "browse-share-list",
        "folder-contents",
        "download",
        "upload",
        "queue-position",
        "transfer-resume-cancel-and-retry",
        "private-message",
        "batch-private-message",
        "public-room",
        "private-room-and-ticker",
        "user-watch-status-and-stats",
        "interests-and-recommendations",
        "privileges",
        "distributed-tree",
    )
    slskdn_only = (
        "peer-capability",
        "dht-rendezvous",
        "overlay-handshake-and-keepalive",
        "mesh-sync",
        "mesh-service-dht",
        "mesh-service-pods",
        "mesh-content-and-preview",
        "private-gateway-and-vpn",
        "shadow-index",
        "hole-punch",
        "mesh-introspection",
        "collections-and-share-grants",
        "download-requests",
        "multisource-and-swarm",
        "relay",
        "solid-and-federation",
        "virtualsoulfind-v2",
        "songid",
        "streaming-and-playback",
        "source-feeds-and-discovery",
    )
    return [
        *((target, feature) for target in ("slskd", "slskdn") for feature in shared),
        *(("slskdn", feature) for feature in slskdn_only),
    ]


def live_interop_entries(features: list[tuple[str, str]]) -> list[dict[str, Any]]:
    entries = []
    for target, feature in features:
        for case in (
            "slskr-initiates-to-target",
            "target-initiates-to-slskr",
            "reconnect-retry-and-resume",
            "malformed-denied-timeout-and-cancel",
            "restart-and-persisted-state",
        ):
            entries.append(
                {
                    "id": f"live-interop:{target}:{feature}:{case}",
                    "workstream": "live-interop",
                    "featureFamily": feature,
                    "targets": [target],
                    "surface": "live-interop-case",
                    "subject": feature,
                    "case": case,
                    "status": "needs-proof",
                    "coverage": {
                        "targetFeatureInventory": "complete",
                        "liveBehavioralProof": "open",
                    },
                    "evidence": [
                        "docs/live-interop-test-matrix.md",
                        "scripts/run-live-interop-matrix.sh",
                        "scripts/run-slskdn-cross-client-interop.sh",
                    ],
                }
            )
    return entries


def summarize(entries: list[dict[str, Any]]) -> dict[str, Any]:
    by_workstream: dict[str, collections.Counter[str]] = collections.defaultdict(collections.Counter)
    totals: collections.Counter[str] = collections.Counter()
    for entry in entries:
        by_workstream[entry["workstream"]][entry["status"]] += 1
        totals[entry["status"]] += 1
    statuses = ("complete", "partial", "missing", "needs-proof")
    materialized_entry_count = len(entries)
    complete_count = totals["complete"]
    proof_case_closure_percentage = (
        round((complete_count / materialized_entry_count) * 100, 2)
        if materialized_entry_count
        else 0.0
    )
    return {
        "materializedEntryCount": materialized_entry_count,
        "statusCounts": {status: totals[status] for status in statuses},
        "workstreams": {
            name: {
                "total": sum(counts.values()),
                **{status: counts[status] for status in statuses},
            }
            for name, counts in sorted(by_workstream.items())
        },
        "unmaterializedWorkstreamCount": len(UNMATERIALIZED_WORKSTREAMS),
        # This is a literal executable-proof-case ratio, not a subjective
        # estimate of user-visible feature completeness. The manifest cases
        # intentionally have different granularity, so keep the label explicit.
        "proofCaseClosurePercentage": proof_case_closure_percentage,
        "overallPercentage": proof_case_closure_percentage,
        "percentageBasis": "complete materialized proof cases / all materialized proof cases",
        "goalAchieved": False,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--slskd-root", type=Path, required=True)
    parser.add_argument("--slskdn-root", type=Path, required=True)
    parser.add_argument("--slskr-root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--check-frozen", action="store_true")
    parser.add_argument("--require-complete", action="store_true")
    args = parser.parse_args()

    root = args.slskr_root.resolve()
    slskd_root = args.slskd_root.resolve()
    slskdn_root = args.slskdn_root.resolve()
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
        *api_entries("slskd", slskd_api),
        *api_entries("slskdn", slskdn_api),
        *webui_entries(webui),
        *persistence_entries("slskd", slskd_database_domains),
        *persistence_entries("slskdn", slskdn_database_domains),
        *file_lifecycle_entries("slskd", slskd_file_write_domains),
        *file_lifecycle_entries("slskdn", slskdn_file_write_domains),
        *security_component_entries("slskd", slskd_security_components),
        *security_component_entries("slskdn", slskdn_security_components),
        *operator_entries("slskd", slskd_operator_families),
        *operator_entries("slskdn", slskdn_operator_families),
        *protocol_entries("slskd", slskd_protocol_units),
        *protocol_entries("slskdn", slskdn_protocol_units),
        *live_interop_entries(interop_features),
    ]
    manifest = {
        "schemaVersion": 1,
        "goal": "frozen externally observable 1:1 parity and bidirectional interoperability",
        "frozenTargets": {
            "slskd": config["slskd"]["revision"],
            "slskdn": config["slskdn"]["revision"],
            "slskNetRuntime": "af73ff3f84fda7ba890bb5aea3adf712e5400cf6",
        },
        "summary": summarize(entries),
        "unmaterializedWorkstreams": UNMATERIALIZED_WORKSTREAMS,
        "entries": entries,
    }

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
