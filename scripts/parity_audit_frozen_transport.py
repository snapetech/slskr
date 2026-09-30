"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations

from pathlib import Path
from typing import Any
from parity_audit_shared import (
    LIVE_INTEROP_EXPECTED_FAILURE_DETAIL_TOKENS,
    LIVE_INTEROP_PROOF_REQUIREMENTS,
    UNIVERSAL_BIDIRECTIONAL_TRANSPORTS,
    UNIVERSAL_BIDIRECTIONAL_TRANSPORT_TARGETS,
    UNIVERSAL_TRANSPORT_TARGETS,
)


def validate_universal_transport_scope_contracts(
    slskd_root: Path,
    slskdn_root: Path,
) -> dict[str, frozenset[str]]:
    """Return the source-bound target set for each strict transport check.

    The strict transport contract is target-profile aware. Frozen slskd has
    relay HTTP code, but it has no mesh service router, type-1 obfuscation, or
    private-gateway transport. Those are slskdN-only capabilities; requiring
    them against slskd would test a feature the target does not expose.
    """
    slskd_sources = [
        path.read_text(encoding="utf-8-sig")
        for path in (slskd_root / "src/slskd").rglob("*.cs")
        if path.is_file()
    ]
    slskdn_sources = [
        path.read_text(encoding="utf-8-sig")
        for path in (slskdn_root / "src/slskd").rglob("*.cs")
        if path.is_file()
    ]
    if not slskd_sources or not slskdn_sources:
        raise ValueError("strict transport scope requires both frozen source trees")
    slskd_text = "\n".join(slskd_sources).lower()
    slskdn_text = "\n".join(slskdn_sources).lower()
    source_contracts = {
        "obfuscated-peer-bidirectional": "soulseekobfuscationsupport",
        "overlay-udp-bidirectional": "meshservicerouter",
        "overlay-quic-control-bidirectional": "quicstream",
        "quic-data-bidirectional": "quicstream",
        "relay-gateway-bidirectional": "privategatewaymeshservice",
        "mesh-sync-bidirectional": "meshservicerouter",
        "virtualsoulfind-bidirectional": "virtualsoulfindv2controller",
    }
    if "obfus" in slskd_text:
        raise ValueError(
            "frozen slskd source now exposes obfuscation; refresh the strict transport target contract"
        )
    for check_id, token in source_contracts.items():
        if token not in slskdn_text:
            raise ValueError(
                f"frozen slskdN source no longer exposes {token} for {check_id}"
            )
        if token in slskd_text:
            raise ValueError(
                f"frozen slskd source unexpectedly exposes {token} for {check_id}"
            )
    if set(UNIVERSAL_BIDIRECTIONAL_TRANSPORT_TARGETS) != set(
        UNIVERSAL_BIDIRECTIONAL_TRANSPORTS
    ):
        raise ValueError("strict transport target map is missing a declared transport")
    if any(
        not targets or not targets.issubset(UNIVERSAL_TRANSPORT_TARGETS)
        for targets in UNIVERSAL_BIDIRECTIONAL_TRANSPORT_TARGETS.values()
    ):
        raise ValueError("strict transport target map contains an invalid target set")
    return UNIVERSAL_BIDIRECTIONAL_TRANSPORT_TARGETS


def frozen_slskdn_expected_failure_checks(slskdn_root: Path) -> frozenset[str]:
    """Return negative live checks justified by the exact slskdN source.

    The live TSV is not allowed to turn an arbitrary failed request into
    passing evidence.  Only the pinned source shape that omits the mesh
    registrations and contains the gateway identity validation can enable
    these exact negative checks.  A newer target with the registrations must
    pass the positive rows instead.
    """
    application_path = slskdn_root / "src/slskd/Application.cs"
    pods_controller_path = slskdn_root / "src/slskd/API/Native/PodsController.cs"
    pods_service_path = (
        slskdn_root / "src/slskd/Mesh/ServiceFabric/Services/PodsMeshService.cs"
    )
    gateway_service_path = (
        slskdn_root
        / "src/slskd/Mesh/ServiceFabric/Services/PrivateGatewayMeshService.cs"
    )
    if not all(
        path.is_file()
        for path in (
            application_path,
            pods_controller_path,
            pods_service_path,
            gateway_service_path,
        )
    ):
        return frozenset()

    application = application_path.read_text(encoding="utf-8-sig")
    pods_controller = pods_controller_path.read_text(encoding="utf-8-sig")
    # Require the three registrations that are present in the pinned source;
    # this prevents an unrelated or truncated source snapshot from activating
    # the negative proof allowlist.
    required_existing_registrations = (
        "router.RegisterService(dhtService)",
        "router.RegisterService(holePunchService)",
        "router.RegisterService(meshContentService)",
    )
    if not all(token in application for token in required_existing_registrations):
        return frozenset()

    checks: set[str] = set()
    mesh_controller_path = slskdn_root / "src/slskd/Mesh/API/MeshController.cs"
    mesh_service_path = slskdn_root / "src/slskd/Mesh/MeshSyncService.cs"
    if mesh_controller_path.is_file() and mesh_service_path.is_file():
        mesh_controller = mesh_controller_path.read_text(encoding="utf-8-sig")
        mesh_service = mesh_service_path.read_text(encoding="utf-8-sig")
        if (
            'return BadRequest(new { error = "Failed to sync with peer" });'
            in mesh_controller
            and "if (!result.Success)" in mesh_controller
            and 'result.Error = "Mesh sync transport unavailable"' in mesh_service
            and "public async Task<MeshSyncResult> TrySyncWithPeerAsync" in mesh_service
        ):
            checks.add("protocol-ksdn-mesh-sync-reconnect-retry")
    if "PodsMeshService" not in application:
        checks.update(
            {
                "protocol-slskr-pods-list-slskdn",
                "protocol-slskr-pods-get-slskdn",
                "protocol-slskr-pods-join-slskdn",
                "protocol-slskr-pods-post-slskdn",
                "protocol-slskr-pods-messages-slskdn",
                "protocol-slskr-pods-leave-slskdn",
                "protocol-slskr-gateway-pod-join-slskdn",
            }
        )
    if "PrivateGatewayMeshService" not in application:
        checks.update(
            {
                "protocol-slskr-gateway-open-slskdn",
                "protocol-slskr-gateway-send-slskdn",
                "protocol-slskr-gateway-receive-slskdn",
                "protocol-slskr-gateway-close-slskdn",
            }
        )
    if "RequestingPeerId must match GatewayPeerId" in pods_controller:
        checks.add("runtime-slskdn-gateway-pod-create")
    return frozenset(checks)


def frozen_slskdn_transport_not_applicable_contracts(
    slskdn_root: Path,
) -> dict[str, dict[str, dict[str, dict[str, Any]]]]:
    """Return source-bound strict transport capability exceptions.

    A target-owned service that is implemented but never registered is not a
    live transport.  Likewise, the pinned VirtualSoulfind v2 controller reads
    an unbound options type whose default is disabled, even though the root
    configuration projection reports the feature as enabled.  These are
    explicit negative target contracts, not green peer transactions.

    The returned shape is ``check -> target -> direction -> contract``.  The
    strict auditor still requires a fresh capability artifact whose live rows
    and reason codes match these source-bound contracts.
    """
    contracts: dict[str, dict[str, dict[str, dict[str, Any]]]] = {}
    application_path = slskdn_root / "src/slskd/Application.cs"
    virtual_services_path = (
        slskdn_root / "src/slskd/Bootstrap/VirtualSoulfindServiceCollectionExtensions.cs"
    )
    virtual_controller_path = (
        slskdn_root / "src/slskd/VirtualSoulfind/v2/API/VirtualSoulfindV2Controller.cs"
    )
    virtual_root_options_path = slskdn_root / "src/slskd/Core/Options.cs"
    virtual_options_path = slskdn_root / "src/slskd/VirtualSoulfind/v2/VirtualSoulfindV2Options.cs"
    if not all(
        path.is_file()
        for path in (
            application_path,
            virtual_services_path,
            virtual_controller_path,
            virtual_root_options_path,
            virtual_options_path,
        )
    ):
        return contracts

    application = application_path.read_text(encoding="utf-8-sig")
    expected_failures = frozen_slskdn_expected_failure_checks(slskdn_root)

    # The frozen target exposes overlay listeners, but its reverse pod-route
    # path cannot resolve a replacement peer. The registration API is present
    # only on the resolver itself, has no call site, and its DHT key is a
    # SHA-256 value while the target's remote Store endpoint accepts only the
    # 20-byte Soulseek key shape. Keep the reverse directions explicitly
    # negative until the exact target wires a registration path.
    peer_resolution_path = slskdn_root / "src/slskd/PodCore/PeerResolutionService.cs"
    pod_router_path = slskdn_root / "src/slskd/PodCore/PodMessageRouter.cs"
    pod_services_path = slskdn_root / "src/slskd/PodCore/PodServices.cs"
    mesh_dht_client_path = slskdn_root / "src/slskd/Mesh/Dht/MeshDhtClient.cs"
    dht_mesh_service_path = (
        slskdn_root / "src/slskd/Mesh/ServiceFabric/Services/DhtMeshService.cs"
    )
    routing_paths = (
        peer_resolution_path,
        pod_router_path,
        pod_services_path,
        mesh_dht_client_path,
        dht_mesh_service_path,
    )
    if all(path.is_file() for path in routing_paths):
        peer_resolution = peer_resolution_path.read_text(encoding="utf-8-sig")
        pod_router = pod_router_path.read_text(encoding="utf-8-sig")
        pod_services = pod_services_path.read_text(encoding="utf-8-sig")
        mesh_dht_client = mesh_dht_client_path.read_text(encoding="utf-8-sig")
        dht_mesh_service = dht_mesh_service_path.read_text(encoding="utf-8-sig")
        source_files = [
            path
            for path in (slskdn_root / "src/slskd").rglob("*.cs")
            if path.is_file()
        ]
        registration_callsite_exists = any(
            path != peer_resolution_path
            and "RegisterPeerMapping(" in path.read_text(encoding="utf-8-sig")
            for path in source_files
        )
        reverse_overlay_route_is_unwired = (
            "void RegisterPeerMapping" in peer_resolution
            and "PeerMetadataPrefix = \"peer:metadata:\"" in peer_resolution
            and "ResolvePeerIdToEndpointAsync" in peer_resolution
            and "No endpoint for peer" in pod_router
            and "_peerResolution.ResolvePeerIdToEndpointAsync" in pod_router
            and "QUIC overlay routing available but peer resolution service not yet integrated" in pod_services
            and "SHA256.HashData(Encoding.UTF8.GetBytes(key))" in mesh_dht_client
            and "request.Key.Length != 20" in dht_mesh_service
            and not registration_callsite_exists
        )
        if reverse_overlay_route_is_unwired:
            reverse_overlay_contracts = {
                "overlay-udp-bidirectional": "protocol-slskdn-overlay-udp-slskr",
                "overlay-quic-control-bidirectional": "protocol-slskdn-overlay-quic-control-slskr",
                "quic-data-bidirectional": "protocol-slskdn-quic-data-slskr",
            }
            for check_id, evidence_check in reverse_overlay_contracts.items():
                contracts[check_id] = {
                    "slskdn": {
                        "target-to-slskr": {
                            "reason": "frozen-target-overlay-peer-resolution-unwired",
                            "evidenceChecks": [evidence_check],
                            "evidenceStatus": "ok",
                            "evidenceDetailTokens": {
                                evidence_check: "expected-target-negative endpoint-resolution-unavailable",
                            },
                        },
                    },
                }

    relay_checks = (
        "protocol-slskr-gateway-open-slskdn",
        "protocol-slskr-gateway-send-slskdn",
        "protocol-slskr-gateway-receive-slskdn",
        "protocol-slskr-gateway-close-slskdn",
        "protocol-slskr-gateway-pod-join-slskdn",
    )
    if (
        "PrivateGatewayMeshService" not in application
        and set(relay_checks).issubset(expected_failures)
    ):
        relay_contract: dict[str, dict[str, Any]] = {
            "slskr-to-target": {
                "reason": "frozen-target-private-gateway-service-not-registered",
                "evidenceChecks": list(relay_checks),
                "evidenceStatus": "fail",
                "evidenceDetailTokens": {
                    check: LIVE_INTEROP_EXPECTED_FAILURE_DETAIL_TOKENS[check]
                    for check in relay_checks
                },
            },
            "target-to-slskr": {
                "reason": "frozen-target-private-gateway-service-not-registered",
                "evidenceChecks": list(relay_checks),
                "evidenceStatus": "fail",
                "evidenceDetailTokens": {
                    check: LIVE_INTEROP_EXPECTED_FAILURE_DETAIL_TOKENS[check]
                    for check in relay_checks
                },
            },
        }
        contracts["relay-gateway-bidirectional"] = {
            "slskdn": relay_contract,
        }

    virtual_services = virtual_services_path.read_text(encoding="utf-8-sig")
    virtual_controller = virtual_controller_path.read_text(encoding="utf-8-sig")
    virtual_root_options = virtual_root_options_path.read_text(encoding="utf-8-sig")
    virtual_options = virtual_options_path.read_text(encoding="utf-8-sig")
    virtual_options_are_unbound = (
        "services.AddOptions<VirtualSoulfind.v2.VirtualSoulfindV2Options>();" in virtual_services
        and "Configure<VirtualSoulfind.v2.VirtualSoulfindV2Options>" not in virtual_services
        and "IOptionsMonitor<VirtualSoulfindV2Options>" in virtual_controller
        and "_options.CurrentValue.Enabled" in virtual_controller
        and '"VirtualSoulfind v2 is disabled"' in virtual_controller
        and "VirtualSoulfindV2" in virtual_root_options
        and "bool Enabled" in virtual_options
    )
    if virtual_options_are_unbound:
        virtual_contract = {
            direction: {
                "reason": "frozen-target-virtualsoulfind-v2-controller-options-unbound",
                "evidenceChecks": ["runtime-slskdn-virtualsoulfind-v2-create"],
                "evidenceStatus": "ok",
                "evidenceDetailTokens": {
                    "runtime-slskdn-virtualsoulfind-v2-create": "status=503 body=VirtualSoulfind v2 is disabled",
                },
            }
            for direction in ("slskr-to-target", "target-to-slskr")
        }
        contracts["virtualsoulfind-bidirectional"] = {"slskdn": virtual_contract}
    return contracts


def validate_live_interop_mapping_contracts(root: Path) -> None:
    """Keep promoted live checks tied to the behavior they actually exercise.

    This is intentionally a small source-boundary guard rather than a second
    live test.  The backfill row is allowed to promote only because the
    runner invokes the real backfill route and the daemon implementation
    performs a remote FLAC-header read and hash parse.  If either side is
    removed or reduced to a local-only assertion, the manifest must stop
    promoting that row.
    """
    runner = root / "scripts/run-slskdn-cross-client-interop.sh"
    rust_source = root / "crates/slskr/src/hash_backfill_runtime.rs"
    module_registry = root / "crates/slskr/src/lib.rs"
    runner_source = runner.read_text(encoding="utf-8") if runner.is_file() else ""
    rust_text = rust_source.read_text(encoding="utf-8") if rust_source.is_file() else ""
    registry_text = module_registry.read_text(encoding="utf-8") if module_registry.is_file() else ""
    required_runner_tokens = (
        '"http://127.0.0.1:$slskr_http_port/api/v0/backfill/file"',
        'record_check protocol-slskr-backfill-slskdn ok',
        'hash" == "$slskdn_fixture_sha"',
    )
    required_rust_tokens = (
        "async fn read_remote_flac_header(",
        "parse_flac_backfill_hash(&header)",
        '"backfill transfer token did not match"',
    )
    if not all(token in runner_source for token in required_runner_tokens):
        raise ValueError(
            "live backfill mapping no longer has an exact remote route/hash assertion"
        )
    if "mod hash_backfill_runtime;" not in registry_text:
        raise ValueError("live backfill implementation owner is absent from the daemon registry")
    if not all(token in rust_text for token in required_rust_tokens):
        raise ValueError(
            "live backfill mapping no longer has an exact remote FLAC-header implementation"
        )
    required_mesh_sync_tokens = (
        "protocol-ksdn-mesh-sync-reconnect-retry",
        "/api/v0/mesh/sync/$escaped_slskr",
        "/api/v0/mesh/sync/$escaped_slskdn",
        'expected-target-negative status=400 body={"error":"Failed to sync with peer"}',
        "target_attempts=400,400 replacement_attempts=400,400",
    )
    if not all(token in runner_source for token in required_mesh_sync_tokens):
        raise ValueError(
            "live mesh-sync mapping no longer has the exact repeated target-negative contract"
        )
    mapped_checks = LIVE_INTEROP_PROOF_REQUIREMENTS[
        ("slskdn", "source-feeds-and-discovery", "slskr-initiates-to-target")
    ]
    if mapped_checks != ("protocol-slskr-backfill-slskdn",):
        raise ValueError("live backfill mapping changed without updating its source contract")
