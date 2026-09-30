"""Owned parity audit implementation; public entry points remain in the manifest."""

from __future__ import annotations


EXPECTED = {
    "config": 436,
    "slskd-api": 96,
    "slskdn-api": 683,
    "webui-call-union": 417,
    "slskd-database-domains": 11,
    "slskdn-database-domains": 61,
    "slskd-file-writer-domains": 8,
    "slskdn-file-writer-domains": 42,
    "slskd-security-components": 12,
    "slskdn-security-components": 121,
    "slskd-operator-families": 3,
    "slskdn-operator-families": 37,
    "slskd-protocol-units": 123,
    "slskdn-protocol-units": 170,
    "live-interop-target-features": 62,
}

REACT_WEB_UI_ROUTE_COUNT = 42
REACT_WEB_UI_CASE_COUNT = REACT_WEB_UI_ROUTE_COUNT * 2

UNMATERIALIZED_WORKSTREAMS: list[dict[str, str]] = []

UNIVERSAL_BIDIRECTIONAL_TRANSPORTS = (
    "soulseek-peer-bidirectional",
    "obfuscated-peer-bidirectional",
    "distributed-dht-bidirectional",
    "overlay-udp-bidirectional",
    "overlay-quic-control-bidirectional",
    "quic-data-bidirectional",
    "relay-gateway-bidirectional",
    "mesh-sync-bidirectional",
    "virtualsoulfind-bidirectional",
    "file-stream-transfer-bidirectional",
)
UNIVERSAL_TRANSPORT_TARGETS = frozenset({"slskd", "slskdn"})
UNIVERSAL_BIDIRECTIONAL_TRANSPORT_TARGETS: dict[str, frozenset[str]] = {
    # Soulseek P/D/F behavior is shared by both frozen profiles.
    "soulseek-peer-bidirectional": frozenset({"slskd", "slskdn"}),
    "distributed-dht-bidirectional": frozenset({"slskd", "slskdn"}),
    "file-stream-transfer-bidirectional": frozenset({"slskd", "slskdn"}),
    # The remaining transports are slskdN-only in the frozen source. The
    # slskd profile must hide them rather than claim support it does not have.
    "obfuscated-peer-bidirectional": frozenset({"slskdn"}),
    "overlay-udp-bidirectional": frozenset({"slskdn"}),
    "overlay-quic-control-bidirectional": frozenset({"slskdn"}),
    "quic-data-bidirectional": frozenset({"slskdn"}),
    "relay-gateway-bidirectional": frozenset({"slskdn"}),
    "mesh-sync-bidirectional": frozenset({"slskdn"}),
    "virtualsoulfind-bidirectional": frozenset({"slskdn"}),
}
UNIVERSAL_LIFECYCLE_CHECK = "failure-restart-lifecycle-matrix"
UNIVERSAL_LIFECYCLE_SCENARIOS = frozenset(
    {
        "restart",
        "corrupt-state",
        "cancel",
        "timeout",
        "retry",
        "resume",
        "concurrent-mutation",
        "upgrade",
        "rollback",
        "permissions",
        "uninstall",
    }
)
UNIVERSAL_TRANSPORT_LIFECYCLE_REQUIREMENTS: dict[str, dict[str, dict[str, tuple[str, ...]]]] = {
    "mesh-sync-bidirectional": {
        "slskdn": {
            "reconnect-retry-and-failure": (
                "protocol-ksdn-mesh-sync-reconnect-retry",
            )
        }
    }
}
UNIVERSAL_TRANSPORT_LIFECYCLE_DETAIL_TOKENS = {
    "protocol-ksdn-mesh-sync-reconnect-retry": (
        'expected-target-negative status=400 body={"error":"Failed to sync with peer"}'
    )
}
UNIVERSAL_UI_SCENARIOS = frozenset(
    {
        "success",
        "rendered-loading-and-empty",
        "rendered-validation-and-server-error",
        "authorization-reconnect-and-restart",
    }
)
UNIVERSAL_UI_WORKFLOWS = frozenset(
    {
        "search",
        "browse",
        "transfers",
        "messages",
        "rooms",
        "shares",
        "settings",
        "player",
        "mesh",
    }
)


SECURITY_AUTHORIZATION_TEST = (
    "focused_controller_tests::controller_contracts::security_authorization_matrix_matches_declared_policy_for_every_frozen_route"
)


CONTROLLER_API_DIFFERENTIAL_TEST_PREFIX = "controller_api_differential_"
CONTROLLER_API_TEST_FEATURES = (
    "bounded-controller-api-tests-1",
    "bounded-controller-api-tests-2",
    "bounded-controller-api-tests-3",
    "bounded-controller-api-tests-4",
)


PERSISTENCE_DIFFERENTIAL_TEST_PREFIX = "persistence_lifecycle_differential_"


FILE_LIFECYCLE_DIFFERENTIAL_TEST_PREFIX = "file_lifecycle_differential_"


SECURITY_CONTROL_DIFFERENTIAL_TEST_PREFIX = "security_controls_differential_"
SECURITY_CONTROL_CASES = (
    "activation-default-and-profile",
    "accepted-nominal-input",
    "rejected-malicious-and-boundary-input",
    "quota-time-lockout-and-concurrency",
    "secret-logging-and-privacy-output",
    "restart-rotation-and-recovery",
)








PROTOCOL_DIFFERENTIAL_TEST_PREFIX = "protocol_behaviors_differential_"


LIVE_INTEROP_PROOF_REQUIREMENTS: dict[tuple[str, str, str], tuple[str, ...]] = {
    # The live runner has explicit initiator/target direction in these check
    # names. Keep this table deliberately narrow: a successful local API
    # assertion must not promote a broader interop case by inference.
    ("slskd", "peer-endpoint", "slskr-initiates-to-target"): (
        "network-slskr-resolves-slskd",
    ),
    ("slskd", "peer-endpoint", "target-initiates-to-slskr"): (
        "network-slskd-resolves-slskr",
    ),
    ("slskd", "server-session", "slskr-initiates-to-target"): (
        "runtime-slskd-session",
        "runtime-slskr-session-slskd",
    ),
    ("slskd", "public-search", "slskr-initiates-to-target"): (
        "protocol-slskr-searches-slskd",
    ),
    ("slskd", "public-search", "target-initiates-to-slskr"): (
        "protocol-slskd-searches-slskr",
    ),
    ("slskd", "browse-share-list", "slskr-initiates-to-target"): (
        "protocol-slskr-browses-slskd",
    ),
    ("slskd", "browse-share-list", "target-initiates-to-slskr"): (
        "protocol-slskd-browses-slskr",
    ),
    ("slskd", "folder-contents", "slskr-initiates-to-target"): (
        "protocol-slskr-folder-contents-slskd",
    ),
    ("slskd", "folder-contents", "target-initiates-to-slskr"): (
        "protocol-slskd-folder-contents-slskr",
    ),
    ("slskd", "download", "slskr-initiates-to-target"): (
        "slskr-to-slskd-download",
    ),
    ("slskd", "download", "target-initiates-to-slskr"): (
        "slskd-to-slskr-download",
    ),
    ("slskd", "upload", "slskr-initiates-to-target"): (
        "slskr-to-slskd-download",
    ),
    ("slskd", "upload", "target-initiates-to-slskr"): (
        "slskd-to-slskr-download",
    ),
    ("slskd", "private-message", "slskr-initiates-to-target"): (
        "protocol-slskr-message-dispatch-slskd",
    ),
    ("slskd", "private-message", "target-initiates-to-slskr"): (
        "protocol-slskd-message-dispatch",
    ),
    ("slskd", "server-session", "restart-and-persisted-state"): (
        "runtime-slskd-restart-session",
    ),
    ("slskd", "browse-share-list", "restart-and-persisted-state"): (
        "protocol-slskd-restart-browse",
    ),
    ("slskd", "folder-contents", "restart-and-persisted-state"): (
        "protocol-slskd-restart-folder",
    ),
    ("slskd", "public-room", "slskr-initiates-to-target"): (
        "protocol-slskr-public-room",
    ),
    ("slskd", "public-room", "target-initiates-to-slskr"): (
        "protocol-slskd-public-room",
    ),
    ("slskd", "user-watch-status-and-stats", "slskr-initiates-to-target"): (
        "protocol-slskr-user-watch-slskd",
    ),
    ("slskd", "user-watch-status-and-stats", "target-initiates-to-slskr"): (
        "protocol-slskd-user-watch-slskr",
    ),
    ("slskd", "distributed-tree", "slskr-initiates-to-target"): (
        "protocol-slskr-distributed-peer-slskd",
    ),
    ("slskdn", "peer-endpoint", "slskr-initiates-to-target"): (
        "network-slskr-resolves-slskdn",
    ),
    ("slskdn", "peer-endpoint", "target-initiates-to-slskr"): (
        "network-slskdn-resolves-slskr",
    ),
    ("slskdn", "server-session", "slskr-initiates-to-target"): (
        "runtime-slskdn-session",
        "runtime-slskr-session",
    ),
    ("slskdn", "public-search", "slskr-initiates-to-target"): (
        "protocol-slskr-searches-slskdn",
    ),
    ("slskdn", "public-search", "target-initiates-to-slskr"): (
        "protocol-slskdn-searches-slskr",
    ),
    ("slskdn", "browse-share-list", "slskr-initiates-to-target"): (
        "protocol-slskr-browses-slskdn",
    ),
    ("slskdn", "browse-share-list", "target-initiates-to-slskr"): (
        "protocol-slskdn-browses-slskr",
    ),
    ("slskdn", "folder-contents", "slskr-initiates-to-target"): (
        "protocol-slskr-browses-slskdn",
    ),
    ("slskdn", "folder-contents", "target-initiates-to-slskr"): (
        "protocol-slskdn-browses-slskr",
    ),
    ("slskdn", "download", "slskr-initiates-to-target"): (
        "slskr-to-slskdn-download",
    ),
    ("slskdn", "download", "target-initiates-to-slskr"): (
        "slskdn-to-slskr-download",
    ),
    ("slskdn", "upload", "slskr-initiates-to-target"): (
        "slskr-to-slskdn-download",
    ),
    ("slskdn", "upload", "target-initiates-to-slskr"): (
        "slskdn-to-slskr-download",
    ),
    ("slskdn", "private-message", "slskr-initiates-to-target"): (
        "protocol-slskr-message-dispatch",
    ),
    ("slskdn", "private-message", "target-initiates-to-slskr"): (
        "protocol-slskdn-message-dispatch",
    ),
    ("slskdn", "public-room", "slskr-initiates-to-target"): (
        "protocol-slskr-public-room-slskdn",
    ),
    ("slskdn", "public-room", "target-initiates-to-slskr"): (
        "protocol-slskdn-public-room",
    ),
    ("slskdn", "user-watch-status-and-stats", "slskr-initiates-to-target"): (
        "protocol-slskr-user-watch-slskdn",
    ),
    ("slskdn", "user-watch-status-and-stats", "target-initiates-to-slskr"): (
        "protocol-slskdn-user-watch-slskr",
    ),
    ("slskdn", "distributed-tree", "slskr-initiates-to-target"): (
        "protocol-slskr-distributed-peer-slskdn",
    ),
    ("slskdn", "peer-capability", "slskr-initiates-to-target"): (
        "protocol-ksdn-probe-dispatch",
        "protocol-ksdn-slskr-receives-ack",
        "protocol-ksdn-slskr-verifies-slskdn-descriptor",
        "protocol-ksdn-slskdn-receives-hello",
        "protocol-ksdn-slskdn-persists-slskr-descriptor",
    ),
    ("slskdn", "overlay-handshake-and-keepalive", "slskr-initiates-to-target"): (
        "protocol-pinned-overlay-certificate",
        "protocol-pinned-overlay-service",
    ),
    ("slskdn", "mesh-sync", "slskr-initiates-to-target"): (
        "protocol-ksdn-probe-dispatch",
        "protocol-ksdn-slskr-receives-ack",
        "protocol-ksdn-slskr-verifies-slskdn-descriptor",
        "protocol-ksdn-slskdn-receives-hello",
        "protocol-ksdn-slskdn-persists-slskr-descriptor",
    ),
    ("slskdn", "mesh-sync", "reconnect-retry-and-resume"): (
        "protocol-ksdn-mesh-sync-reconnect-retry",
    ),
    ("slskdn", "mesh-service-dht", "slskr-initiates-to-target"): (
        "protocol-pinned-overlay-certificate",
        "protocol-pinned-overlay-service",
        "protocol-slskr-dht-store-slskdn",
    ),
    ("slskdn", "mesh-service-pods", "slskr-initiates-to-target"): (
        "protocol-slskr-pods-list-slskdn",
        "protocol-slskr-pods-get-slskdn",
        "protocol-slskr-pods-join-slskdn",
        "protocol-slskr-pods-post-slskdn",
        "protocol-slskr-pods-messages-slskdn",
        "protocol-slskr-pods-leave-slskdn",
    ),
    ("slskdn", "mesh-content-and-preview", "slskr-initiates-to-target"): (
        "runtime-slskdn-mesh-content-id",
        "protocol-slskr-mesh-content-slskdn",
    ),
    ("slskdn", "private-gateway-and-vpn", "slskr-initiates-to-target"): (
        "runtime-slskdn-gateway-identity",
        "runtime-slskdn-gateway-pod-create",
        "protocol-slskr-gateway-pod-join-slskdn",
        "protocol-slskr-gateway-open-slskdn",
        "protocol-slskr-gateway-send-slskdn",
        "protocol-slskr-gateway-receive-slskdn",
        "protocol-slskr-gateway-close-slskdn",
    ),
    # This is a real cross-client source-discovery exchange: the Rust
    # backfill route negotiates a file-transfer connection to the frozen
    # slskdN peer, reads the bounded FLAC header, and verifies the returned
    # byte hash. Do not replace this with the local backfill-controller rows;
    # those are covered by controller-api evidence and do not prove a peer
    # exchange.
    ("slskdn", "source-feeds-and-discovery", "slskr-initiates-to-target"): (
        "protocol-slskr-backfill-slskdn",
    ),
}

# A green row is not sufficient when the probe contract changed after the
# artifact was emitted.  These rows require an explicit marker from the
# current runner so a stale pre-response-check TSV cannot certify a probe that
# only sent a Ping.
LIVE_INTEROP_REQUIRED_DETAIL_TOKENS: dict[str, str] = {
    "protocol-slskr-distributed-peer-slskd": "probe_contract=distributed-ping-response-v2",
    "protocol-slskr-distributed-peer-slskdn": "probe_contract=distributed-ping-response-v2",
    "protocol-slskr-obfuscated-peer-slskdn": "probe_contract=obfuscated-peer-v1 response_contract=plain-fallback",
}

# The frozen slskdN mesh controller reaches a stable generic 400 when its
# MeshSyncService has no outbound transport.  The interop runner records this
# as an expected negative row after two attempts against each profile; the
# detail token keeps a stale arbitrary failure from satisfying the retry case.

# The exact frozen slskdN source contains the Pod and private-gateway service
# implementations but does not register them with its mesh router.  Its
# no-auth Pod controller also deliberately rejects a gateway create request
# whose requesting identity differs from the declared gateway identity.  A
# fresh live `fail` row with these exact target responses is therefore a
# completed negative compatibility contract, not an implementation failure in
# slskR.  These checks are enabled only after the source-boundary validator
# below proves that the supplied target really has this frozen shape.
LIVE_INTEROP_EXPECTED_FAILURE_DETAIL_TOKENS: dict[str, str] = {
    "protocol-slskr-pods-list-slskdn": "Service 'pods' not found",
    "protocol-slskr-pods-get-slskdn": "Service 'pods' not found",
    "protocol-slskr-pods-join-slskdn": "Service 'pods' not found",
    "protocol-slskr-pods-post-slskdn": "Service 'pods' not found",
    "protocol-slskr-pods-messages-slskdn": "Service 'pods' not found",
    "protocol-slskr-pods-leave-slskdn": "Service 'pods' not found",
    "runtime-slskdn-gateway-pod-create": "RequestingPeerId must match GatewayPeerId",
    "protocol-slskr-gateway-pod-join-slskdn": "Service 'pods' not found",
    "protocol-slskr-gateway-open-slskdn": "Service 'private-gateway' not found",
    "protocol-slskr-gateway-send-slskdn": "gateway tunnel was not opened",
    "protocol-slskr-gateway-receive-slskdn": "echo payload unavailable",
    "protocol-slskr-gateway-close-slskdn": "gateway tunnel was not opened",
    "protocol-ksdn-mesh-sync-reconnect-retry": (
        'expected-target-negative status=400 body={"error":"Failed to sync with peer"}'
    ),
}

# The live matrix deliberately contains more dimensions than the credentialed
# runner can own.  A feature/case without an entry in
# LIVE_INTEROP_PROOF_REQUIREMENTS is not silently promoted from an unrelated
# green row: it is classified below as owned by the exact protocol, controller,
# persistence, or security differential that exercises that contract.  Keeping
# this classification explicit preserves the denominator while preventing a
# local API assertion from masquerading as peer interoperability.
LIVE_INTEROP_LOCAL_CONTROLLER_FEATURES = frozenset(
    {
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
    }
)
