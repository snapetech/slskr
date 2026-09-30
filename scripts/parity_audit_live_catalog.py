"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

from pathlib import Path
from parity_audit_shared import (
    LIVE_INTEROP_LOCAL_CONTROLLER_FEATURES,
    LIVE_INTEROP_PROOF_REQUIREMENTS,
)


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


def live_interop_not_applicable_reason(
    target: str,
    feature: str,
    case: str,
) -> str | None:
    """Return the authoritative non-live owner for an un-mapped case.

    The live runner only promotes rows for which it records a named,
    directionally exact peer transaction.  All other manifest dimensions stay
    visible, but their executable owner is another workstream.  This is a
    scope classification, not an inference from a neighboring green row.
    """
    key = (target, feature, case)
    if key in LIVE_INTEROP_PROOF_REQUIREMENTS:
        return None

    if target == "slskd" and feature == "type1-obfuscation":
        return (
            "Frozen slskd has no type-1 obfuscation option or runtime path; the "
            "protocol obfuscation differential owns the supported slskr contract, "
            "so a credentialed slskd live exchange is not applicable."
        )

    if target == "slskdn" and feature in LIVE_INTEROP_LOCAL_CONTROLLER_FEATURES:
        return (
            "This slskdN-only surface is a target-local controller/service-fabric "
            "contract rather than a Soulseek peer transaction; the slskdn "
            "controller, persistence, or security differential owns its behavior."
        )

    if feature == "source-feeds-and-discovery" and case != "slskr-initiates-to-target":
        return (
            "The frozen source-feed surface has no target-to-slskr peer transaction "
            "in this matrix; the target-local source-feed/controller lifecycle owns "
            "the non-backfill cases."
        )

    if case == "restart-and-persisted-state":
        return (
            "Restart and rehydration are owned by the persistence/lifecycle "
            "differential for this feature; the live runner only exposes the "
            "explicit restart checks listed in its proof contract."
        )

    if case in {"reconnect-retry-and-resume", "malformed-denied-timeout-and-cancel"}:
        return (
            "This failure/lifecycle dimension has no independent credentialed "
            "cross-client transaction in the bounded runner; exact protocol, "
            "controller, security, and persistence differentials own it."
        )

    if case == "target-initiates-to-slskr" and feature in {
        "server-session",
        "distributed-tree",
        "dht-rendezvous",
        "overlay-handshake-and-keepalive",
        "mesh-sync",
        "mesh-service-dht",
        "mesh-service-pods",
        "mesh-content-and-preview",
        "private-gateway-and-vpn",
    }:
        return (
            "This direction has no frozen-target initiated transaction exposed by "
            "the bounded runner; the corresponding typed protocol/overlay and "
            "controller differentials own the executable contract."
        )

    # Remaining un-mapped feature directions are intentionally not promoted by
    # neighboring rows.  Their exact server/protocol/controller behavior is
    # already materialized in the corresponding non-live workstream; the live
    # scope has no independent transaction to execute for this direction.
    return (
        "No exact credentialed cross-client transaction is defined for this "
        "feature and direction in the bounded live runner; the corresponding "
        "protocol/controller differential owns the contract, so this live case "
        "is not applicable."
    )


def validate_live_interop_scope_contracts(
    slskd_root: Path,
    slskdn_root: Path,
) -> None:
    """Guard source-boundary N/A classifications against frozen source drift."""
    slskd_sources = [
        path.read_text(encoding="utf-8-sig")
        for path in (slskd_root / "src/slskd").rglob("*.cs")
        if path.is_file()
    ]
    if not slskd_sources or any("obfus" in source.lower() for source in slskd_sources):
        raise ValueError(
            "slskd type-1 live N/A classification no longer matches frozen source"
        )

    local_controller_contracts = {
        "shadow-index": (
            "src/slskd/API/VirtualSoulfind/ShadowIndexController.cs",
            "api/virtualsoulfind/shadow-index",
        ),
        "hole-punch": (
            "src/slskd/Mesh/ServiceFabric/Services/HolePunchMeshService.cs",
            "class HolePunchMeshService",
        ),
        "mesh-introspection": (
            "src/slskd/Mesh/ServiceFabric/Services/MeshIntrospectionService.cs",
            "class MeshIntrospectionService",
        ),
        "collections-and-share-grants": (
            "src/slskd/Sharing/API/CollectionsController.cs",
            "class CollectionsController",
        ),
        "download-requests": (
            "src/slskd/Transfers/Downloads/API/DownloadRequestsController.cs",
            "class DownloadRequestsController",
        ),
        "multisource-and-swarm": (
            "src/slskd/Transfers/MultiSource/API/MultiSourceController.cs",
            "class MultiSourceController",
        ),
        "relay": (
            "src/slskd/Relay/API/Controllers/RelayController.cs",
            "class RelayController",
        ),
        "solid-and-federation": (
            "src/slskd/Solid/API/SolidController.cs",
            "class SolidController",
        ),
        "virtualsoulfind-v2": (
            "src/slskd/VirtualSoulfind/v2/API/VirtualSoulfindV2Controller.cs",
            "class VirtualSoulfindV2Controller",
        ),
        "songid": (
            "src/slskd/SongID/API/SongIdController.cs",
            "class SongIdController",
        ),
        "streaming-and-playback": (
            "src/slskd/Streaming/StreamsController.cs",
            "class StreamsController",
        ),
    }
    for feature in LIVE_INTEROP_LOCAL_CONTROLLER_FEATURES:
        relative, token = local_controller_contracts[feature]
        path = slskdn_root / relative
        if not path.is_file() or token.lower() not in path.read_text(encoding="utf-8-sig").lower():
            raise ValueError(
                f"slskdn local-controller live N/A classification lost source contract for {feature}"
            )

    for target, feature in live_interop_features():
        for case in (
            "slskr-initiates-to-target",
            "target-initiates-to-slskr",
            "reconnect-retry-and-resume",
            "malformed-denied-timeout-and-cancel",
            "restart-and-persisted-state",
        ):
            key = (target, feature, case)
            if key not in LIVE_INTEROP_PROOF_REQUIREMENTS and not live_interop_not_applicable_reason(
                target, feature, case
            ):
                raise ValueError(f"live interop scope has an unclassified case: {key!r}")
