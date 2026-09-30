"""Frozen protocol inventory and per-case parity manifest entries."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any

from parity_not_applicable import protocol_not_applicable_cases


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


def protocol_entries(
    target: str,
    units: list[dict[str, Any]],
    protocol_ledger: dict[tuple[str, str, str], bool] | None = None,
    frozen_root: Path | None = None,
) -> list[dict[str, Any]]:
    entries = []
    for unit in units:
        subject = f"{unit['family']}:{unit['name']}:{unit['value']}"
        not_applicable_cases = protocol_not_applicable_cases(frozen_root, unit)
        for case in (
            "exact-frame-and-encoding",
            "decode-dispatch-and-side-effects",
            "malformed-truncated-oversize-and-unknown",
            "timeout-cancel-reconnect-and-failure",
            "live-bidirectional-exchange",
        ):
            proven = (
                protocol_ledger.get((target, subject, case))
                if protocol_ledger is not None
                else None
            )
            not_applicable_reason = not_applicable_cases.get(case)
            entries.append(
                {
                    "id": f"protocol:{target}:{subject}:{case}",
                    "workstream": "protocol-behaviors",
                    "featureFamily": unit["family"],
                    "targets": [target],
                    "surface": "protocol-unit-case",
                    "subject": subject,
                    "case": case,
                    "status": "complete" if proven or not_applicable_reason else "needs-proof",
                    "coverage": {
                        "frozenProtocolInventory": "complete",
                        "behavioralDifferentialOrNotApplicableProof": "complete"
                        if proven
                        else "not-applicable"
                        if not_applicable_reason
                        else "open",
                    },
                    **(
                        {"notApplicableReason": not_applicable_reason}
                        if not_applicable_reason
                        else {}
                    ),
                    "evidence": unit["source"],
                }
            )
    return entries
