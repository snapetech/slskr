use super::*;

pub(crate) async fn podcore_dynamic_get_response(
    path: &str,
    query: Option<&str>,
    state: &AppState,
    versioned_v0: bool,
) -> HttpResponse {
    let blank_route_value = |value: &str| decoded_path_segment(value).trim().is_empty();

    if let Some(rest) = path.strip_prefix("/api/podcore/backfill/") {
        let segments = rest.split('/').collect::<Vec<_>>();
        if segments.len() == 2 && segments[1] == "last-seen" && blank_route_value(segments[0]) {
            return routing::bad_request_response("Pod ID is required");
        }
    }
    if let Some(rest) = path.strip_prefix("/api/podcore/messages/") {
        let segments = rest.split('/').collect::<Vec<_>>();
        if segments.len() == 3 && segments[2] == "count" {
            if blank_route_value(segments[0]) {
                return routing::bad_request_response("Pod ID is required");
            }
            if blank_route_value(segments[1]) {
                return routing::bad_request_response("Channel ID is required");
            }
        }
    }
    if let Some(rest) = path.strip_prefix("/api/podcore/membership/") {
        let segments = rest.split('/').collect::<Vec<_>>();
        if segments.len() == 2 && (blank_route_value(segments[0]) || blank_route_value(segments[1]))
        {
            return routing::bad_request_response("PodId and PeerId are required");
        }
        if segments.len() == 3
            && segments[2] == "verify"
            && (blank_route_value(segments[0]) || blank_route_value(segments[1]))
        {
            return routing::bad_request_response("PodId and PeerId are required");
        }
    }
    if let Some(rest) = path.strip_prefix("/api/podcore/") {
        let segments = rest.split('/').collect::<Vec<_>>();
        if segments.len() >= 2 && segments[1] == "channels" && blank_route_value(segments[0]) {
            return routing::bad_request_response("Pod ID is required");
        }
        if segments.len() == 3 && segments[1] == "channels" && blank_route_value(segments[2]) {
            return routing::bad_request_response("Channel ID is required");
        }
        if segments.len() >= 4 && segments[1] == "opinions" {
            if blank_route_value(segments[0]) {
                return routing::bad_request_response("Pod ID is required");
            }
            if segments[2] == "content" && blank_route_value(segments[3]) {
                return routing::bad_request_response("Content ID is required");
            }
            if segments.len() == 6
                && segments[2] == "content"
                && segments[4] == "variant"
                && blank_route_value(segments[5])
            {
                return routing::bad_request_response("Variant hash is required");
            }
        }
    }
    if let Some(rest) = path.strip_prefix("/api/podcore/membership/") {
        let segments = rest.split('/').collect::<Vec<_>>();
        if segments.len() == 3
            && matches!(segments[0], "join" | "leave")
            && segments[1] == "pending"
            && blank_route_value(segments[2])
        {
            return routing::bad_request_response("PodId is required");
        }
    }
    if let Some(rest) = path.strip_prefix("/api/podcore/routing/") {
        let segments = rest.split('/').collect::<Vec<_>>();
        if segments.len() == 3
            && segments[0] == "seen"
            && (blank_route_value(segments[1]) || blank_route_value(segments[2]))
        {
            return routing::bad_request_response("MessageId and PodId are required");
        }
    }

    // The ASP.NET discovery controller binds these route values as required
    // strings and returns 400 when the catch-all/query value is blank. The
    // direct router still classifies a trailing slash as a dynamic route, so
    // preserve that validation instead of letting decoded-segment parsing
    // turn it into a generic 404.
    match path {
        "/api/podcore/dht/metadata/" => {
            return routing::bad_request_response("Pod ID is required");
        }
        "/api/podcore/discovery/content/" => {
            return routing::bad_request_response("ContentId is required");
        }
        "/api/podcore/discovery/name/" => {
            return routing::bad_request_response(
                "Name is required and must be within length limits",
            );
        }
        "/api/podcore/discovery/tag/" => {
            return routing::bad_request_response(
                "Tag is required and must be within length limits",
            );
        }
        "/api/podcore/discovery/tags/" => {
            return routing::bad_request_response(
                "Tags are required and must be within length limits",
            );
        }
        _ => {}
    }
    let Some(segments) = decoded_segments_after(path, "/api/podcore/") else {
        return routing::not_found_response();
    };
    match segments.as_slice() {
        [pod_id, section] if section == "channels" => {
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && state.pods.read().await.validate_storage().is_err()
            {
                return routing::internal_server_error_response(
                    "An error occurred while getting channels",
                );
            }
            let pods = state.pods.read().await;
            pods.get(pod_id)
                .map_or_else(routing::not_found_response, |pod| {
                    routing::ok_response(
                        serde_json::Value::Array(
                            pod.channels
                                .into_iter()
                                .map(|channel| serde_json::json!(channel))
                                .collect(),
                        )
                        .to_string(),
                    )
                })
        }
        [pod_id, section, channel_id] if section == "channels" => {
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && state.pods.read().await.validate_storage().is_err()
            {
                return routing::internal_server_error_response(
                    "An error occurred while getting the channel",
                );
            }
            let pods = state.pods.read().await;
            let channel = pods.get(pod_id).and_then(|pod| {
                pod.channels
                    .into_iter()
                    .find(|channel| channel.channel_id == *channel_id)
            });
            channel.map_or_else(routing::not_found_response, |channel| {
                routing::ok_response(serde_json::json!(channel).to_string())
            })
        }
        [pod_id, section, content, content_id, tail @ ..]
            if section == "opinions" && content == "content" =>
        {
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && state
                    .controller_features
                    .read()
                    .await
                    .validate_storage()
                    .is_err()
            {
                return routing::internal_server_error_response(
                    "An error occurred while getting opinions",
                );
            }
            let opinions = state
                .controller_features
                .read()
                .await
                .values_with_prefix(&format!("pod/opinion/{pod_id}/{content_id}/"));
            let discovery = state.content_discovery.read().await;
            let variants = discovery
                .hash_entries()
                .iter()
                .filter(|entry| entry.music_brainz_id.eq_ignore_ascii_case(content_id))
                .count();
            let peers = discovery.peer_ids_for_recordings(std::slice::from_ref(content_id));
            drop(discovery);
            let member_affinities = pod_member_affinities_json(state, pod_id).await;
            let scores = opinions
                .iter()
                .filter_map(|opinion| {
                    opinion
                        .get("score")
                        .or_else(|| opinion.get("rating"))
                        .and_then(serde_json::Value::as_f64)
                })
                .collect::<Vec<_>>();
            let average = if scores.is_empty() {
                0.0
            } else {
                scores.iter().sum::<f64>() / scores.len() as f64
            };
            let mut variant_groups = BTreeMap::<String, Vec<serde_json::Value>>::new();
            let mut member_contributions = BTreeMap::<String, f64>::new();
            let mut total_weighted_score = 0.0;
            let mut total_weight = 0.0;
            for opinion in &opinions {
                let variant_hash = opinion
                    .get("variantHash")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let sender = opinion
                    .get("senderPeerId")
                    .or_else(|| opinion.get("issuer"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let score = opinion
                    .get("score")
                    .or_else(|| opinion.get("rating"))
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0);
                let affinity = member_affinities
                    .get(&sender)
                    .and_then(|member| member.get("affinityScore"))
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.5)
                    .clamp(0.0, 1.0);
                *member_contributions.entry(sender).or_default() += affinity;
                total_weighted_score += score * affinity;
                total_weight += affinity;
                variant_groups
                    .entry(variant_hash)
                    .or_default()
                    .push(opinion.clone());
            }
            let weighted_average = if total_weight > 0.0 {
                total_weighted_score / total_weight
            } else {
                0.0
            };
            let variant_averages = variant_groups
                .values()
                .map(|group| {
                    group
                        .iter()
                        .filter_map(|opinion| {
                            opinion
                                .get("score")
                                .or_else(|| opinion.get("rating"))
                                .and_then(serde_json::Value::as_f64)
                        })
                        .sum::<f64>()
                        / group.len().max(1) as f64
                })
                .collect::<Vec<_>>();
            let variant_average = if variant_averages.is_empty() {
                0.0
            } else {
                variant_averages.iter().sum::<f64>() / variant_averages.len() as f64
            };
            let variant_variance = variant_averages
                .iter()
                .map(|value| (value - variant_average).powi(2))
                .sum::<f64>();
            let variant_standard_deviation = if variant_averages.len() <= 1 {
                0.0
            } else {
                (variant_variance / (variant_averages.len() - 1) as f64).sqrt()
            };
            let consensus_strength = if variant_averages.is_empty() {
                0.0
            } else {
                (1.0 - variant_standard_deviation / 5.0).max(0.0)
                    * (opinions.len() as f64 / 10.0).min(1.0)
            };
            let variant_aggregates = variant_groups
                .iter()
                .map(|(variant_hash, variant_opinions)| {
                    let variant_scores = variant_opinions
                        .iter()
                        .filter_map(|opinion| {
                            opinion
                                .get("score")
                                .or_else(|| opinion.get("rating"))
                                .and_then(serde_json::Value::as_f64)
                        })
                        .collect::<Vec<_>>();
                    let variant_average =
                        variant_scores.iter().sum::<f64>() / variant_scores.len().max(1) as f64;
                    let variant_variance = variant_scores
                        .iter()
                        .map(|value| (value - variant_average).powi(2))
                        .sum::<f64>();
                    let score_standard_deviation = if variant_scores.len() <= 1 {
                        0.0
                    } else {
                        (variant_variance / (variant_scores.len() - 1) as f64).sqrt()
                    };
                    let weighted_opinions = variant_opinions
                        .iter()
                        .map(|opinion| {
                            let sender = opinion
                                .get("senderPeerId")
                                .or_else(|| opinion.get("issuer"))
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default();
                            let affinity = member_affinities
                                .get(sender)
                                .and_then(|member| member.get("affinityScore"))
                                .and_then(serde_json::Value::as_f64)
                                .unwrap_or(0.5)
                                .clamp(0.0, 1.0);
                            let score = opinion
                                .get("score")
                                .or_else(|| opinion.get("rating"))
                                .and_then(serde_json::Value::as_f64)
                                .unwrap_or(0.0);
                            serde_json::json!({
                                "opinion": opinion,
                                "affinityWeight": affinity,
                                "weightedScore": score * affinity,
                            })
                        })
                        .collect::<Vec<_>>();
                    let affinity_weight_sum = weighted_opinions
                        .iter()
                        .filter_map(|opinion| {
                            opinion
                                .get("affinityWeight")
                                .and_then(serde_json::Value::as_f64)
                        })
                        .sum::<f64>();
                    let weighted_average_score = if affinity_weight_sum > 0.0 {
                        weighted_opinions
                            .iter()
                            .filter_map(|opinion| {
                                opinion
                                    .get("weightedScore")
                                    .and_then(serde_json::Value::as_f64)
                            })
                            .sum::<f64>()
                            / affinity_weight_sum
                    } else {
                        0.0
                    };
                    serde_json::json!({
                        "variantHash": variant_hash,
                        "weightedAverageScore": weighted_average_score,
                        "unweightedAverageScore": variant_average,
                        "opinionCount": variant_opinions.len(),
                        "scoreStandardDeviation": score_standard_deviation,
                        "affinityWeightSum": affinity_weight_sum,
                        "opinions": weighted_opinions,
                    })
                })
                .collect::<Vec<_>>();
            let value = match tail {
                [] => serde_json::Value::Array(opinions),
                [kind] if kind == "aggregated" => {
                    serde_json::json!({
                        "podId": pod_id,
                        "contentId": content_id,
                        "weightedAverageScore": weighted_average,
                        "unweightedAverageScore": average,
                        "totalOpinions": opinions.len(),
                        "uniqueVariants": variant_groups.len().max(variants),
                        "contributingMembers": member_contributions.len(),
                        "consensusStrength": consensus_strength,
                        "variantAggregates": variant_aggregates,
                        "memberContributions": member_contributions,
                        "lastUpdated": chrono::Utc::now().to_rfc3339(),
                    })
                }
                [kind] if kind == "recommendations" => serde_json::Value::Array(
                    peers.into_iter().map(serde_json::Value::String).collect(),
                ),
                [kind] if kind == "stats" => {
                    serde_json::json!({
                        "podId": pod_id,
                        "contentId": content_id,
                        "totalOpinions": opinions.len(),
                        "uniqueVariants": variant_groups.len().max(variants),
                        "averageScore": average,
                        "minScore": scores.iter().copied().reduce(f64::min).unwrap_or(0.0),
                        "maxScore": scores.iter().copied().reduce(f64::max).unwrap_or(0.0),
                        "scoreDistribution": scores.iter().fold(
                            serde_json::Map::new(),
                            |mut distribution, score| {
                                let bucket = score.floor().to_string();
                                let count = distribution
                                    .get(&bucket)
                                    .and_then(serde_json::Value::as_u64)
                                    .unwrap_or(0)
                                    + 1;
                                distribution.insert(bucket, serde_json::json!(count));
                                distribution
                            },
                        ),
                        "lastUpdated": chrono::Utc::now().to_rfc3339(),
                    })
                }
                [kind, variant_hash] if kind == "variant" => serde_json::Value::Array(
                    opinions
                        .iter()
                        .filter(|opinion| {
                            opinion
                                .get("variantHash")
                                .and_then(serde_json::Value::as_str)
                                == Some(variant_hash)
                        })
                        .cloned()
                        .collect(),
                ),
                _ => return routing::not_found_response(),
            };
            routing::ok_response(value.to_string())
        }
        [pod_id, section, members, affinity]
            if section == "opinions" && members == "members" && affinity == "affinity" =>
        {
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && state
                    .controller_features
                    .read()
                    .await
                    .validate_storage()
                    .is_err()
            {
                return routing::internal_server_error_response(
                    "An error occurred while getting member affinities",
                );
            }
            routing::ok_response(pod_member_affinities_json(state, pod_id).await.to_string())
        }
        [section, pod_id, last_seen] if section == "backfill" && last_seen == "last-seen" => {
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && state
                    .controller_features
                    .read()
                    .await
                    .validate_storage()
                    .is_err()
            {
                return routing::internal_server_error_response(
                    "An error occurred while getting last seen timestamps",
                );
            }
            let features = state.controller_features.read().await;
            let pod = state.pods.read().await.get(pod_id);
            let mut latest_by_channel = serde_json::Map::new();
            for channel in pod.map(|pod| pod.channels).unwrap_or_default() {
                if let Some(last_seen) = features
                    .get(&format!("pod/backfill/{pod_id}/{}", channel.channel_id))
                    .and_then(|value| value.get("lastSeen"))
                    .and_then(serde_json::Value::as_u64)
                {
                    latest_by_channel.insert(channel.channel_id, serde_json::json!(last_seen));
                }
            }
            routing::ok_response(serde_json::Value::Object(latest_by_channel).to_string())
        }
        [section, metadata, tail @ ..]
            if section == "dht" && metadata == "metadata" && !tail.is_empty() =>
        {
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && state
                    .controller_features
                    .read()
                    .await
                    .validate_storage()
                    .is_err()
            {
                return routing::internal_server_error_response("Failed to retrieve pod metadata");
            }
            let pod_id = tail.join("/");
            let publication = state
                .controller_features
                .read()
                .await
                .get(&format!("pod/dht/{pod_id}"))
                .cloned();
            let Some(publication) = publication else {
                return HttpResponse {
                    status: "404 Not Found",
                    content_type: "application/json",
                    body: serde_json::json!({"found": false, "error": "Pod not found"}).to_string(),
                };
            };
            let Some(published_pod) = publication.get("publishedPod").filter(|pod| !pod.is_null())
            else {
                return HttpResponse {
                    status: "404 Not Found",
                    content_type: "application/json",
                    body: serde_json::json!({"found": false, "error": "Pod not found"}).to_string(),
                };
            };
            let signature = publication
                .get("signature")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let public_key = publication
                .get("publicKey")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if !verify_pod_dht_publication(published_pod, signature, public_key) {
                return HttpResponse {
                    status: "404 Not Found",
                    content_type: "application/json",
                    body: serde_json::json!({"found": false, "error": "Pod not found"}).to_string(),
                };
            }
            let retrieved_at = chrono::Utc::now();
            let expires_at = publication
                .get("expiresAt")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&chrono::Utc));
            if expires_at.is_some_and(|expires_at| expires_at <= retrieved_at) {
                return HttpResponse {
                    status: "404 Not Found",
                    content_type: "application/json",
                    body: serde_json::json!({"found": false, "error": "Pod not found"}).to_string(),
                };
            }
            routing::ok_response(
                serde_json::json!({
                    "found": true,
                    "podId": pod_id,
                    "publishedPod": published_pod,
                    "retrievedAt": retrieved_at.to_rfc3339(),
                    "expiresAt": expires_at.map(|value| value.to_rfc3339()),
                    "isValidSignature": true,
                })
                .to_string(),
            )
        }
        [section, filter, tail @ ..]
            if section == "discovery" && filter == "content" && !tail.is_empty() =>
        {
            let value = tail.join("/");
            if state
                .controller_features
                .read()
                .await
                .validate_storage()
                .is_err()
            {
                return routing::internal_server_error_response(
                    "Failed to discover pods by content",
                );
            }
            let started_at = std::time::Instant::now();
            let required_keys = vec![format!(
                "pod:discover:content:{}",
                value.to_ascii_lowercase()
            )];
            let records = state
                .controller_features
                .read()
                .await
                .values_with_prefix("pod/discovery/")
                .into_iter()
                .filter(|record| pod_discovery_record_is_active(record, chrono::Utc::now()))
                .collect::<Vec<_>>();
            let pods = state.pods.read().await;
            let matches = pod_discovery_rows(&records, &pods, &required_keys);
            let total_found = matches.len();
            state
                .podcore_runtime_stats
                .record_discovery_search(filter, started_at.elapsed().as_millis() as u64);
            routing::ok_response(
                serde_json::json!({
                    "pods": matches,
                    "searchTerm": value,
                    "searchType": filter,
                    "searchedAt": chrono::Utc::now().to_rfc3339(),
                    "totalFound": total_found,
                })
                .to_string(),
            )
        }
        [section, filter, value]
            if section == "discovery" && matches!(filter.as_str(), "name" | "tag" | "tags") =>
        {
            if value.trim().is_empty() {
                return routing::bad_request_response("Search value is required");
            }
            if filter != "tags" && value.len() > 128 {
                return routing::bad_request_response("Search value must be within length limits");
            }
            let started_at = std::time::Instant::now();
            let tags = value
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .collect::<Vec<_>>();
            if filter == "tags"
                && (value.len() > 128 * 16
                    || tags.is_empty()
                    || tags.len() > 16
                    || tags.iter().any(|tag| tag.len() > 128))
            {
                return routing::bad_request_response(
                    "Tags are required and tag count/length limits must be respected",
                );
            }
            let storage_error = match filter.as_str() {
                "name" => "Failed to discover pods by name",
                "tag" => "Failed to discover pods by tag",
                "tags" => "Failed to discover pods by tags",
                _ => "Failed to discover pods",
            };
            if state
                .controller_features
                .read()
                .await
                .validate_storage()
                .is_err()
            {
                return routing::internal_server_error_response(storage_error);
            }
            let required_keys = match filter.as_str() {
                "name" => vec![format!("pod:discover:name:{}", value.to_ascii_lowercase())],
                "tag" => vec![format!("pod:discover:tag:{}", value.to_ascii_lowercase())],
                "tags" => tags
                    .iter()
                    .map(|tag| format!("pod:discover:tag:{}", tag.to_ascii_lowercase()))
                    .collect(),
                _ => Vec::new(),
            };
            let records = state
                .controller_features
                .read()
                .await
                .values_with_prefix("pod/discovery/")
                .into_iter()
                .filter(|record| pod_discovery_record_is_active(record, chrono::Utc::now()))
                .collect::<Vec<_>>();
            let pods = state.pods.read().await;
            let matches = pod_discovery_rows(&records, &pods, &required_keys);
            let total_found = matches.len();
            state
                .podcore_runtime_stats
                .record_discovery_search(filter, started_at.elapsed().as_millis() as u64);
            routing::ok_response(
                serde_json::json!({
                    "pods": matches,
                    "searchTerm": if filter == "tags" { tags.join(",") } else { value.clone() },
                    "searchType": filter,
                    "searchedAt": chrono::Utc::now().to_rfc3339(),
                    "totalFound": total_found,
                })
                .to_string(),
            )
        }
        [section, pod_id, peer_id] if section == "membership" => {
            if state.pods.read().await.validate_storage().is_err() {
                return routing::internal_server_error_response("Failed to retrieve membership");
            }
            let member = state
                .pods
                .read()
                .await
                .member_for_verification(pod_id, peer_id);
            let Some(member) = member else {
                return HttpResponse {
                    status: "404 Not Found",
                    content_type: "application/json",
                    body: serde_json::json!({
                        "found": false,
                        "error": "Membership not found"
                    })
                    .to_string(),
                };
            };
            let key = format!("pod/membership/{pod_id}/{peer_id}");
            let stored_record = state.controller_features.read().await.get(&key).cloned();
            let now = chrono::Utc::now();
            let signed_record = stored_record.unwrap_or_else(|| {
                pod_membership_signed_record(
                    state,
                    pod_id,
                    &member,
                    if member.is_banned { "ban" } else { "join" },
                    u64::try_from(now.timestamp_millis()).unwrap_or_default(),
                )
            });
            let record_timestamp = signed_record
                .get("timestampUnixMs")
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| {
                    chrono::DateTime::from_timestamp_millis(i64::try_from(value).ok()?)
                })
                .unwrap_or(now);
            routing::ok_response(
                serde_json::json!({
                    "found": true,
                    "podId": pod_id,
                    "peerId": peer_id,
                    "signedRecord": signed_record,
                    "retrievedAt": now.to_rfc3339(),
                    "expiresAt": (record_timestamp + chrono::Duration::hours(24)).to_rfc3339(),
                    "isValidSignature": true,
                })
                .to_string(),
            )
        }
        [section, pod_id, peer_id, verify] if section == "membership" && verify == "verify" => {
            if state.pods.read().await.validate_storage().is_err() {
                return routing::internal_server_error_response("Failed to verify membership");
            }
            let pods = state.pods.read().await;
            let member = pods.member_for_verification(pod_id, peer_id);
            let response = member.map_or_else(
                || {
                    serde_json::json!({
                        "isValidMember": false,
                        "isBanned": false,
                        "role": null,
                        "errorMessage": "Membership not found",
                    })
                },
                |member| {
                    serde_json::json!({
                        "isValidMember": !member.is_banned,
                        "isBanned": member.is_banned,
                        "role": member.role,
                    })
                },
            );
            routing::ok_response(response.to_string())
        }
        [section, pod_id, channel_id, count] if section == "messages" && count == "count" => {
            let messages = state.pod_channels.read().await;
            if messages.validate_storage().is_err() {
                return routing::internal_server_error_response(
                    "An error occurred while getting message count",
                );
            }
            let messages = messages.list(pod_id, channel_id, None);
            routing::ok_response((messages.len() as u64).to_string())
        }
        [section, pod_id, search] if section == "messages" && search == "search" => {
            let needle = query_parameter(query, "query")
                .unwrap_or_default()
                .to_ascii_lowercase();
            if needle.trim().is_empty() {
                return routing::bad_request_response("query is required");
            }
            let limit = query_parameter(query, "limit")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(50)
                .clamp(1, 500);
            let channel_filter = query_parameter(query, "channelId");
            let pods = state.pods.read().await;
            let Some(pod) = pods.get(pod_id) else {
                return routing::not_found_response();
            };
            drop(pods);
            let messages = state.pod_channels.read().await;
            if messages.validate_storage().is_err() {
                return routing::internal_server_error_response(
                    "An error occurred while searching messages",
                );
            }
            let rows = pod
                .channels
                .iter()
                .flat_map(|channel| messages.list(pod_id, &channel.channel_id, None))
                .filter(|message| {
                    channel_filter
                        .as_deref()
                        .is_none_or(|channel| message.channel_id == channel)
                        && message.body.to_ascii_lowercase().contains(&needle)
                })
                .take(limit)
                .collect::<Vec<_>>();
            routing::ok_response(
                serde_json::Value::Array(
                    rows.into_iter()
                        .map(|message| serde_json::json!(message))
                        .collect(),
                )
                .to_string(),
            )
        }
        [section, seen, message_id, pod_id] if section == "routing" && seen == "seen" => {
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && state
                    .controller_features
                    .read()
                    .await
                    .validate_storage()
                    .is_err()
            {
                return routing::internal_server_error_response(
                    "Failed to check message seen status",
                );
            }
            let key = format!("pod/routing-seen/{pod_id}/{message_id}");
            let seen = state.controller_features.read().await.get(&key).is_some();
            routing::ok_response(serde_json::json!({"isSeen": seen}).to_string())
        }
        [section, membership, pod_id, peer_id]
            if section == "verification" && membership == "membership" =>
        {
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && state.pods.read().await.validate_storage().is_err()
            {
                return routing::internal_server_error_response("Failed to verify membership");
            }
            if pod_id.chars().count() > 128 || peer_id.chars().count() > 128 {
                return routing::bad_request_response(
                    "PodId and PeerId are required and must be within length limits",
                );
            }
            let member = state
                .pods
                .read()
                .await
                .member_for_verification(pod_id, peer_id);
            let response = if let Some(member) = member {
                serde_json::json!({
                    "isValidMember": true,
                    "isBanned": member.is_banned,
                    "role": member.role,
                })
            } else {
                serde_json::json!({
                    "isValidMember": false,
                    "isBanned": false,
                    "errorMessage": "Membership not found",
                })
            };
            routing::ok_response(response.to_string())
        }
        [section, role, pod_id, peer_id, required_role]
            if section == "verification" && role == "role" =>
        {
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && state.pods.read().await.validate_storage().is_err()
            {
                return routing::internal_server_error_response("Failed to check role");
            }
            if pod_id.chars().count() > 128
                || peer_id.chars().count() > 128
                || required_role.chars().count() > 128
            {
                return routing::bad_request_response(
                    "PodId, PeerId, and RequiredRole are required and must be within length limits",
                );
            }
            let pods = state.pods.read().await;
            let member = pods.members(pod_id).and_then(|members| {
                members
                    .into_iter()
                    .find(|member| member.peer_id.eq_ignore_ascii_case(peer_id))
            });
            let role_rank = |role: &str| match role.to_ascii_lowercase().as_str() {
                "member" => Some(1_u8),
                "mod" | "moderator" => Some(2_u8),
                "owner" => Some(3_u8),
                _ => None,
            };
            let valid = member.as_ref().is_some_and(|member| {
                role_rank(&member.role)
                    .zip(role_rank(required_role))
                    .is_some_and(|(actual, required)| actual >= required)
            });
            routing::ok_response(serde_json::json!({"hasRole": valid}).to_string())
        }
        _ => routing::not_found_response(),
    }
}
