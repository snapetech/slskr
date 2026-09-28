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

include!("controller_tests/segment_01.rs");
include!("controller_tests/segment_02.rs");
include!("controller_tests/segment_03.rs");
include!("controller_tests/segment_04.rs");
include!("controller_tests/segment_05.rs");
include!("controller_tests/segment_06.rs");
include!("controller_tests/segment_08.rs");
include!("controller_tests/segment_09.rs");
include!("controller_tests/segment_10.rs");
include!("controller_tests/segment_11.rs");
include!("controller_tests/segment_12.rs");
include!("controller_tests/segment_13.rs");
include!("controller_tests/segment_14.rs");
include!("controller_tests/segment_15.rs");
include!("controller_tests/segment_16.rs");
include!("controller_tests/segment_17.rs");
include!("controller_tests/segment_18.rs");
include!("controller_tests/segment_19.rs");
include!("controller_tests/segment_20.rs");
include!("controller_tests/segment_21.rs");
include!("controller_tests/segment_22.rs");
include!("controller_tests/segment_23.rs");
include!("controller_tests/segment_24.rs");
include!("controller_tests/segment_25.rs");
include!("controller_tests/segment_26.rs");
include!("controller_tests/segment_27.rs");
include!("controller_tests/segment_28.rs");
include!("controller_tests/segment_29.rs");
