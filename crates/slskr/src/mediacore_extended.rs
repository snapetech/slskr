use super::*;

pub(crate) async fn mediacore_extended_response(
    path: &str,
    query: Option<&str>,
    state: &AppState,
) -> HttpResponse {
    // MediaCoreStatsService owns a process-local stopwatch in the frozen
    // runtime. Keep the dashboard's uptime tied to this daemon process rather
    // than returning the old fixed zero-duration compatibility value.
    static MEDIA_CORE_STARTED_AT: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let media_core_uptime = MEDIA_CORE_STARTED_AT
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis() as u64;
    let discovery = state.content_discovery.read().await;
    let library = state.library.read().await;
    let shares = state.shares.read().await;
    let controller_features = state.controller_features.read().await;
    let registered_mappings = controller_features
        .values_with_prefix("mediacore/contentid/")
        .into_iter()
        .filter_map(|value| {
            Some((
                value.get("externalId")?.as_str()?.to_owned(),
                value.get("contentId")?.as_str()?.to_owned(),
            ))
        })
        .collect::<Vec<_>>();
    let descriptor_records = controller_features.values_with_prefix("mediacore/descriptor/");
    // Metadata portability writes descriptor records, but the frozen
    // ContentDescriptorPublisher only counts descriptors that it itself has
    // successfully published.  Publication metadata is the durable marker
    // for that local publisher projection.
    let published_descriptors = descriptor_records
        .iter()
        .filter(|descriptor| descriptor.get("publishedAt").is_some())
        .cloned()
        .collect::<Vec<_>>();
    let cache_records = controller_features.values_with_prefix("mediacore/cache/");
    let ipld_link_records = controller_features.values_with_prefix("mediacore/ipld/");
    let mediacore_metrics = controller_features.values_with_prefix("mediacore/metrics/");
    drop(controller_features);
    let now = chrono::Utc::now();
    let metric_sum = |operation: &str, field: &str| -> u64 {
        mediacore_metrics
            .iter()
            .filter(|metric| metric["operation"].as_str() == Some(operation))
            .filter_map(|metric| metric["value"][field].as_u64())
            .sum()
    };
    let metric_count = |operation: &str| -> u64 { metric_sum(operation, "count") };
    let mappings = discovery
        .hash_entries()
        .len()
        .saturating_add(registered_mappings.len());
    let mut registered_domains = registered_mappings
        .iter()
        .filter_map(|(_, content_id)| content_id.split(':').nth(1))
        .map(str::to_ascii_lowercase)
        .collect::<HashSet<_>>();
    if discovery
        .hash_entries()
        .iter()
        .any(|entry| !entry.music_brainz_id.is_empty())
    {
        registered_domains.insert("music".to_owned());
    }
    let domains = registered_domains.len();
    let active_cache_records = cache_records
        .iter()
        .filter(|record| {
            record
                .get("expiresAt")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .is_some_and(|value| value.with_timezone(&chrono::Utc) > now)
        })
        .collect::<Vec<_>>();
    let descriptors = active_cache_records.len();
    let mut mappings_by_domain = BTreeMap::<String, usize>::new();
    let mut mappings_by_type = BTreeMap::<String, usize>::new();
    for content_id in registered_mappings
        .iter()
        .map(|(_, content_id)| content_id.as_str())
        .chain(
            discovery
                .hash_entries()
                .iter()
                .filter(|entry| !entry.music_brainz_id.is_empty())
                .map(|_| "content:music:recording:musicbrainz"),
        )
    {
        let parts = content_id.split(':').collect::<Vec<_>>();
        if parts.len() == 4 && parts[0].eq_ignore_ascii_case("content") {
            *mappings_by_domain.entry(parts[1].to_owned()).or_default() += 1;
            *mappings_by_type.entry(parts[2].to_owned()).or_default() += 1;
        }
    }
    let total_retrievals = metric_sum("retrieve", "count");
    let cache_hits = metric_sum("retrieve", "hits");
    let cache_misses = metric_sum("retrieve", "misses");
    let cache_size_bytes = active_cache_records
        .iter()
        .filter_map(|record| {
            record
                .get("descriptor")
                .or(Some(*record))
                .and_then(|descriptor| serde_json::to_vec(descriptor).ok())
        })
        .map(|descriptor| descriptor.len() as u64)
        .sum::<u64>();
    let total_fuzzy_matches = metric_count("fuzzy");
    let successful_fuzzy_matches = metric_sum("fuzzy", "successes");
    let mut registered_content_ids = registered_mappings
        .iter()
        .map(|(_, content_id)| content_id.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    registered_content_ids.extend(
        discovery
            .hash_entries()
            .iter()
            .filter(|entry| !entry.music_brainz_id.trim().is_empty())
            .map(|entry| {
                format!("content:music:recording:{}", entry.music_brainz_id).to_ascii_lowercase()
            }),
    );
    let mut ipld_seen_nodes = HashSet::new();
    let mut ipld_connected_nodes = HashSet::new();
    let mut total_ipld_links = 0_u64;
    let mut broken_ipld_links = 0_u64;
    for content_id in &registered_content_ids {
        ipld_seen_nodes.insert(content_id.clone());
        let Some(record) = ipld_link_records.iter().find(|record| {
            record["contentId"]
                .as_str()
                .is_some_and(|source| source.eq_ignore_ascii_case(content_id))
        }) else {
            continue;
        };
        let links = record["links"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        if !links.is_empty() {
            ipld_connected_nodes.insert(content_id.clone());
        }
        for link in links {
            total_ipld_links = total_ipld_links.saturating_add(1);
            let Some(target) = link["target"].as_str() else {
                continue;
            };
            let target = target.to_ascii_lowercase();
            ipld_seen_nodes.insert(target.clone());
            ipld_connected_nodes.insert(target.clone());
            if !registered_content_ids.contains(&target) {
                broken_ipld_links = broken_ipld_links.saturating_add(1);
            }
        }
    }
    let total_ipld_graphs = registered_content_ids.len() as u64;
    let total_ipld_nodes = ipld_seen_nodes.len() as u64;
    let orphaned_ipld_nodes = total_ipld_nodes.saturating_sub(ipld_connected_nodes.len() as u64);
    let total_perceptual_hashes = metric_count("perceptual-hash");
    let total_imports = metric_sum("portability", "imports");
    let successful_imports = metric_sum("portability", "successfulImports");
    let total_exports = metric_sum("portability", "exports");
    let total_data_transferred = metric_sum("portability", "dataTransferred");
    let expiring_cutoff = now + chrono::Duration::minutes(60);
    let mut active_publications = 0_u64;
    let mut expiring_publications = 0_u64;
    let mut total_storage_bytes = 0_u64;
    let mut total_remaining_ttl_hours = 0.0_f64;
    let mut publications_by_domain = BTreeMap::<String, u64>::new();
    let mut last_publish_operation = None;
    for descriptor in &published_descriptors {
        let content_id = descriptor
            .get("contentId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let domain = content_id
            .strip_prefix("content:")
            .and_then(|value| value.split(':').next())
            .map(|domain| {
                if domain.eq_ignore_ascii_case("mb") {
                    "audio".to_owned()
                } else {
                    domain.to_owned()
                }
            })
            .unwrap_or_else(|| "unknown".to_owned());
        let published_at = descriptor
            .get("publishedAt")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&chrono::Utc));
        let expires_at = descriptor
            .get("expiresAt")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&chrono::Utc));
        if let Some(expires_at) = expires_at.filter(|expires_at| *expires_at > now) {
            if published_at > last_publish_operation {
                last_publish_operation = published_at;
            }
            active_publications = active_publications.saturating_add(1);
            if expires_at <= expiring_cutoff {
                expiring_publications = expiring_publications.saturating_add(1);
            }
            total_remaining_ttl_hours += (expires_at - now).num_milliseconds() as f64 / 3_600_000.0;
            *publications_by_domain.entry(domain).or_default() += 1;
            total_storage_bytes = total_storage_bytes.saturating_add(
                descriptor
                    .get("sizeBytes")
                    .or_else(|| descriptor.get("size"))
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
            );
        }
    }
    let total_published = published_descriptors.len() as u64;
    let average_ttl_hours = if active_publications == 0 {
        0.0
    } else {
        total_remaining_ttl_hours / active_publications as f64
    };
    let successful_publishes = metric_sum("publish", "count");
    let republished_descriptors = metric_sum("publish", "republished");
    let registry = serde_json::json!({
        "totalMappings": mappings,
        "totalDomains": domains,
        "mappingsByDomain": mappings_by_domain,
        "mappingsByType": mappings_by_type,
        "lastUpdated": chrono::Utc::now().to_rfc3339(),
        "averageMappingsPerDomain": if domains == 0 { 0.0 } else { mappings as f64 / domains as f64 },
    });
    let descriptor_stats = serde_json::json!({
        "totalRetrievals": total_retrievals,
        "cacheHits": cache_hits,
        "cacheMisses": cache_misses,
        "cacheHitRatio": if total_retrievals == 0 { 0.0 } else { cache_hits as f64 / total_retrievals as f64 },
        "averageRetrievalTime": "00:00:00",
        "activeCacheEntries": descriptors,
        "cacheSizeBytes": cache_size_bytes,
        "expiredEntriesCleaned": 0,
        "lastCacheCleanup": chrono::Utc::now().to_rfc3339(),
        "retrievalsByDomain": {},
    });
    let fuzzy = serde_json::json!({
        "totalMatches": total_fuzzy_matches,
        "successfulMatches": successful_fuzzy_matches,
        "successRate": if total_fuzzy_matches == 0 { 0.0 } else { successful_fuzzy_matches as f64 / total_fuzzy_matches as f64 },
        "averageMatchingTime": "00:00:00",
        "accuracyByAlgorithm": {},
        "matchesByDomain": {},
        "averageConfidenceScore": if total_fuzzy_matches == 0 { 0.0 } else { metric_sum("fuzzy", "confidence") as f64 / total_fuzzy_matches as f64 },
    });
    let ipld = serde_json::json!({
        "totalLinks": total_ipld_links,
        "totalNodes": total_ipld_nodes,
        "totalGraphs": total_ipld_graphs,
        "linksByType": {},
        "graphsByRoot": {},
        "brokenLinksDetected": broken_ipld_links,
        "orphanedNodes": orphaned_ipld_nodes,
        "averageTraversalTime": "00:00:00",
        "graphConnectivityRatio": if total_ipld_nodes == 0 { 0.0 } else { ipld_connected_nodes.len() as f64 / total_ipld_nodes as f64 },
    });
    let perceptual = serde_json::json!({
        "totalHashesComputed": total_perceptual_hashes,
        "averageComputationTime": "00:00:00",
        "statsByAlgorithm": {},
        "overallAccuracy": 0,
        "hashesByContentType": if total_perceptual_hashes == 0 { serde_json::json!({}) } else { serde_json::json!({"audio": total_perceptual_hashes}) },
        "duplicateHashesDetected": 0,
    });
    let portability = serde_json::json!({
        "totalExports": total_exports,
        "totalImports": total_imports,
        "successfulImports": successful_imports,
        "importSuccessRate": if total_imports == 0 { 0.0 } else { successful_imports as f64 / total_imports as f64 },
        "conflictsByType": {},
        "resolutionsUsed": {"Merge": 0, "Overwrite": 0, "Skip": 0, "KeepExisting": 0},
        "averageExportTime": "00:00:00",
        "averageImportTime": "00:00:00",
        "totalDataTransferred": total_data_transferred,
    });
    let publishing = serde_json::json!({
        "totalPublished": total_published,
        "activePublications": active_publications,
        "expiredPublications": expiring_publications,
        "publicationSuccessRate": if successful_publishes == 0 { 0.0 } else { 1.0 },
        "publicationsByDomain": publications_by_domain,
        "averagePublishTime": "00:00:00",
        "republishedDescriptors": republished_descriptors,
        "failedPublications": 0,
        "recentErrors": {},
    });
    if let Some(external_id) = path_segment_after(path, "/api/mediacore/contentid/resolve/") {
        let external_id = decoded_path_segment(external_id);
        if let Some((_, content_id)) = registered_mappings
            .iter()
            .find(|(registered, _)| registered.eq_ignore_ascii_case(&external_id))
        {
            return routing::ok_response(serde_json::json!({"contentId": content_id}).to_string());
        }
        return HttpResponse {
            status: "404 Not Found",
            content_type: "application/json; charset=utf-8",
            body: serde_json::json!({"error": "External ID not found"}).to_string(),
        };
    }
    if let Some(segments) = decoded_segments_after(path, "/api/mediacore/contentid/domain/") {
        if let [domain, marker, type_name] = segments.as_slice() {
            if marker == "type" {
                let mut content_ids = discovery
                    .hash_entries()
                    .iter()
                    .filter(|entry| !entry.music_brainz_id.is_empty())
                    .map(|entry| format!("content:{domain}:{type_name}:{}", entry.music_brainz_id))
                    .collect::<Vec<_>>();
                content_ids.extend(registered_mappings.iter().filter_map(|(_, content_id)| {
                    let parts = content_id.split(':').collect::<Vec<_>>();
                    (parts.len() == 4
                        && parts[1].eq_ignore_ascii_case(domain)
                        && parts[2].eq_ignore_ascii_case(type_name))
                    .then(|| content_id.clone())
                }));
                content_ids.sort();
                content_ids.dedup();
                return routing::ok_response(
                    serde_json::json!({
                        "normalizedDomain": domain,
                        "normalizedType": type_name,
                        "contentIds": content_ids,
                    })
                    .to_string(),
                );
            }
        } else if let [domain] = segments.as_slice() {
            let mut content_ids = discovery
                .hash_entries()
                .iter()
                .filter(|entry| !entry.music_brainz_id.is_empty())
                .map(|entry| format!("content:{domain}:recording:{}", entry.music_brainz_id))
                .collect::<Vec<_>>();
            content_ids.extend(registered_mappings.iter().filter_map(|(_, content_id)| {
                let parts = content_id.split(':').collect::<Vec<_>>();
                (parts.len() == 4 && parts[1].eq_ignore_ascii_case(domain))
                    .then(|| content_id.clone())
            }));
            content_ids.sort();
            content_ids.dedup();
            return routing::ok_response(
                serde_json::json!({"normalizedDomain": domain, "contentIds": content_ids})
                    .to_string(),
            );
        }
    }
    if let Some(external_id) = path_segment_after(path, "/api/mediacore/contentid/exists/") {
        let external_id = decoded_path_segment(external_id);
        let exists = registered_mappings
            .iter()
            .any(|(registered, _)| registered.eq_ignore_ascii_case(&external_id))
            || discovery.hash_entries().iter().any(|entry| {
                entry.music_brainz_id.eq_ignore_ascii_case(&external_id)
                    || entry.flac_key.eq_ignore_ascii_case(&external_id)
                    || entry.byte_hash.eq_ignore_ascii_case(&external_id)
                    || entry.full_file_hash.eq_ignore_ascii_case(&external_id)
                    || entry.file_sha256.eq_ignore_ascii_case(&external_id)
            });
        return routing::ok_response(serde_json::json!({"exists": exists}).to_string());
    }
    if let Some(content_id) = path_segment_after(path, "/api/mediacore/contentid/external/") {
        let content_id = decoded_path_segment(content_id);
        let mut external_ids = discovery
            .hash_entries()
            .iter()
            .filter(|entry| {
                entry.music_brainz_id.eq_ignore_ascii_case(&content_id)
                    || format!("content:music:recording:{}", entry.music_brainz_id)
                        .eq_ignore_ascii_case(&content_id)
            })
            .flat_map(|entry| [entry.flac_key.clone(), entry.music_brainz_id.clone()])
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        external_ids.extend(
            registered_mappings
                .iter()
                .filter(|(_, registered_content_id)| {
                    registered_content_id.eq_ignore_ascii_case(&content_id)
                })
                .map(|(external_id, _)| external_id.clone()),
        );
        external_ids.sort();
        external_ids.dedup();
        return routing::ok_response(serde_json::json!({"externalIds": external_ids}).to_string());
    }
    if let Some(raw_content_id) = path.strip_prefix("/api/mediacore/contentid/validate/") {
        let content_id = decoded_path_segment(raw_content_id);
        let parts = content_id.split(':').collect::<Vec<_>>();
        let valid = parts.len() == 4
            && parts[0].eq_ignore_ascii_case("content")
            && parts[1..].iter().all(|part| !part.trim().is_empty());
        let value = if valid {
            serde_json::json!({
                "contentId": content_id,
                "isValid": true,
                "domain": parts[1],
                "type": parts[2],
                "id": parts[3],
            })
        } else {
            serde_json::json!({
                "contentId": content_id,
                "isValid": false,
                "error": "Invalid ContentID format. Expected: content:<domain>:<type>:<id>",
            })
        };
        return routing::ok_response(value.to_string());
    }
    if let Some(raw_content_id) = path.strip_prefix("/api/mediacore/ipld/traverse/") {
        let start_content_id = decoded_path_segment(raw_content_id).trim().to_owned();
        if start_content_id.is_empty() {
            return routing::bad_request_response("Start ContentID is required");
        }
        let link_name = query_parameter(query, "linkName")
            .unwrap_or_default()
            .trim()
            .to_owned();
        if link_name.is_empty() {
            return routing::bad_request_response("Link name is required");
        }
        let max_depth = match query_parameter(query, "maxDepth") {
            None => 3,
            Some(value) => match value.parse::<usize>() {
                Ok(value) if (1..=10).contains(&value) => value,
                _ => return routing::bad_request_response("Max depth must be between 1 and 10"),
            },
        };
        let links_for = |content_id: &str| {
            ipld_link_records
                .iter()
                .find(|record| record["contentId"].as_str() == Some(content_id))
                .and_then(|record| record["links"].as_array())
                .cloned()
                .unwrap_or_default()
        };
        let incoming_for = |content_id: &str| {
            ipld_link_records
                .iter()
                .filter(|record| {
                    record["links"].as_array().is_some_and(|links| {
                        links
                            .iter()
                            .any(|link| link["target"].as_str() == Some(content_id))
                    })
                })
                .filter_map(|record| record["contentId"].as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        };
        let mut visited = HashSet::new();
        let mut visited_nodes = Vec::new();
        let mut paths = Vec::new();
        let mut pending = vec![(
            start_content_id.clone(),
            0_usize,
            vec![start_content_id.clone()],
            Vec::<serde_json::Value>::new(),
        )];
        while let Some((content_id, depth, content_ids, path_links)) = pending.pop() {
            if !visited.insert(content_id.clone()) {
                continue;
            }
            let outgoing_links = links_for(&content_id);
            visited_nodes.push(serde_json::json!({
                "contentId": content_id,
                "outgoingLinks": outgoing_links,
                "incomingLinks": incoming_for(&content_id),
            }));
            if depth >= max_depth {
                continue;
            }
            for link in links_for(&content_id)
                .into_iter()
                .filter(|link| link["name"].as_str() == Some(link_name.as_str()))
            {
                let Some(target) = link["target"].as_str().map(str::to_owned) else {
                    continue;
                };
                let mut next_content_ids = content_ids.clone();
                next_content_ids.push(target.clone());
                let mut next_path_links = path_links.clone();
                next_path_links.push(link.clone());
                paths.push(serde_json::json!({
                    "contentIds": next_content_ids,
                    "links": next_path_links,
                }));
                if depth + 1 < max_depth && !visited.contains(&target) {
                    pending.push((target, depth + 1, next_content_ids, next_path_links));
                }
            }
        }
        return routing::ok_response(
            serde_json::json!({
                "startContentId": start_content_id,
                "linkName": link_name,
                "visitedNodes": visited_nodes,
                "paths": paths,
                "completedTraversal": true,
            })
            .to_string(),
        );
    }
    if let Some(raw_content_id) = path.strip_prefix("/api/mediacore/ipld/graph/") {
        let content_id = decoded_path_segment(raw_content_id);
        if content_id.trim().is_empty() {
            return routing::bad_request_response("ContentID is required");
        }
        let max_depth = match query_parameter(query, "maxDepth") {
            None => 2,
            Some(value) => match value.parse::<usize>() {
                Ok(value) if (1..=5).contains(&value) => value,
                _ => return routing::bad_request_response("Max depth must be between 1 and 5"),
            },
        };
        let links_for = |node_content_id: &str| {
            ipld_link_records
                .iter()
                .find(|record| record["contentId"].as_str() == Some(node_content_id))
                .and_then(|record| record["links"].as_array())
                .cloned()
                .unwrap_or_default()
        };
        let incoming_for = |node_content_id: &str| {
            ipld_link_records
                .iter()
                .filter(|record| {
                    record["links"].as_array().is_some_and(|links| {
                        links
                            .iter()
                            .any(|link| link["target"].as_str() == Some(node_content_id))
                    })
                })
                .filter_map(|record| record["contentId"].as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        };
        let mut visited = HashSet::from([content_id.clone()]);
        let mut nodes = Vec::new();
        let mut paths = Vec::new();
        let mut pending = vec![(content_id.clone(), 0_usize)];
        while let Some((node_content_id, depth)) = pending.pop() {
            let outgoing_links = links_for(&node_content_id);
            nodes.push(serde_json::json!({
                "contentId": node_content_id.clone(),
                "outgoingLinks": outgoing_links,
                "incomingLinks": incoming_for(&node_content_id),
            }));
            if depth >= max_depth {
                continue;
            }
            for link in links_for(&node_content_id) {
                let Some(target) = link["target"].as_str().map(str::to_owned) else {
                    continue;
                };
                if !visited.insert(target.clone()) {
                    continue;
                }
                paths.push(serde_json::json!({
                    "contentIds": [node_content_id, target],
                    "links": [link],
                }));
                pending.push((target, depth + 1));
            }
        }
        return routing::ok_response(
            serde_json::json!({
                "rootContentId": content_id,
                "nodes": nodes,
                "paths": paths,
            })
            .to_string(),
        );
    }
    if let Some(raw_content_id) = path.strip_prefix("/api/mediacore/ipld/inbound/") {
        let target_content_id = decoded_path_segment(raw_content_id);
        let inbound_links = ipld_link_records
            .iter()
            .filter(|value| {
                value
                    .get("links")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|links| {
                        links.iter().any(|link| {
                            link.get("target")
                                .or_else(|| link.get("targetContentId"))
                                .and_then(serde_json::Value::as_str)
                                .is_some_and(|link| link.eq_ignore_ascii_case(&target_content_id))
                        })
                    })
            })
            .map(|value| {
                value
                    .get("contentId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        return routing::ok_response(
            serde_json::json!({"inboundLinks": inbound_links}).to_string(),
        );
    }
    if let Some(raw_content_id) = path.strip_prefix("/api/mediacore/retrieve/descriptor/") {
        let content_id = decoded_path_segment(raw_content_id).trim().to_owned();
        if content_id.is_empty() {
            return routing::bad_request_response("ContentID is required");
        }
        let bypass_cache = query_parameter(query, "bypassCache")
            .as_deref()
            .and_then(parse_bool_value)
            .unwrap_or(false);
        return match mediacore_retrieve_descriptor(&content_id, bypass_cache, state).await {
            Ok(result) if result.found => routing::ok_response(result.value.to_string()),
            Ok(_) => HttpResponse {
                status: "404 Not Found",
                content_type: "application/json",
                body: serde_json::json!({
                    "contentId": content_id,
                    "found": false,
                    "error": "Descriptor not found",
                })
                .to_string(),
            },
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    if let Some(domain) = path_segment_after(path, "/api/mediacore/retrieve/query/domain/") {
        let domain = decoded_path_segment(domain).trim().to_ascii_lowercase();
        if domain.is_empty() {
            return routing::bad_request_response("Domain is required");
        }
        let type_filter = query_parameter(query, "type")
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty());
        let max_results = match query_parameter(query, "maxResults") {
            None => 50,
            Some(value) => match value.parse::<usize>() {
                Ok(value) if (1..=500).contains(&value) => value,
                _ => return routing::bad_request_response("Max results must be between 1 and 500"),
            },
        };
        let mut descriptors = active_cache_records
            .iter()
            .filter_map(|record| record.get("descriptor"))
            .filter(|descriptor| {
                let content_id = descriptor
                    .get("contentId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let content_parts = content_id.split(':').collect::<Vec<_>>();
                let descriptor_domain = descriptor
                    .get("domain")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| content_parts.get(1).copied())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let descriptor_type = descriptor
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| content_parts.get(2).copied())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                descriptor_domain == domain
                    && type_filter
                        .as_deref()
                        .is_none_or(|requested| descriptor_type == requested)
            })
            .cloned()
            .collect::<Vec<_>>();
        let has_more_results = descriptors.len() > max_results;
        descriptors.truncate(max_results);
        return routing::ok_response(
            serde_json::json!({
                "domain": domain,
                "type": type_filter,
                "totalFound": descriptors.len(),
                "queryDuration": "00:00:00",
                "descriptors": descriptors,
                "hasMoreResults": has_more_results,
            })
            .to_string(),
        );
    }
    let mut registered_content_ids = registered_mappings
        .iter()
        .map(|(_, content_id)| content_id.clone())
        .collect::<Vec<_>>();
    registered_content_ids.extend(
        discovery
            .hash_entries()
            .iter()
            .filter(|entry| !entry.music_brainz_id.trim().is_empty())
            .map(|entry| format!("content:music:recording:{}", entry.music_brainz_id)),
    );
    registered_content_ids.sort();
    registered_content_ids.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    let registered_lookup = registered_content_ids
        .iter()
        .map(|content_id| content_id.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut broken_links = Vec::new();
    let mut orphaned_links = Vec::new();
    for record in &ipld_link_records {
        let source = record
            .get("contentId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let source_registered = registered_lookup.contains(&source.to_ascii_lowercase());
        let links = record
            .get("links")
            .and_then(serde_json::Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        for link in links {
            let target = link
                .get("target")
                .or_else(|| link.get("targetContentId"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if target.trim().is_empty() {
                continue;
            }
            let name = link
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let description = format!("{source} -> {target} ({name})");
            if !source_registered {
                orphaned_links.push(description);
            } else if !registered_lookup.contains(&target.to_ascii_lowercase()) {
                broken_links.push(description);
            }
        }
    }
    let value = match path {
        "/api/mediacore/contentid/stats" => serde_json::json!({
            "totalMappings": mappings,
            "totalDomains": domains,
            "mappingsByDomain": registry["mappingsByDomain"],
        }),
        "/api/mediacore/ipld/validate" => serde_json::json!({
            "isValid": broken_links.is_empty() && orphaned_links.is_empty(),
            "brokenLinks": broken_links,
            "orphanedLinks": orphaned_links,
            "totalLinksValidated": registered_content_ids.len(),
        }),
        "/api/mediacore/perceptualhash/algorithms" => serde_json::json!({
            "algorithms": ["Chromaprint", "PHash", "Spectral"],
            "descriptions": {
                "ChromaPrint": "Audio fingerprinting algorithm for music identification",
                "PHash": "Perceptual hash for image/video similarity detection",
                "Spectral": "Simple spectral analysis hash (fallback)",
            }
        }),
        "/api/mediacore/portability/merge-strategies" => serde_json::json!({
            "strategies": [
                {"strategy":"PreferNewer","name":"Prefer Newer","description":"Prefer metadata with newer timestamps"},
                {"strategy":"PreferHigherPriority","name":"Prefer Higher Priority","description":"Prefer metadata from higher priority sources"},
                {"strategy":"CombineAll","name":"Combine All","description":"Combine metadata fields from all sources"}
            ]
        }),
        "/api/mediacore/portability/strategies" => serde_json::json!({
            "strategies": [
                {"strategy":"Skip","name":"Skip","description":"Skip conflicting entries without importing them"},
                {"strategy":"Overwrite","name":"Overwrite","description":"Replace existing metadata with imported data"},
                {"strategy":"Merge","name":"Merge","description":"Intelligently merge existing and imported metadata"},
                {"strategy":"KeepExisting","name":"Keep Existing","description":"Keep existing metadata and ignore imported data"}
            ]
        }),
        "/api/mediacore/publish/stats" => serde_json::json!({
            "totalPublishedDescriptors": total_published,
            "activePublications": active_publications,
            "expiringSoon": expiring_publications,
            "lastPublishOperation": last_publish_operation
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| "0001-01-01T00:00:00+00:00".to_owned()),
            "publicationsByDomain": publications_by_domain,
            "totalStorageBytes": total_storage_bytes,
            "averageTtlHours": average_ttl_hours,
        }),
        "/api/mediacore/retrieve/stats" | "/api/mediacore/stats/descriptors" => descriptor_stats,
        "/api/mediacore/stats/fuzzy" => fuzzy,
        "/api/mediacore/stats/ipld" => ipld,
        "/api/mediacore/stats/perceptual" => perceptual,
        "/api/mediacore/stats/portability" => portability,
        "/api/mediacore/stats/publishing" => publishing,
        "/api/mediacore/stats/registry" => registry,
        "/api/mediacore/stats/dashboard" => serde_json::json!({
            "timestamp": now.to_rfc3339(),
            "uptime": format_timespan_millis(media_core_uptime),
            "contentRegistry": registry,
            "descriptors": descriptor_stats,
            "fuzzyMatching": fuzzy,
            "ipldMapping": ipld,
            "perceptualHashing": perceptual,
            "metadataPortability": portability,
            "publishing": publishing,
            "contentPublishing": publishing,
            "systemResources": {
                "libraryItems": library.records.len(),
                "sharedFiles": shares.entries.len(),
                "hashEntries": discovery.hash_entries().len(),
            },
        }),
        _ => return routing::not_found_response(),
    };
    routing::ok_response(value.to_string())
}
