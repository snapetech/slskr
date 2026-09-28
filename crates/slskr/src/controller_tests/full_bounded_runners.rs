//! Controller full bounded runners ownership.

use super::*;

pub(super) fn run_bounded_future<F>(future: F)
where
    F: std::future::Future<Output = ()>,
{
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("bounded differential runtime")
        .block_on(future);
}

pub(super) fn run_controller_future_on_large_stack<F, Fut>(name: &'static str, factory: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + 'static,
{
    std::thread::Builder::new()
        .name(name.to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create large-stack controller test runtime")
                .block_on(factory())
        })
        .expect("spawn large-stack controller test")
        .join()
        .expect("join large-stack controller test");
}

#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) fn run_bounded_controller_api_tests_1() {
    run_bounded_future(async {
        controller_api_differential_native_library_fallback_uses_current_case_mode_for_share_filters().await;
        controller_api_differential_session_issue_and_revoke().await;
        controller_api_differential_server_session_open_cases().await;
        controller_api_differential_bounded_activity_and_network_polling_routes_project_local_state().await;
        controller_api_differential_native_network_stats_edge_contracts().await;
        controller_api_differential_native_read_projection_runtime_contracts().await;
        controller_api_differential_native_solid_status_edge_contracts().await;
        controller_api_differential_native_solid_resolution_runtime_contracts().await;
        controller_api_differential_incremental_transfer_and_message_routes_validate_cursors_and_bound_history().await;
        controller_api_differential_portforwarding_start_readback().await;
        controller_api_differential_overlay_gateway_populated_gets().await;
        controller_api_differential_automation_compat_routes_use_expected_shapes().await;
        controller_api_differential_peer_and_mesh_preview_stream_tickets_are_short_lived().await;
        controller_api_differential_peer_stream_ticket_validation_and_limits().await;
        controller_api_differential_mesh_stream_ticket_validation_and_limits().await;
        controller_api_differential_controller_application_dump_contracts().await;
        run_controller_future_on_large_stack("native-application-dump-gates", || {
            controller_api_differential_native_application_dump_gates_impl()
        });
        controller_api_differential_native_application_open_cases().await;
        controller_api_differential_search_api_creates_reads_and_completes_records().await;
        controller_api_differential_search_creation_rehydrates().await;
        controller_api_differential_search_mutation_lifecycle().await;
        controller_api_differential_library_issue_fix_rehydrates().await;
        controller_api_differential_transfer_api_creates_updates_and_reports_stats().await;
        controller_api_differential_transfer_cleanup_persistence().await;
        controller_api_differential_transfer_report_contracts().await;
        controller_api_differential_transfer_upload_diagnostics().await;
        controller_api_differential_compatibility_aliases_reach_state_backed_routes().await;
        controller_api_differential_native_capability_and_library_health_contracts().await;
        controller_api_differential_mesh_stats_reflect_real_merge_activity_not_hardcoded_zeros()
            .await;
        controller_api_differential_mesh_message_runtime().await;
        controller_api_differential_mesh_controller_edge_cases().await;
        controller_api_differential_mesh_runtime_and_nat_lifecycle().await;
        controller_api_differential_mesh_merge_publish_restart_and_concurrency().await;
        controller_api_differential_mesh_sync_failure_and_concurrency_contracts().await;
        controller_api_differential_podcore_content_metadata_requires_a_real_content_id().await;
        controller_api_differential_podcore_content_metadata_uses_musicbrainz_recording_release_and_artist_shapes().await;
        controller_api_differential_podcore_content_search_returns_musicbrainz_recording_results()
            .await;
        controller_api_differential_podcore_dht_stats_reflect_real_publications_not_a_pod_count_proxy().await;
        controller_api_differential_podcore_dht_metadata_reads_and_verifies_the_published_record()
            .await;
        controller_api_differential_podcore_backfill_sync_and_sync_all_report_real_local_work()
            .await;
        controller_api_differential_podcore_discovery_stats_use_registrations_and_search_activity()
            .await;
        controller_api_differential_user_notes_lifecycle().await;
        controller_api_differential_mesh_http_disabled_shape().await;
        controller_api_differential_library_jobs_and_discovery_projections().await;
        controller_api_differential_user_browse_api_requests_and_ingests_entries().await;
        controller_api_differential_controller_browse_status_tracks_request_failure_and_completion(
        )
        .await;
        controller_api_differential_joined_room_server_snapshot_populates_the_real_user_roster()
            .await;
        controller_api_differential_soulseek_user_interests_route_returns_remote_server_response()
            .await;
        controller_api_differential_uuid_guarded_families_reject_malformed_first_id().await;
        controller_api_differential_versioned_get_contract_fixed_route_responses().await;
        controller_api_differential_versioned_get_contract_missing_resource_responses().await;
        controller_api_differential_read_only_routes_have_real_contract_shapes().await;
        controller_api_differential_runtime_control_routes_survive_persistence_failure().await;
        controller_api_differential_contact_wishlist_collection_routes_survive_persistence_failure(
        )
        .await;
        controller_api_differential_contacts_versioned_crud_persistence_and_concurrency().await;
        controller_api_differential_contacts_discovery_and_read_edges().await;
        controller_api_differential_library_interests_nowplaying_messages_survive_persistence_failure().await;
        controller_api_differential_pod_management_routes_persist_crud_members_and_bindings().await;
        controller_api_differential_collections_items_crud_reorder_lifecycle().await;
        controller_api_differential_collections_persistence_and_concurrency().await;
        controller_api_differential_activity_hashdb_and_transport_status_gets().await;
        controller_api_differential_mesh_rendezvous_and_capabilities_gets().await;
        controller_api_differential_dht_rendezvous_residuals().await;
        controller_api_differential_swarm_analytics_gets().await;
        controller_api_differential_compatibility_projection_tail().await;
        controller_api_differential_mesh_signal_runtime_switches().await;
        controller_api_differential_bridge_config_populated_projection().await;
    });
}

#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
pub(super) fn run_bounded_controller_api_tests_2() {
    run_bounded_future(async {
        controller_api_differential_bridge_clients_populated_projection().await;
        controller_api_differential_bridge_rooms_populated_projection().await;
        controller_api_differential_podcore_stats_gets().await;
        controller_api_differential_podcore_routing().await;
        controller_api_differential_podcore_maintenance_mutations().await;
        controller_api_differential_podcore_channel_crud().await;
        controller_api_differential_podcore_channel_lifecycle().await;
        controller_api_differential_podcore_opinion_empty_gets().await;
        controller_api_differential_podcore_opinion_populated_gets().await;
        controller_api_differential_podcore_opinion_publish().await;
        controller_api_differential_podcore_opinion_actions().await;
        controller_api_differential_podcore_opinion_missing_gets_and_actions().await;
        controller_api_differential_podcore_signing_verify().await;
        controller_api_differential_share_grants_crud().await;
        controller_api_differential_share_grants_persistence_and_concurrency().await;
        controller_api_differential_mediacore_mutations().await;
        controller_api_differential_mediacore_validation_tail().await;
        controller_api_differential_mediacore_descriptor_lifecycle().await;
        controller_api_differential_mediacore_ipld_and_fuzzy_find().await;
        controller_api_differential_materialized_empty_state_gets().await;
        controller_api_differential_mediacore_stats_malformed_queries().await;
        controller_api_differential_mediacore_resource_malformed_queries().await;
        controller_api_differential_openapi_mutation_dtos_tail().await;
        controller_api_differential_collections_ownership_scoping().await;
        controller_api_differential_share_grants_ownership_scoping().await;
        controller_api_differential_bridge_routes().await;
        controller_api_differential_unversioned_bridge_version_validation().await;
        controller_api_differential_unversioned_mutation_version_validation().await;
        controller_api_differential_songid_run_lifecycle().await;
        controller_api_differential_listening_party_and_transports_status().await;
        controller_api_differential_listening_party_open_cases().await;
        controller_api_differential_activitypub_actor_and_webfinger().await;
        controller_api_differential_activitypub_collection_empty_and_missing_gets().await;
        controller_api_differential_activitypub_open_cases();
        controller_api_differential_hashdb_paging().await;
        controller_api_differential_hashdb_validation_and_empty_contracts().await;
        controller_api_differential_discovery_graph_and_opinions().await;
        controller_api_differential_opinion_open_cases().await;
        controller_api_differential_discovery_graph_edge_contracts().await;
        controller_api_differential_deterministic_openapi_mutations().await;
        controller_api_differential_versioned_openapi_validation_rejections().await;
        controller_api_differential_versioned_openapi_large_dtos().await;
        controller_api_differential_versioned_auxiliary_mutations().await;
        controller_api_differential_release_radar().await;
        controller_api_differential_library_health().await;
        controller_api_differential_library_health_versioned_edge_states().await;
        controller_api_differential_bridge_admin_and_federation_diagnostics().await;
        controller_api_differential_bridge_admin_stats_and_source_feed_preview().await;
        controller_api_differential_extended_controller_mutations().await;
        controller_api_differential_native_ranking_contracts().await;
        controller_api_differential_quarantine_jury().await;
        controller_api_differential_quarantine_jury_open_cases().await;
        controller_api_differential_content_bound_stream_tickets().await;
        run_controller_future_on_large_stack("primary-stream-ticket-lifecycle", || {
            controller_api_differential_primary_stream_ticket_lifecycle_impl()
        });
        controller_api_differential_port_forwarding().await;
        controller_api_differential_security_reputation().await;
        controller_api_differential_realm_subject_indexes().await;
        controller_api_differential_musicbrainz_overlay_export().await;
        controller_api_differential_pod_membership_workflow().await;
        controller_api_differential_pod_channel_messages().await;
        controller_api_differential_podcore_message_storage().await;
        controller_api_differential_podcore_membership_storage().await;
        controller_api_differential_podcore_discovery_storage().await;
        controller_api_differential_podcore_join_leave_residuals().await;
        controller_api_differential_security_ban_residuals().await;
        controller_api_differential_security_diagnostics_residuals().await;
        controller_api_differential_soulseek_discovery_residuals().await;
    });
}

#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-3"
))]
pub(super) fn run_bounded_controller_api_tests_3() {
    run_bounded_future(async {
        controller_api_differential_multisource_residuals().await;
        controller_api_differential_podcore_route_value_validation().await;
        controller_api_differential_podcore_request_validation().await;
        controller_api_differential_hashdb_history_backfill().await;
        controller_api_differential_user_group().await;
        controller_api_differential_virtual_soulfind_v2().await;
        controller_api_differential_virtual_soulfind_v2_residuals().await;
        controller_api_differential_source_discovery().await;
        controller_api_differential_discovery_open_cases().await;
        controller_api_differential_pod_and_jury_stats().await;
        controller_api_differential_playback_feedback_and_diagnostics().await;
        controller_api_differential_nowplaying_delete_and_playback_diagnostics_edge_states().await;
        controller_api_differential_native_nowplaying_webhook_contracts().await;
        controller_api_differential_activitypub_inbox_relationships();
        controller_api_differential_activitypub_outbox_and_undo();
        controller_api_differential_conversations_delete_survives_persistence_failure().await;
        controller_api_differential_spotify_oauth_authorize_and_callback().await;
        controller_api_differential_spotify_connection_status_and_disconnect().await;
        controller_api_differential_mesh_http_gateway().await;
        controller_api_differential_solid_status_and_webid_resolution().await;
        controller_api_differential_pod_membership_self_publish().await;
        controller_api_differential_pod_membership_moderation_publish().await;
        controller_api_differential_transfer_reports_required_direction().await;
        controller_api_differential_transfer_download_cancel().await;
        controller_api_differential_transfer_upload_cancel().await;
        controller_api_differential_analyzer_hashdb_and_telemetry().await;
        controller_api_differential_source_provider_catalog().await;
        controller_api_differential_source_provider_edge_contracts().await;
        controller_api_differential_versioned_soulseek_recommendations().await;
        controller_api_differential_versioned_soulseek_item_discovery().await;
        controller_api_differential_versioned_soulseek_similar_users().await;
        controller_api_differential_versioned_autoreplace_status().await;
        controller_api_differential_versioned_autoreplace_populated_state().await;
        controller_api_differential_native_autoreplace_edge_contracts().await;
        controller_api_differential_native_options_edge_contracts().await;
        controller_api_differential_native_events_edge_contracts().await;
        controller_api_differential_versioned_mesh_health_and_signals().await;
        controller_api_differential_versioned_signals_edge_contracts().await;
        controller_api_differential_traces_summary_contracts().await;
        controller_api_differential_compatibility_info_and_fairness_contracts().await;
        controller_api_differential_compatibility_user_browse_contracts().await;
        controller_api_differential_build_info_and_file_delete().await;
        controller_api_differential_application_version_state_contracts().await;
        controller_api_differential_application_populated_versioned_state().await;
        controller_api_differential_populated_compatibility_status_and_capabilities().await;
        controller_api_differential_versioned_capability_peer_projections().await;
        controller_api_differential_versioned_capability_peers_nominal().await;
        controller_api_differential_native_capabilities_contracts().await;
        controller_api_differential_native_profile_contracts().await;
        controller_api_differential_versioned_destinations_nominal().await;
        controller_api_differential_versioned_backfill_candidates_nominal().await;
        controller_api_differential_versioned_multisource_jobs_nominal().await;
        controller_api_differential_versioned_swarm_trends_nominal().await;
        controller_api_differential_versioned_swarm_dashboard_nominal().await;
        controller_api_differential_versioned_transfer_history_nominal().await;
        controller_api_differential_versioned_transfer_empty_lists_nominal().await;
        controller_api_differential_versioned_transfer_changes_nominal().await;
        controller_api_differential_versioned_transfer_summary_nominal().await;
        controller_api_differential_versioned_transfer_histogram_nominal().await;
        controller_api_differential_versioned_transfer_reports_populated_state().await;
        controller_api_differential_versioned_destinations_populated_state().await;
        controller_api_differential_native_destinations_edge_contracts().await;
        controller_api_differential_runtime_failure_security_and_shares().await;
        controller_api_differential_sharegroups_and_shares_rebuild().await;
        controller_api_differential_sharegroups_persistence_and_concurrency().await;
        controller_api_differential_options_and_conversations_projection().await;
        controller_api_differential_options_current_overlay_lifecycle().await;
    });
}

#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
pub(super) fn run_bounded_controller_api_tests_4() {
    run_bounded_future(async {
        controller_api_differential_spotify_pods_mesh_and_party().await;
        controller_api_differential_application_interests_bridge_mediacore_adversarial().await;
        controller_api_differential_storage_and_server().await;
        controller_api_differential_searches_dispatch_and_visualizer_launch().await;
        controller_api_differential_options_dht_and_bridge_redaction().await;
        controller_api_differential_mediacore_delete_and_mesh_tickets().await;
        controller_api_differential_lidarr_and_source_feed_contracts().await;
        controller_api_differential_source_feed_open_cases().await;
        controller_api_differential_controller_file_transfer_and_room_contracts().await;
        controller_api_differential_controller_file_application_and_roster_edges().await;
        controller_api_differential_controller_share_and_relay_lifecycle().await;
        controller_api_differential_controller_fixed_route_malformed_paths().await;
        controller_api_differential_controller_parameterized_malformed_paths().await;
        controller_api_differential_controller_empty_and_missing_state().await;
        controller_api_differential_controller_relay_controller_routes().await;
        controller_api_differential_controller_core_application_session_events_and_telemetry()
            .await;
        controller_api_differential_controller_core_failure_restart_and_empty_contracts().await;
        controller_api_differential_controller_users_and_shares().await;
        controller_api_differential_controller_rooms_and_conversations().await;
        controller_api_differential_controller_rooms_conversations_restart_and_failure().await;
        controller_api_differential_controller_server_state_and_lifecycle().await;
        controller_api_differential_controller_search_lifecycle().await;
        controller_api_differential_controller_search_failure_restart_and_idempotency().await;
        controller_api_differential_native_search_compatibility_contracts().await;
        controller_api_differential_native_searches_open_cases().await;
        controller_api_differential_controller_upload_lifecycle().await;
        controller_api_differential_controller_transfer_failure_restart_and_idempotency().await;
        controller_api_differential_controller_transfer_batch_cleanup_and_failures().await;
        controller_api_differential_controller_user_browse_contracts().await;
        controller_api_differential_controller_download_edge_contracts().await;
        controller_api_differential_controller_runtime_failure_isolation_contracts().await;
        controller_api_differential_native_security_runtime_failure_contracts().await;
        controller_api_differential_controller_options_overlay_contracts().await;
        controller_api_differential_native_transfers_runtime_failure_contracts().await;
        controller_api_differential_native_transfers_empty_and_missing_contracts().await;
        controller_api_differential_native_transfers_malformed_contracts().await;
        controller_api_differential_native_transfers_nominal_populated_contracts().await;
        controller_api_differential_native_transfers_restart_and_concurrency().await;
        controller_api_differential_options_action_routes().await;
        controller_api_differential_controller_residual_core_contracts();
        controller_api_differential_hashdb_domain_contracts().await;
        controller_api_differential_pods_controller_residuals().await;
        controller_api_differential_wishlist_controller_residuals().await;
        controller_api_differential_virtual_soulfind_legacy_residuals().await;
        controller_api_differential_rooms_controller_residuals().await;
        controller_api_differential_bridge_controller_residuals().await;
        run_controller_future_on_large_stack("podcore-residuals", || {
            controller_api_differential_podcore_residuals_impl()
        });
        controller_api_differential_mediacore_residuals().await;
        controller_api_differential_musicbrainz_residuals().await;
        controller_api_differential_jobs_residuals().await;
        controller_api_differential_library_residuals().await;
        controller_api_differential_security_controller_residuals().await;
        controller_api_differential_integrations_residuals().await;
        controller_api_differential_backfill_residuals().await;
        controller_api_differential_native_native_open_cases().await;
        controller_api_differential_audio_canonical_dedupe_and_migration().await;
        controller_api_differential_taste_recommendation_open_cases().await;
        controller_api_differential_songid_open_cases().await;
        controller_api_differential_share_grants_open_cases().await;
        controller_api_differential_shares_open_cases().await;
        controller_api_differential_users_open_cases().await;
        controller_api_differential_telemetry_open_cases().await;
        run_controller_future_on_large_stack("relay-open-cases", || {
            controller_api_differential_relay_open_cases_impl()
        });
        controller_api_differential_conversations_open_cases().await;
        controller_api_differential_downloads_open_cases().await;
        controller_api_differential_files_open_cases().await;
    });
}

#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1",
    feature = "bounded-controller-api-tests-2",
    feature = "bounded-controller-api-tests-3",
    feature = "bounded-controller-api-tests-4"
))]
pub(super) fn run_bounded_controller_api_tests() {
    #[cfg(feature = "bounded-controller-api-tests")]
    {
        run_bounded_controller_api_tests_1();
        run_bounded_controller_api_tests_2();
        run_bounded_controller_api_tests_3();
        run_bounded_controller_api_tests_4();
    }
    #[cfg(all(
        not(feature = "bounded-controller-api-tests"),
        feature = "bounded-controller-api-tests-1"
    ))]
    run_bounded_controller_api_tests_1();
    #[cfg(all(
        not(feature = "bounded-controller-api-tests"),
        feature = "bounded-controller-api-tests-2"
    ))]
    run_bounded_controller_api_tests_2();
    #[cfg(all(
        not(feature = "bounded-controller-api-tests"),
        feature = "bounded-controller-api-tests-3"
    ))]
    run_bounded_controller_api_tests_3();
    #[cfg(all(
        not(feature = "bounded-controller-api-tests"),
        feature = "bounded-controller-api-tests-4"
    ))]
    run_bounded_controller_api_tests_4();
}

#[cfg(feature = "bounded-persistence-tests")]
pub(super) fn run_bounded_persistence_tests() {
    run_bounded_future(async {
        persistence_lifecycle_differential_search_event_transfer_message_domains_roundtrip_and_rehydrate().await;
        persistence_lifecycle_differential_transfers_domain_rehydrates_from_sqlite().await;
        persistence_lifecycle_differential_collections_notes_wishlist_sharing_domains_roundtrip_and_rehydrate().await;
        persistence_lifecycle_differential_wishlist_ignored_results_domain().await;
        persistence_lifecycle_differential_pod_core_file_state().await;
        persistence_lifecycle_differential_collections_notes_wishlist_sharing_domains_update_delete_and_readback().await;
        persistence_lifecycle_differential_search_and_message_domains_update_delete_and_readback()
            .await;
        persistence_lifecycle_differential_covered_domains_schema_create_and_migrate().await;
        persistence_lifecycle_differential_covered_domains_transaction_and_concurrency_atomicity()
            .await;
        persistence_lifecycle_differential_covered_domains_corrupt_state_and_upgrade_failure()
            .await;
        persistence_lifecycle_differential_controller_batches_domain().await;
        persistence_lifecycle_differential_controller_share_files_domain().await;
        persistence_lifecycle_differential_transfers_domain_full_lifecycle().await;
        persistence_lifecycle_differential_native_hashdb_domains().await;
        persistence_lifecycle_differential_native_songid_runs().await;
        persistence_lifecycle_differential_native_traffic_stats_domain().await;
    });
}

#[cfg(feature = "bounded-file-lifecycle-tests")]
pub(super) fn run_bounded_file_lifecycle_tests() {
    run_bounded_future(async {
        file_lifecycle_differential_options_controller_backup_and_reload().await;
        file_lifecycle_differential_options_controller_rejects_backup_symlink().await;
        file_lifecycle_differential_files_service_roots_and_metadata().await;
        file_lifecycle_differential_download_service_path_and_retry().await;
        file_lifecycle_differential_relay_agent_download_cleanup_and_reload().await;
        file_lifecycle_differential_secure_file_writer_download_open();
        file_lifecycle_differential_dht_certificate_manager_identity_files().await;
        file_lifecycle_differential_atomic_file_writer_common_cases();
        file_lifecycle_differential_mesh_certificate_pin_manager().await;
        file_lifecycle_differential_gold_star_club_revocation();
        file_lifecycle_differential_multisource_download_service().await;
    });
}

#[cfg(feature = "bounded-protocol-tests")]
pub(super) fn run_bounded_protocol_tests() {
    run_bounded_future(async {
        protocol_behaviors_differential_overlay_gateway_mesh_search().await;
        protocol_behaviors_differential_mesh_sync_private_runtime().await;
        protocol_behaviors_differential_virtual_soulfind_bridge_round_trips().await;
        protocol_behaviors_differential_virtual_soulfind_bridge_raw_frames().await;
        protocol_behaviors_differential_virtual_soulfind_bridge_dispatch().await;
        protocol_behaviors_differential_virtual_soulfind_bridge_malformed_frames().await;
    });
}

#[cfg(feature = "bounded-security-control-tests")]
pub(super) fn run_bounded_security_control_tests() {
    run_bounded_future(async {
        security_controls_differential_reputation_and_violation_runtime().await;
        security_controls_differential_path_and_file_guards();
        security_controls_differential_share_token_store();
        run_controller_future_on_large_stack(
            "security-controls-csrf-filter-bounded",
            security_controls_differential_csrf_filter_impl,
        );
        security_controls_differential_hardening_validator();
        security_controls_differential_certificate_manager().await;
        security_controls_differential_overlay_message_validation().await;
        security_controls_differential_solid_fetch_policy().await;
        security_controls_differential_mesh_surface().await;
        security_controls_differential_content_safety().await;
        security_controls_differential_soulseek_safety();
        security_controls_differential_security_event_sink();
        security_controls_differential_integrity_controls();
        security_controls_differential_runtime_controls();
        security_controls_differential_route_security_adapters().await;
        security_controls_differential_mesh_transport().await;
        security_controls_differential_core_security().await;
        security_controls_differential_native_security_controller().await;
        security_controls_differential_passthrough_authentication();
        security_controls_differential_authentication_and_jwt();
    });
}

#[cfg(feature = "bounded-security-authorization-tests")]
pub(super) fn run_bounded_security_authorization_tests() {
    security_authorization_matrix_matches_declared_policy_for_every_frozen_route();
}

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
pub fn run_bounded_differential_tests() {
    #[cfg(any(
        feature = "bounded-controller-api-tests",
        feature = "bounded-controller-api-tests-1",
        feature = "bounded-controller-api-tests-2",
        feature = "bounded-controller-api-tests-3",
        feature = "bounded-controller-api-tests-4"
    ))]
    run_bounded_controller_api_tests();
    #[cfg(feature = "bounded-persistence-tests")]
    run_bounded_persistence_tests();
    #[cfg(feature = "bounded-file-lifecycle-tests")]
    run_bounded_file_lifecycle_tests();
    #[cfg(feature = "bounded-protocol-tests")]
    run_bounded_protocol_tests();
    #[cfg(feature = "bounded-security-control-tests")]
    run_bounded_security_control_tests();
    #[cfg(feature = "bounded-security-authorization-tests")]
    run_bounded_security_authorization_tests();
}
