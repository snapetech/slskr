use super::*;

pub(crate) async fn podcore_stats_response(
    path: &str,
    query: Option<&str>,
    state: &AppState,
    versioned_v0: bool,
) -> HttpResponse {
    let pods = state.pods.read().await;
    let membership_stats = pods.membership_stats();
    let pod_messages = state.pod_channels.read().await;
    let message_stats = pod_messages.stats();
    match path {
        "/api/podcore/discovery/all" => {
            if state
                .controller_features
                .read()
                .await
                .validate_storage()
                .is_err()
            {
                return routing::internal_server_error_response("Failed to discover pods");
            }
            let started_at = std::time::Instant::now();
            let limit = query_parameter(query, "limit")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(50);
            if limit == 0 || limit > 100 {
                return routing::bad_request_response("Limit must be between 1 and 100");
            }
            let records = state
                .controller_features
                .read()
                .await
                .values_with_prefix("pod/discovery/")
                .into_iter()
                .filter(|record| pod_discovery_record_is_active(record, chrono::Utc::now()))
                .collect::<Vec<_>>();
            let rows = pod_discovery_rows(&records, &pods, &["pod:discover:all".to_owned()])
                .into_iter()
                .take(limit)
                .collect::<Vec<_>>();
            state
                .podcore_runtime_stats
                .record_discovery_search("all", started_at.elapsed().as_millis() as u64);
            routing::ok_response(
                serde_json::json!({
                    "pods": rows,
                    "searchType": "all",
                    "searchTerm": limit.to_string(),
                    "totalFound": rows.len(),
                    "searchedAt": chrono::Utc::now().to_rfc3339(),
                })
                .to_string(),
            )
        }
        "/api/podcore/discovery/stats" => {
            if state
                .controller_features
                .read()
                .await
                .validate_storage()
                .is_err()
            {
                return routing::internal_server_error_response(
                    "Failed to get discovery statistics",
                );
            }
            let now = chrono::Utc::now();
            let discovery_records = state
                .controller_features
                .read()
                .await
                .values_with_prefix("pod/discovery/");
            let mut active_entries = 0_u64;
            let mut expired_entries = 0_u64;
            let mut registrations_by_tag = BTreeMap::<String, u64>::new();
            let mut last_recorded_operation: Option<chrono::DateTime<chrono::Utc>> = None;
            for record in &discovery_records {
                let discovery_key_count = record
                    .get("discoveryKeys")
                    .and_then(serde_json::Value::as_array)
                    .map_or(1, Vec::len) as u64;
                let expires_at = record
                    .get("expiresAt")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                    .map(|value| value.with_timezone(&chrono::Utc));
                if expires_at.is_none_or(|expires_at| expires_at > now) {
                    active_entries += discovery_key_count;
                } else {
                    expired_entries += discovery_key_count;
                }
                if let Some(registered_at) = record
                    .get("registeredAt")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                {
                    let registered_at = registered_at.with_timezone(&chrono::Utc);
                    if last_recorded_operation.is_none_or(|current| registered_at > current) {
                        last_recorded_operation = Some(registered_at);
                    }
                }
                if let Some(tags) = record.get("tags").and_then(serde_json::Value::as_array) {
                    for tag in tags.iter().filter_map(serde_json::Value::as_str) {
                        *registrations_by_tag.entry(tag.to_owned()).or_default() += 1;
                    }
                }
            }
            let runtime = &state.podcore_runtime_stats;
            let searches = runtime
                .discovery_searches_by_type
                .lock()
                .map(|searches| searches.clone())
                .unwrap_or_default();
            let search_count = runtime
                .discovery_searches
                .load(std::sync::atomic::Ordering::Relaxed);
            let search_duration_ms = runtime
                .total_discovery_search_time_ms
                .load(std::sync::atomic::Ordering::Relaxed);
            let last_operation = runtime
                .last_discovery_operation
                .lock()
                .ok()
                .and_then(|operation| operation.clone())
                .or_else(|| last_recorded_operation.map(|operation| operation.to_rfc3339()))
                .unwrap_or_else(|| PODCORE_MIN_DATETIME.to_owned());
            routing::ok_response(
                serde_json::json!({
                    "totalRegisteredPods": discovery_records.len(),
                    "activeDiscoveryEntries": active_entries,
                    "expiredEntries": expired_entries,
                    "registrationsByTag": registrations_by_tag,
                    "searchesByType": searches,
                    "lastDiscoveryOperation": last_operation,
                    "averageDiscoveryTime": format_timespan_millis(
                        search_duration_ms / search_count.max(1),
                    ),
                })
                .to_string(),
            )
        }
        "/api/podcore/content/metadata" => {
            // Matches the oracle's PodContentController.GetContentMetadata
            // + ContentLinkService.CreateBasicMetadata: requires a real
            // contentId query parameter and returns a single content
            // object shaped by its `content:<domain>:<type>:<id>` parts,
            // not an unrelated pod/channel-count aggregate that ignored
            // the query entirely. Audio metadata is enriched through the
            // configured MusicBrainz client; other domains use the real
            // basic metadata projection until their provider is available.
            let content_id = query_parameter(query, "contentId")
                .map(|value| value.trim().to_owned())
                .unwrap_or_default();
            if content_id.is_empty() {
                return routing::bad_request_response("Content ID is required");
            }
            let Some(parsed) = parse_podcore_content_id(&content_id) else {
                return HttpResponse {
                    status: "404 Not Found",
                    content_type: "application/json",
                    body: serde_json::json!("Content not found").to_string(),
                };
            };
            let metadata = if parsed.domain_lower == "audio" {
                let settings = state.integration_settings.read().await.musicbrainz.clone();
                match podcore_audio_metadata(&settings, &parsed).await {
                    Ok(metadata) => metadata,
                    Err(_) => {
                        return HttpResponse {
                            status: "404 Not Found",
                            content_type: "application/json",
                            body: serde_json::json!("Content not found").to_string(),
                        };
                    }
                }
            } else {
                fallback_podcore_metadata(&parsed)
            };
            routing::ok_response(metadata.to_string())
        }
        "/api/podcore/membership/stats" => {
            if pods.validate_storage().is_err() {
                return routing::internal_server_error_response(
                    "Failed to get membership statistics",
                );
            }
            routing::ok_response(
                serde_json::json!({
                    "totalMemberships": membership_stats.total_memberships,
                    "activeMemberships": membership_stats.active_memberships,
                    "bannedMemberships": membership_stats.banned_memberships,
                    "expiredMemberships": membership_stats.expired_memberships,
                    "membershipsByRole": membership_stats.memberships_by_role,
                    "membershipsByPod": membership_stats.memberships_by_pod,
                    "lastOperation": membership_stats
                        .last_operation
                        .unwrap_or_else(|| PODCORE_MIN_DATETIME.to_owned()),
                })
                .to_string(),
            )
        }
        "/api/podcore/dht/stats" => {
            let runtime = &state.podcore_runtime_stats;
            let active_publications = runtime
                .dht_active_publications
                .load(std::sync::atomic::Ordering::Relaxed);
            let expired_publications = runtime
                .dht_expired_publications
                .load(std::sync::atomic::Ordering::Relaxed);
            let publications_by_domain = runtime
                .dht_publications_by_domain
                .lock()
                .map(|values| values.clone())
                .unwrap_or_default();
            let publications_by_visibility = runtime
                .dht_publications_by_visibility
                .lock()
                .map(|values| values.clone())
                .unwrap_or_default();
            let last_publish_operation = runtime
                .dht_last_publish_operation
                .lock()
                .ok()
                .and_then(|value| value.clone())
                .unwrap_or_else(|| PODCORE_MIN_DATETIME.to_owned());
            let total_published = state
                .pod_dht_publish_count
                .load(std::sync::atomic::Ordering::Relaxed);
            let failed_publications = state
                .pod_dht_failed_publish_count
                .load(std::sync::atomic::Ordering::Relaxed);
            let total_publish_time_ms = state
                .pod_dht_publish_time_ms
                .load(std::sync::atomic::Ordering::Relaxed);
            let average_publish_time = total_publish_time_ms
                .checked_div(total_published)
                .unwrap_or_default();
            routing::ok_response(
                serde_json::json!({
                    "totalPublished": total_published,
                    "activePublications": active_publications,
                    "expiredPublications": expired_publications,
                    "failedPublications": failed_publications,
                    "averagePublishTime": format_timespan_millis(average_publish_time),
                    "publicationsByDomain": publications_by_domain,
                    "publicationsByVisibility": publications_by_visibility,
                    "lastPublishOperation": last_publish_operation,
                })
                .to_string(),
            )
        }
        "/api/podcore/backfill/stats" => {
            let runtime = &state.podcore_runtime_stats;
            let requests = runtime
                .backfill_requests
                .load(std::sync::atomic::Ordering::Relaxed);
            let received = runtime
                .backfill_requests_received
                .load(std::sync::atomic::Ordering::Relaxed);
            let total_duration_ms = runtime
                .total_backfill_duration_ms
                .load(std::sync::atomic::Ordering::Relaxed);
            let requests_by_pod = runtime
                .backfill_requests_by_pod
                .lock()
                .map(|requests| requests.clone())
                .unwrap_or_default();
            let last_operation = runtime
                .last_backfill_operation
                .lock()
                .ok()
                .and_then(|operation| operation.clone());
            let average_duration_ms = if requests == 0 {
                0.0
            } else {
                total_duration_ms as f64 / requests as f64
            };
            routing::ok_response(
                serde_json::json!({
                    "totalBackfillRequestsSent": requests,
                    "totalBackfillRequestsReceived": received,
                    "totalMessagesBackfilled": runtime.messages_backfilled.load(std::sync::atomic::Ordering::Relaxed),
                    "totalBackfillBytesTransferred": runtime.backfill_bytes_transferred.load(std::sync::atomic::Ordering::Relaxed),
                    "averageBackfillDurationMs": average_duration_ms,
                    "backfillRequestsByPod": requests_by_pod,
                    "lastBackfillOperation": last_operation
                        .unwrap_or_else(|| PODCORE_MIN_DATETIME.to_owned()),
                })
                .to_string(),
            )
        }
        "/api/podcore/messages/stats" => {
            if pod_messages.validate_storage().is_err() {
                return routing::internal_server_error_response(
                    "An error occurred while getting storage statistics",
                );
            }
            let mut value = serde_json::json!({
                "totalMessages": message_stats.total_messages,
                "totalSizeBytes": message_stats.total_messages.saturating_mul(200),
                "messagesPerPod": message_stats.messages_per_pod,
                "messagesPerChannel": message_stats.messages_per_channel,
            });
            if let Some(timestamp) = message_stats.oldest_timestamp_unix_ms {
                value["oldestMessage"] = chrono::DateTime::from_timestamp_millis(
                    i64::try_from(timestamp).unwrap_or(i64::MAX),
                )
                .map(|value| serde_json::Value::String(value.to_rfc3339()))
                .unwrap_or(serde_json::Value::Null);
            }
            if let Some(timestamp) = message_stats.newest_timestamp_unix_ms {
                value["newestMessage"] = chrono::DateTime::from_timestamp_millis(
                    i64::try_from(timestamp).unwrap_or(i64::MAX),
                )
                .map(|value| serde_json::Value::String(value.to_rfc3339()))
                .unwrap_or(serde_json::Value::Null);
            }
            routing::ok_response(value.to_string())
        }
        "/api/podcore/routing/stats" => {
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && state
                    .controller_features
                    .read()
                    .await
                    .validate_storage()
                    .is_err()
            {
                return routing::internal_server_error_response("Failed to get routing statistics");
            }
            // The frozen router uses a 24-hour Bloom filter sized for 10,000
            // messages at a 1% target false-positive rate. The persisted
            // seen records are the local equivalent, so derive the same
            // filter metrics from the active item count instead of exposing
            // a permanent all-zero compatibility projection.
            const BLOOM_BITS: f64 = 95_851.0;
            const BLOOM_HASHES: f64 = 7.0;
            const ROUTING_WINDOW_MILLIS: u64 = 24 * 60 * 60 * 1_000;
            let now = unix_timestamp_millis();
            let entries = state
                .controller_features
                .read()
                .await
                .entries_with_prefix("pod/routing-seen/");
            let active_items = entries
                .iter()
                .filter(|(_, value)| {
                    value
                        .get("seenAt")
                        .and_then(serde_json::Value::as_u64)
                        .is_some_and(|seen_at| seen_at.saturating_add(ROUTING_WINDOW_MILLIS) > now)
                })
                .count();
            let fill_ratio = 1.0 - (-BLOOM_HASHES * active_items as f64 / BLOOM_BITS).exp();
            let false_positive_rate = fill_ratio.powf(BLOOM_HASHES);
            let runtime = &state.podcore_runtime_stats;
            let total_messages = runtime
                .routing_messages
                .load(std::sync::atomic::Ordering::Relaxed);
            let total_time_ms = runtime
                .total_routing_time_ms
                .load(std::sync::atomic::Ordering::Relaxed);
            let last_operation = runtime
                .last_routing_operation
                .lock()
                .ok()
                .and_then(|operation| operation.clone())
                .unwrap_or_else(|| PODCORE_MIN_DATETIME.to_owned());
            let routing_by_pod = runtime
                .routing_messages_by_pod
                .lock()
                .map(|messages| messages.clone())
                .unwrap_or_default();
            routing::ok_response(
                serde_json::json!({
                    "totalMessagesRouted": total_messages,
                    "totalRoutingAttempts": runtime
                        .routing_attempts
                        .load(std::sync::atomic::Ordering::Relaxed),
                    "successfulRoutingCount": runtime
                        .routing_successes
                        .load(std::sync::atomic::Ordering::Relaxed),
                    "failedRoutingCount": runtime
                        .routing_failures
                        .load(std::sync::atomic::Ordering::Relaxed),
                    "averageRoutingTimeMs": if total_messages == 0 {
                        0.0
                    } else {
                        total_time_ms as f64 / total_messages as f64
                    },
                    "activeDeduplicationItems": active_items,
                    "bloomFilterFillRatio": fill_ratio,
                    "estimatedFalsePositiveRate": false_positive_rate,
                    "lastRoutingOperation": last_operation,
                    "routingStatsByPod": routing_by_pod,
                })
                .to_string(),
            )
        }
        "/api/podcore/signing/stats" => {
            use std::sync::atomic::Ordering;
            let stats = &state.pod_signature_stats;
            let signatures_created = stats.signatures_created.load(Ordering::Relaxed);
            let signatures_verified = stats.signatures_verified.load(Ordering::Relaxed);
            let successful_verifications = stats.successful_verifications.load(Ordering::Relaxed);
            let failed_verifications = stats.failed_verifications.load(Ordering::Relaxed);
            let total_signing_time_ms = stats.total_signing_time_ms.load(Ordering::Relaxed);
            let total_verification_time_ms =
                stats.total_verification_time_ms.load(Ordering::Relaxed);
            let last_operation_at = stats
                .last_operation_at
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            routing::ok_response(
                serde_json::json!({
                    "totalSignaturesCreated": signatures_created,
                    "totalSignaturesVerified": signatures_verified,
                    "successfulVerifications": successful_verifications,
                    "failedVerifications": failed_verifications,
                    "averageSigningTimeMs": total_signing_time_ms as f64 / signatures_created.max(1) as f64,
                    "averageVerificationTimeMs": total_verification_time_ms as f64 / signatures_verified.max(1) as f64,
                    "lastSignatureOperation": last_operation_at
                        .unwrap_or_else(|| PODCORE_MIN_DATETIME.to_owned()),
                })
                .to_string(),
            )
        }
        "/api/podcore/verification/stats" => {
            use std::sync::atomic::Ordering;
            let stats = &state.pod_verification_stats;
            let total_verifications = stats.total_verifications.load(Ordering::Relaxed);
            let successful_verifications = stats.successful_verifications.load(Ordering::Relaxed);
            let failed_membership_checks = stats.failed_membership_checks.load(Ordering::Relaxed);
            let failed_signature_checks = stats.failed_signature_checks.load(Ordering::Relaxed);
            let banned_member_rejections = stats.banned_member_rejections.load(Ordering::Relaxed);
            let total_verification_time_ms =
                stats.total_verification_time_ms.load(Ordering::Relaxed);
            let last_verification = stats
                .last_verification_at
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            routing::ok_response(
                serde_json::json!({
                    "totalVerifications": total_verifications,
                    "successfulVerifications": successful_verifications,
                    "failedMembershipChecks": failed_membership_checks,
                    "failedSignatureChecks": failed_signature_checks,
                    "bannedMemberRejections": banned_member_rejections,
                    "averageVerificationTimeMs": total_verification_time_ms as f64 / total_verifications.max(1) as f64,
                    "lastVerification": last_verification
                        .unwrap_or_else(|| PODCORE_MIN_DATETIME.to_owned()),
                })
                .to_string(),
            )
        }
        _ => routing::not_found_response(),
    }
}
