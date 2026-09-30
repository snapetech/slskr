#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
#[path = "controller_tests/native_route_contracts.rs"]
mod native_route_contracts;
#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
use self::native_route_contracts::{
    controller_api_differential_compatibility_aliases_reach_state_backed_routes,
    controller_api_differential_native_capability_and_library_health_contracts,
};

#[cfg(feature = "full-controller-tests")]
#[path = "controller_tests/collection_authorization_contracts.rs"]
mod collection_authorization_contracts;

#[cfg(feature = "full-controller-tests")]
#[path = "controller_tests/controller_surface_contracts.rs"]
mod controller_surface_contracts;

#[path = "controller_tests/quarantine_verdict_fixture.rs"]
mod quarantine_verdict_fixture;
use self::quarantine_verdict_fixture::quarantine_signed_verdict_json;

#[cfg(feature = "full-controller-tests")]
#[path = "controller_tests/quarantine_contracts.rs"]
mod quarantine_contracts;

#[cfg(feature = "full-controller-tests")]
#[path = "controller_tests/listening_party_contracts.rs"]
mod listening_party_contracts;

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
#[path = "controller_tests/mesh_contracts.rs"]
mod mesh_contracts;
#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
use self::mesh_contracts::{
    controller_api_differential_mesh_controller_edge_cases,
    controller_api_differential_mesh_merge_publish_restart_and_concurrency,
    controller_api_differential_mesh_message_runtime,
    controller_api_differential_mesh_runtime_and_nat_lifecycle,
    controller_api_differential_mesh_stats_reflect_real_merge_activity_not_hardcoded_zeros,
    controller_api_differential_mesh_sync_failure_and_concurrency_contracts,
};

#[cfg(feature = "full-controller-tests")]
#[path = "controller_tests/media_contracts.rs"]
mod media_contracts;

#[cfg(feature = "full-controller-tests")]
#[path = "controller_tests/songid_contracts.rs"]
mod songid_contracts;

#[cfg(feature = "full-controller-tests")]
#[path = "controller_tests/bridge_contracts.rs"]
mod bridge_contracts;

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    ffi::OsString,
    fs,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::Engine;
use slskr_client::protocol::peer::{FileEntry, FileSearchResponse};
use tokio::sync::{mpsc, RwLock};

use crate::config::{json_escape, redact_username, ConfigEnv, FileConfig};
use crate::utils::{
    normalize_api_path, parse_route, percent_decode, query_params, split_request_target,
};
#[path = "controller_tests/full_pods_contracts.rs"]
mod full_pods_contracts;
use self::full_pods_contracts::*;

#[path = "controller_tests/full_security_contracts.rs"]
mod full_security_contracts;
use self::full_security_contracts::*;

#[path = "controller_tests/full_state_fixtures.rs"]
mod full_state_fixtures;
use self::full_state_fixtures::*;

#[path = "controller_tests/full_configuration_contracts.rs"]
mod full_configuration_contracts;
use self::full_configuration_contracts::*;

#[path = "controller_tests/full_peer_network_contracts.rs"]
mod full_peer_network_contracts;
use self::full_peer_network_contracts::*;

#[path = "controller_tests/full_route_fixtures.rs"]
mod full_route_fixtures;
use self::full_route_fixtures::*;

#[path = "controller_tests/full_controller_api_contracts.rs"]
mod full_controller_api_contracts;
use self::full_controller_api_contracts::*;

#[path = "controller_tests/full_native_api_contracts.rs"]
mod full_native_api_contracts;
use self::full_native_api_contracts::*;

#[path = "controller_tests/full_search_contracts.rs"]
mod full_search_contracts;
use self::full_search_contracts::*;

#[path = "controller_tests/full_shares_contracts.rs"]
mod full_shares_contracts;
use self::full_shares_contracts::*;

#[path = "controller_tests/full_native_api_differential.rs"]
mod full_native_api_differential;
use self::full_native_api_differential::*;

#[path = "controller_tests/full_messaging_contracts.rs"]
mod full_messaging_contracts;
use self::full_messaging_contracts::*;

#[path = "controller_tests/full_session_contracts.rs"]
mod full_session_contracts;
use self::full_session_contracts::*;

#[path = "controller_tests/full_session_differential.rs"]
mod full_session_differential;
use self::full_session_differential::*;

#[path = "controller_tests/full_versioned_api_contracts.rs"]
mod full_versioned_api_contracts;
use self::full_versioned_api_contracts::*;

#[path = "controller_tests/full_telemetry_contracts.rs"]
mod full_telemetry_contracts;
use self::full_telemetry_contracts::*;

#[path = "controller_tests/full_mesh_contracts.rs"]
mod full_mesh_contracts;
use self::full_mesh_contracts::*;

#[path = "controller_tests/full_web_routes_contracts.rs"]
mod full_web_routes_contracts;
use self::full_web_routes_contracts::*;

#[path = "controller_tests/full_lifecycle_contracts.rs"]
mod full_lifecycle_contracts;
use self::full_lifecycle_contracts::*;

#[path = "controller_tests/full_persistence_contracts.rs"]
mod full_persistence_contracts;
use self::full_persistence_contracts::*;

#[path = "controller_tests/full_integrations_contracts.rs"]
mod full_integrations_contracts;
use self::full_integrations_contracts::*;

#[path = "controller_tests/full_route_validation_contracts.rs"]
mod full_route_validation_contracts;
use self::full_route_validation_contracts::*;

#[path = "controller_tests/full_lifecycle_differential.rs"]
mod full_lifecycle_differential;
use self::full_lifecycle_differential::*;

#[path = "controller_tests/full_application_contracts.rs"]
mod full_application_contracts;
use self::full_application_contracts::*;

#[path = "controller_tests/full_capabilities_contracts.rs"]
mod full_capabilities_contracts;
use self::full_capabilities_contracts::*;

#[path = "controller_tests/full_shares_differential.rs"]
mod full_shares_differential;
use self::full_shares_differential::*;

#[path = "controller_tests/full_peer_network_differential.rs"]
mod full_peer_network_differential;
use self::full_peer_network_differential::*;

#[path = "controller_tests/full_mesh_share_contracts.rs"]
mod full_mesh_share_contracts;
use self::full_mesh_share_contracts::*;

#[path = "controller_tests/full_mesh_overlay_contracts.rs"]
mod full_mesh_overlay_contracts;
use self::full_mesh_overlay_contracts::*;

#[path = "controller_tests/full_mesh_service_contracts.rs"]
mod full_mesh_service_contracts;
use self::full_mesh_service_contracts::*;

#[path = "controller_tests/full_dht_rendezvous_differential.rs"]
mod full_dht_rendezvous_differential;
use self::full_dht_rendezvous_differential::*;

#[path = "controller_tests/full_swarm_analytics_contracts.rs"]
mod full_swarm_analytics_contracts;
use self::full_swarm_analytics_contracts::*;

#[path = "controller_tests/full_transfers_contracts.rs"]
mod full_transfers_contracts;
use self::full_transfers_contracts::*;

#[path = "controller_tests/full_mesh_fixtures.rs"]
mod full_mesh_fixtures;
use self::full_mesh_fixtures::*;

#[path = "controller_tests/full_content_discovery_contracts.rs"]
mod full_content_discovery_contracts;
use self::full_content_discovery_contracts::*;

#[path = "controller_tests/full_browse_contracts.rs"]
mod full_browse_contracts;
use self::full_browse_contracts::*;

#[path = "controller_tests/full_storage_contracts.rs"]
mod full_storage_contracts;
use self::full_storage_contracts::*;

#[path = "controller_tests/full_automation_differential.rs"]
mod full_automation_differential;
use self::full_automation_differential::*;

#[path = "controller_tests/full_controller_api_differential.rs"]
mod full_controller_api_differential;
use self::full_controller_api_differential::*;

#[path = "controller_tests/full_native_api_differential_fixtures.rs"]
mod full_native_api_differential_fixtures;
use self::full_native_api_differential_fixtures::*;

#[path = "controller_tests/full_search_differential.rs"]
mod full_search_differential;
use self::full_search_differential::*;

#[path = "controller_tests/full_events_contracts.rs"]
mod full_events_contracts;
use self::full_events_contracts::*;

#[path = "controller_tests/full_library_contracts.rs"]
mod full_library_contracts;
use self::full_library_contracts::*;

#[path = "controller_tests/full_compatibility_contracts.rs"]
mod full_compatibility_contracts;
use self::full_compatibility_contracts::*;

#[path = "controller_tests/full_media_contracts.rs"]
mod full_media_contracts;
use self::full_media_contracts::*;

#[path = "controller_tests/full_library_differential.rs"]
mod full_library_differential;
use self::full_library_differential::*;

#[path = "controller_tests/full_integrations_contracts_02.rs"]
mod full_integrations_contracts_02;
use self::full_integrations_contracts_02::*;

#[path = "controller_tests/full_transfers_differential.rs"]
mod full_transfers_differential;
use self::full_transfers_differential::*;

#[path = "controller_tests/full_bridge_contracts.rs"]
mod full_bridge_contracts;
use self::full_bridge_contracts::*;

#[path = "controller_tests/full_protocol_differential.rs"]
mod full_protocol_differential;
use self::full_protocol_differential::*;

#[path = "controller_tests/full_bridge_pod_fixtures.rs"]
mod full_bridge_pod_fixtures;
use self::full_bridge_pod_fixtures::*;

#[path = "controller_tests/full_quarantine_contracts.rs"]
mod full_quarantine_contracts;
use self::full_quarantine_contracts::*;

#[path = "controller_tests/full_pods_differential.rs"]
mod full_pods_differential;
use self::full_pods_differential::*;

#[path = "controller_tests/full_messaging_differential.rs"]
mod full_messaging_differential;
use self::full_messaging_differential::*;

#[path = "controller_tests/full_federation_contracts.rs"]
mod full_federation_contracts;
use self::full_federation_contracts::*;

#[path = "controller_tests/full_federation_fixtures.rs"]
mod full_federation_fixtures;
use self::full_federation_fixtures::*;

#[path = "controller_tests/full_relay_contracts.rs"]
mod full_relay_contracts;
use self::full_relay_contracts::*;

#[path = "controller_tests/full_capabilities_differential.rs"]
mod full_capabilities_differential;
use self::full_capabilities_differential::*;

#[path = "controller_tests/full_versioned_api_differential.rs"]
mod full_versioned_api_differential;
use self::full_versioned_api_differential::*;

#[path = "controller_tests/full_route_validation_differential.rs"]
mod full_route_validation_differential;
use self::full_route_validation_differential::*;

#[path = "controller_tests/full_persistence_differential.rs"]
mod full_persistence_differential;
use self::full_persistence_differential::*;

#[path = "controller_tests/full_library_differential_02.rs"]
mod full_library_differential_02;
use self::full_library_differential_02::*;

#[path = "controller_tests/full_activity_differential.rs"]
mod full_activity_differential;
use self::full_activity_differential::*;

#[path = "controller_tests/full_compatibility_differential.rs"]
mod full_compatibility_differential;
use self::full_compatibility_differential::*;

#[path = "controller_tests/full_mesh_differential_02.rs"]
mod full_mesh_differential_02;
use self::full_mesh_differential_02::*;

#[path = "controller_tests/full_bridge_differential.rs"]
mod full_bridge_differential;
use self::full_bridge_differential::*;

#[path = "controller_tests/full_pods_differential_02.rs"]
mod full_pods_differential_02;
use self::full_pods_differential_02::*;

#[path = "controller_tests/full_pods_differential_03.rs"]
mod full_pods_differential_03;
use self::full_pods_differential_03::*;

#[path = "controller_tests/full_media_differential.rs"]
mod full_media_differential;
use self::full_media_differential::*;

#[path = "controller_tests/full_media_differential_02.rs"]
mod full_media_differential_02;
use self::full_media_differential_02::*;

#[path = "controller_tests/full_federation_differential.rs"]
mod full_federation_differential;
use self::full_federation_differential::*;

#[path = "controller_tests/full_federation_differential_fixtures.rs"]
mod full_federation_differential_fixtures;
use self::full_federation_differential_fixtures::*;

#[path = "controller_tests/full_content_discovery_differential.rs"]
mod full_content_discovery_differential;
use self::full_content_discovery_differential::*;

#[path = "controller_tests/full_application_differential.rs"]
mod full_application_differential;
use self::full_application_differential::*;

#[path = "controller_tests/full_quarantine_differential.rs"]
mod full_quarantine_differential;
use self::full_quarantine_differential::*;

#[path = "controller_tests/full_media_differential_fixtures.rs"]
mod full_media_differential_fixtures;
use self::full_media_differential_fixtures::*;

#[path = "controller_tests/full_security_differential.rs"]
mod full_security_differential;
use self::full_security_differential::*;

#[path = "controller_tests/full_integrations_differential.rs"]
mod full_integrations_differential;
use self::full_integrations_differential::*;

#[path = "controller_tests/full_pods_differential_04.rs"]
mod full_pods_differential_04;
use self::full_pods_differential_04::*;

#[path = "controller_tests/full_pods_differential_05.rs"]
mod full_pods_differential_05;
use self::full_pods_differential_05::*;

#[path = "controller_tests/full_messaging_differential_02.rs"]
mod full_messaging_differential_02;
use self::full_messaging_differential_02::*;

#[path = "controller_tests/full_pods_differential_06.rs"]
mod full_pods_differential_06;
use self::full_pods_differential_06::*;

#[path = "controller_tests/full_native_api_differential_02.rs"]
mod full_native_api_differential_02;
use self::full_native_api_differential_02::*;

#[path = "controller_tests/full_telemetry_differential.rs"]
mod full_telemetry_differential;
use self::full_telemetry_differential::*;

#[path = "controller_tests/full_versioned_api_differential_02.rs"]
mod full_versioned_api_differential_02;
use self::full_versioned_api_differential_02::*;

#[path = "controller_tests/full_shares_differential_02.rs"]
mod full_shares_differential_02;
use self::full_shares_differential_02::*;

#[path = "controller_tests/full_configuration_differential.rs"]
mod full_configuration_differential;
use self::full_configuration_differential::*;

#[path = "controller_tests/full_storage_differential.rs"]
mod full_storage_differential;
use self::full_storage_differential::*;

#[path = "controller_tests/full_persistence_differential_02.rs"]
mod full_persistence_differential_02;
use self::full_persistence_differential_02::*;

#[path = "controller_tests/full_transfers_contracts_02.rs"]
mod full_transfers_contracts_02;
use self::full_transfers_contracts_02::*;

#[path = "controller_tests/full_search_contracts_02.rs"]
mod full_search_contracts_02;
use self::full_search_contracts_02::*;

#[path = "controller_tests/full_pods_contracts_02.rs"]
mod full_pods_contracts_02;
use self::full_pods_contracts_02::*;

#[path = "controller_tests/full_dispatch_contracts.rs"]
mod full_dispatch_contracts;
use self::full_dispatch_contracts::*;

#[path = "controller_tests/full_integrations_differential_02.rs"]
mod full_integrations_differential_02;
use self::full_integrations_differential_02::*;

#[path = "controller_tests/full_controller_api_differential_02.rs"]
mod full_controller_api_differential_02;
use self::full_controller_api_differential_02::*;

#[path = "controller_tests/full_controller_api_differential_03.rs"]
mod full_controller_api_differential_03;
use self::full_controller_api_differential_03::*;

#[path = "controller_tests/full_controller_api_differential_04.rs"]
mod full_controller_api_differential_04;
use self::full_controller_api_differential_04::*;

#[path = "controller_tests/full_native_api_differential_03.rs"]
mod full_native_api_differential_03;
use self::full_native_api_differential_03::*;

#[path = "controller_tests/full_controller_api_differential_05.rs"]
mod full_controller_api_differential_05;
use self::full_controller_api_differential_05::*;

#[path = "controller_tests/full_controller_api_differential_06.rs"]
mod full_controller_api_differential_06;
use self::full_controller_api_differential_06::*;

#[path = "controller_tests/full_native_api_differential_04.rs"]
mod full_native_api_differential_04;
use self::full_native_api_differential_04::*;

#[path = "controller_tests/full_security_differential_02.rs"]
mod full_security_differential_02;
use self::full_security_differential_02::*;

#[path = "controller_tests/full_security_differential_03.rs"]
mod full_security_differential_03;
use self::full_security_differential_03::*;

#[path = "controller_tests/full_controller_api_differential_fixtures.rs"]
mod full_controller_api_differential_fixtures;
use self::full_controller_api_differential_fixtures::*;

#[path = "controller_tests/full_persistence_differential_03.rs"]
mod full_persistence_differential_03;
use self::full_persistence_differential_03::*;

#[path = "controller_tests/full_content_discovery_differential_02.rs"]
mod full_content_discovery_differential_02;
use self::full_content_discovery_differential_02::*;

#[path = "controller_tests/full_bridge_differential_02.rs"]
mod full_bridge_differential_02;
use self::full_bridge_differential_02::*;

#[path = "controller_tests/full_pods_differential_fixtures.rs"]
mod full_pods_differential_fixtures;
use self::full_pods_differential_fixtures::*;

#[path = "controller_tests/full_library_differential_03.rs"]
mod full_library_differential_03;
use self::full_library_differential_03::*;

#[path = "controller_tests/full_security_differential_04.rs"]
mod full_security_differential_04;
use self::full_security_differential_04::*;

#[path = "controller_tests/full_integrations_differential_03.rs"]
mod full_integrations_differential_03;
use self::full_integrations_differential_03::*;

#[path = "controller_tests/full_storage_differential_02.rs"]
mod full_storage_differential_02;
use self::full_storage_differential_02::*;

#[path = "controller_tests/full_persistence_fixtures.rs"]
mod full_persistence_fixtures;
use self::full_persistence_fixtures::*;

#[path = "controller_tests/full_media_differential_03.rs"]
mod full_media_differential_03;
use self::full_media_differential_03::*;

#[path = "controller_tests/full_relay_differential.rs"]
mod full_relay_differential;
use self::full_relay_differential::*;

#[path = "controller_tests/full_relay_differential_fixtures.rs"]
mod full_relay_differential_fixtures;
use self::full_relay_differential_fixtures::*;

#[path = "controller_tests/full_messaging_differential_03.rs"]
mod full_messaging_differential_03;
use self::full_messaging_differential_03::*;

#[path = "controller_tests/full_bounded_runners.rs"]
mod full_bounded_runners;
use self::full_bounded_runners::*;

#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1",
    feature = "bounded-controller-api-tests-2",
    feature = "bounded-controller-api-tests-3",
    feature = "bounded-controller-api-tests-4",
    feature = "bounded-persistence-tests",
    feature = "bounded-file-lifecycle-tests",
    feature = "bounded-protocol-tests",
    feature = "bounded-security-control-tests",
    feature = "bounded-security-authorization-tests"
))]
pub use self::full_bounded_runners::run_bounded_differential_tests;
