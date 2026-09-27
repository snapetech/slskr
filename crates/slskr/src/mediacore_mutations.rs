use super::*;

pub(crate) async fn mediacore_mutation_response(
    method: &str,
    path: &str,
    body: &str,
    state: &AppState,
) -> Option<HttpResponse> {
    if path == "/api/mediacore/contentid/register" {
        let Some(external_id) =
            extract_json_string_field(body, "externalId").filter(|value| !value.trim().is_empty())
        else {
            return Some(routing::bad_request_response("ExternalId is required"));
        };
        let Some(content_id) =
            extract_json_string_field(body, "contentId").filter(|value| !value.trim().is_empty())
        else {
            return Some(routing::bad_request_response("ContentId is required"));
        };
        let parts = content_id.split(':').collect::<Vec<_>>();
        if parts.len() != 4
            || !parts[0].eq_ignore_ascii_case("content")
            || parts[1..].iter().any(|part| part.trim().is_empty())
        {
            return Some(routing::bad_request_response(
                "Invalid ContentID format. Expected: content:<domain>:<type>:<id>",
            ));
        }
        let record = serde_json::json!({"externalId": external_id, "contentId": content_id});
        return Some(
            match state
                .controller_features
                .upsert(format!("mediacore/contentid/{external_id}"), record)
                .await
            {
                Ok(()) => routing::ok_response(
                    serde_json::json!({"message": "ContentID mapping registered successfully"})
                        .to_string(),
                ),
                Err(error) => routing::service_unavailable_response(&error),
            },
        );
    }
    if matches!(
        path,
        "/api/mediacore/perceptualhash/audio" | "/api/mediacore/perceptualhash/image"
    ) {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload) => payload,
            Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
        };
        let requested_algorithm = payload
            .get("algorithm")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(if path.ends_with("/audio") {
                "Chromaprint"
            } else {
                "PHash"
            });
        let algorithm = match requested_algorithm.to_ascii_lowercase().as_str() {
            "chromaprint" => "Chromaprint",
            "phash" => "PHash",
            "spectral" => "Spectral",
            _ => return Some(routing::bad_request_response("Unsupported algorithm")),
        };
        let numeric_hash = if path.ends_with("/audio") {
            let Some(samples) = payload
                .get("samples")
                .and_then(serde_json::Value::as_array)
                .filter(|samples| !samples.is_empty())
            else {
                return Some(routing::bad_request_response("Audio samples are required"));
            };
            if samples.len() > 60 * 60 * 48_000 {
                return Some(routing::bad_request_response(
                    "Audio sample count exceeds the one-hour analysis limit",
                ));
            }
            let sample_rate = payload
                .get("sampleRate")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default();
            if sample_rate <= 0 {
                return Some(routing::bad_request_response(
                    "Valid sample rate is required",
                ));
            }
            let samples = samples
                .iter()
                .map(|sample| sample.as_f64().unwrap_or_default())
                .collect::<Vec<_>>();
            if algorithm == "Chromaprint" && samples.len() < 4_096 {
                0
            } else {
                let window_size = samples.len() / 8;
                let features = (0..8)
                    .map(|window| {
                        let start = window * window_size;
                        let end = (start + window_size).min(samples.len());
                        let values = &samples[start..end];
                        if values.is_empty() {
                            0.0
                        } else {
                            (values.iter().map(|value| value * value).sum::<f64>()
                                / values.len() as f64)
                                .sqrt()
                        }
                    })
                    .collect::<Vec<_>>();
                let mut sorted = features.clone();
                sorted.sort_by(f64::total_cmp);
                let median = sorted[features.len() / 2];
                features
                    .iter()
                    .enumerate()
                    .fold(0_u64, |hash, (index, value)| {
                        if *value > median {
                            hash | (1_u64 << index)
                        } else {
                            hash
                        }
                    })
            }
        } else {
            let Some(pixels) = payload
                .get("pixels")
                .and_then(serde_json::Value::as_str)
                .and_then(|pixels| STANDARD.decode(pixels.as_bytes()).ok())
                .filter(|pixels| !pixels.is_empty())
            else {
                return Some(routing::bad_request_response("Image pixels are required"));
            };
            let width = payload
                .get("width")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default();
            let height = payload
                .get("height")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default();
            if width <= 0 || height <= 0 {
                return Some(routing::bad_request_response(
                    "Valid image dimensions are required",
                ));
            }
            let pixel_count = width.saturating_mul(height);
            if pixel_count > 16_777_216 {
                return Some(routing::bad_request_response(
                    "Image dimensions exceed the analysis limit",
                ));
            }
            if ![pixel_count, pixel_count * 3, pixel_count * 4]
                .contains(&i64::try_from(pixels.len()).unwrap_or(i64::MAX))
            {
                return Some(routing::bad_request_response(
                    "Pixel buffer length must match width and height",
                ));
            }
            if algorithm == "PHash" {
                if pixels.len() != usize::try_from(pixel_count * 4).unwrap_or(usize::MAX) {
                    return Some(routing::internal_server_error_response(
                        "Failed to compute image perceptual hash",
                    ));
                }
                let width = usize::try_from(width).unwrap_or_default();
                let height = usize::try_from(height).unwrap_or_default();
                let grayscale = (0..width * height)
                    .map(|index| {
                        let offset = index * 4;
                        (0.299 * f64::from(pixels[offset])
                            + 0.587 * f64::from(pixels[offset + 1])
                            + 0.114 * f64::from(pixels[offset + 2]))
                            / 255.0
                    })
                    .collect::<Vec<_>>();
                let mut low_frequency = (0..32)
                    .map(|index| {
                        let x = index % 8;
                        let y = index / 8;
                        let source_x = x * width / 8;
                        let source_y = y * height / 8;
                        let value =
                            grayscale[(source_y * width + source_x).min(grayscale.len() - 1)];
                        if index % 2 == 0 {
                            value
                        } else {
                            -value
                        }
                    })
                    .collect::<Vec<_>>();
                low_frequency.sort_by(f64::total_cmp);
                let median = low_frequency[low_frequency.len() / 2];
                low_frequency
                    .iter()
                    .enumerate()
                    .fold(0_u64, |hash, (index, value)| {
                        if *value > median {
                            hash | (1_u64 << index)
                        } else {
                            hash
                        }
                    })
            } else {
                let step = (pixels.len() / 64).max(1);
                (0..64)
                    .take_while(|index| index * step < pixels.len())
                    .fold(0_u64, |hash, index| {
                        if pixels[index * step] > 128 {
                            hash | (1_u64 << index)
                        } else {
                            hash
                        }
                    })
            }
        };
        return Some(routing::ok_response(
            serde_json::json!({
                "algorithm": algorithm,
                "hex": format!("{numeric_hash:016X}"),
                "numericHash": numeric_hash,
            })
            .to_string(),
        ));
    }
    if path == "/api/mediacore/perceptualhash/similarity" {
        let Some(left) = extract_json_string_field(body, "hashA") else {
            return Some(routing::bad_request_response(
                "Two hash values are required",
            ));
        };
        let Some(right) = extract_json_string_field(body, "hashB") else {
            return Some(routing::bad_request_response(
                "Two hash values are required",
            ));
        };
        let payload = serde_json::from_str::<serde_json::Value>(body).unwrap_or_default();
        let threshold = payload
            .get("threshold")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.8);
        if !(0.0..=1.0).contains(&threshold) {
            return Some(routing::bad_request_response(
                "Threshold must be between 0 and 1",
            ));
        }
        let normalized_left = left
            .trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        let normalized_right = right
            .trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        let (Ok(left_hash), Ok(right_hash)) = (
            u64::from_str_radix(normalized_left, 16),
            u64::from_str_radix(normalized_right, 16),
        ) else {
            return Some(routing::bad_request_response(
                "Hash values must be valid hexadecimal numbers",
            ));
        };
        let distance = (left_hash ^ right_hash).count_ones();
        let similarity = 1.0 - f64::from(distance) / 64.0;
        return Some(routing::ok_response(
            serde_json::json!({
                "hashA": normalized_left,
                "hashB": normalized_right,
                "hammingDistance": distance,
                "similarity": similarity,
                "areSimilar": similarity >= threshold,
                "threshold": threshold,
            })
            .to_string(),
        ));
    }
    if let Some(content_id) = path_segment_after(path, "/api/mediacore/fuzzymatch/find/") {
        let content_id = decoded_path_segment(content_id);
        let request = serde_json::from_str::<serde_json::Value>(body).unwrap_or_default();
        let min_confidence = request
            .get("minConfidence")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.7);
        let max_candidates = request
            .get("maxCandidates")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(50);
        let max_results = request
            .get("maxResults")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(10);
        if !(0.0..=1.0).contains(&min_confidence) || max_candidates == 0 || max_results == 0 {
            return Some(routing::bad_request_response("Invalid search parameters"));
        }
        let Some(target_domain) = normalized_content_domain(&content_id) else {
            return Some(routing::ok_response(
                serde_json::json!({
                    "targetContentId": content_id,
                    "totalCandidates": 0,
                    "matches": [],
                    "searchParameters": {
                        "minConfidence": min_confidence,
                        "maxCandidates": max_candidates,
                        "maxResults": max_results,
                    },
                })
                .to_string(),
            ));
        };
        let features = state.controller_features.read().await;
        let mut candidates = features
            .values_with_prefix("mediacore/contentid/")
            .into_iter()
            .filter_map(|mapping| {
                mapping
                    .get("contentId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .filter(|candidate| !candidate.eq_ignore_ascii_case(&content_id))
            .filter(|candidate| {
                normalized_content_domain(candidate)
                    .is_some_and(|domain| domain.eq_ignore_ascii_case(&target_domain))
            })
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        candidates.truncate(usize::try_from(max_candidates).unwrap_or(usize::MAX));
        let descriptors = features.values_with_prefix("mediacore/descriptor/");
        drop(features);
        let mut matches = candidates
            .iter()
            .filter_map(|candidate| {
                let perceptual =
                    descriptor_perceptual_similarity(&content_id, candidate, &descriptors);
                let target_identifier = content_id.rsplit(':').next().unwrap_or(&content_id);
                let candidate_identifier = candidate.rsplit(':').next().unwrap_or(candidate);
                let text = levenshtein_similarity(target_identifier, candidate_identifier);
                let confidence = perceptual * 0.7 + text * 0.3;
                (confidence >= min_confidence).then(|| {
                    let reason = if perceptual > text {
                        "PerceptualHash"
                    } else if text > perceptual {
                        "TextSimilarity"
                    } else {
                        "Combined"
                    };
                    serde_json::json!({
                        "targetContentId": content_id,
                        "candidateContentId": candidate,
                        "confidence": confidence,
                        "reason": reason,
                    })
                })
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right["confidence"]
                .as_f64()
                .unwrap_or_default()
                .total_cmp(&left["confidence"].as_f64().unwrap_or_default())
                .then_with(|| {
                    left["candidateContentId"]
                        .as_str()
                        .unwrap_or_default()
                        .cmp(right["candidateContentId"].as_str().unwrap_or_default())
                })
        });
        matches.truncate(usize::try_from(max_results).unwrap_or(usize::MAX));
        let total_candidates = candidates.len();
        return Some(routing::ok_response(
            serde_json::json!({
                "targetContentId": content_id,
                "totalCandidates": total_candidates,
                "matches": matches,
                "searchParameters": {
                    "minConfidence": min_confidence,
                    "maxCandidates": max_candidates,
                    "maxResults": max_results,
                },
            })
            .to_string(),
        ));
    }
    if matches!(
        path,
        "/api/mediacore/fuzzymatch/text" | "/api/mediacore/fuzzymatch/perceptual"
    ) {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload) => payload,
            Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
        };
        let perceptual = path.ends_with("/perceptual");
        let left = payload
            .get(if perceptual { "contentIdA" } else { "textA" })
            .and_then(serde_json::Value::as_str);
        let right = payload
            .get(if perceptual { "contentIdB" } else { "textB" })
            .and_then(serde_json::Value::as_str);
        let (Some(left), Some(right)) = (left, right) else {
            return Some(routing::bad_request_response("Both values are required"));
        };
        if left.trim().is_empty() || right.trim().is_empty() {
            return Some(routing::bad_request_response("Both values are required"));
        }
        if perceptual {
            let threshold = payload
                .get("threshold")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.7);
            if !(0.0..=1.0).contains(&threshold) {
                return Some(routing::bad_request_response(
                    "Threshold must be between 0 and 1",
                ));
            }
            let descriptors = state
                .controller_features
                .read()
                .await
                .values_with_prefix("mediacore/descriptor/");
            let similarity =
                descriptor_perceptual_similarity(left.trim(), right.trim(), &descriptors);
            return Some(routing::ok_response(
                serde_json::json!({
                    "contentIdA": left.trim(),
                    "contentIdB": right.trim(),
                    "similarity": similarity,
                    "isSimilar": similarity >= threshold,
                    "threshold": threshold,
                })
                .to_string(),
            ));
        }
        let left = left.trim();
        let right = right.trim();
        let levenshtein = levenshtein_similarity(left, right);
        let phonetic = phonetic_similarity(left, right);
        let combined = levenshtein * 0.7 + phonetic * 0.3;
        return Some(routing::ok_response(
            serde_json::json!({
                "textA": left,
                "textB": right,
                "levenshteinSimilarity": levenshtein,
                "phoneticSimilarity": phonetic,
                "combinedSimilarity": combined,
            })
            .to_string(),
        ));
    }
    if let Some(content_id) = path_segment_after(path, "/api/mediacore/ipld/links/") {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload) => payload,
            Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
        };
        if payload
            .get("links")
            .is_some_and(|links| json_array_exceeds_limit(links, MAX_MEDIACORE_BATCH_ITEMS))
        {
            return Some(routing::bad_request_response(
                "links must contain at most 100 items",
            ));
        }
        let links = payload
            .get("links")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        if links.is_empty() {
            return Some(routing::bad_request_response(
                "At least one link is required",
            ));
        }
        let valid_links = links.iter().all(|link| {
            link.get("name")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| !value.trim().is_empty())
                && link
                    .get("target")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty())
        });
        if !valid_links {
            return Some(routing::bad_request_response(
                "Each link requires a non-empty name and target",
            ));
        }
        let link_count = links.len();
        let value = serde_json::json!({
            "contentId": decoded_path_segment(content_id),
            "links": links,
            "linkCount": link_count,
            "updatedAt": unix_timestamp(),
        });
        let _ = record_mediacore_metric(
            state,
            "ipld",
            serde_json::json!({"count": link_count, "nodes": link_count.saturating_add(1)}),
        )
        .await;
        let key = format!("mediacore/ipld/{}", decoded_path_segment(content_id));
        return Some(
            match state.controller_features.upsert(key, value.clone()).await {
                Ok(()) => routing::ok_response(
                    serde_json::json!({"message": "Links added successfully"}).to_string(),
                ),
                Err(error) => routing::service_unavailable_response(&error),
            },
        );
    }
    if path.starts_with("/api/mediacore/publish/descriptor") {
        let route_id = path
            .strip_prefix("/api/mediacore/publish/descriptor/")
            .map(decoded_path_segment);
        if method == "DELETE" {
            let Some(content_id) = route_id else {
                return Some(routing::bad_request_response("ContentID is required"));
            };
            if content_id.trim().is_empty() {
                return Some(routing::bad_request_response("ContentID is required"));
            }
            let key = format!("mediacore/descriptor/{content_id}");
            let existing = state.controller_features.read().await.get(&key).cloned();
            let was_published = existing
                .as_ref()
                .is_some_and(|value| value.get("publishedAt").is_some());
            if was_published {
                let Some(mut descriptor) = existing else {
                    unreachable!("publication record disappeared while holding a read snapshot")
                };
                if let Some(descriptor_object) = descriptor.as_object_mut() {
                    descriptor_object.remove("version");
                    descriptor_object.remove("publishedAt");
                    descriptor_object.remove("expiresAt");
                }
                if let Err(error) = state.controller_features.upsert(key, descriptor).await {
                    return Some(routing::service_unavailable_response(&error));
                }
            }
            return Some(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "contentId": content_id,
                    "wasPublished": was_published,
                })
                .to_string(),
            ));
        }
        let request = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(serde_json::Value::Object(value)) => serde_json::Value::Object(value),
            Ok(_) => {
                return Some(routing::bad_request_response(
                    "descriptor must be an object",
                ));
            }
            Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
        };
        if let Some(content_id) = route_id {
            if content_id.trim().is_empty() {
                return Some(routing::bad_request_response("ContentID is required"));
            }
            if request
                .get("updates")
                .and_then(serde_json::Value::as_object)
                .is_none()
            {
                return Some(routing::bad_request_response("Updates are required"));
            }
            return Some(routing::bad_request_response("Failed to update descriptor"));
        }
        let Some(descriptor) = request
            .get("descriptor")
            .cloned()
            .filter(|value| value.is_object())
        else {
            return Some(routing::bad_request_response("Descriptor is required"));
        };
        let Some(_content_id) = descriptor
            .get("contentId")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
        else {
            return Some(routing::bad_request_response(
                "Descriptor ContentID is required",
            ));
        };
        let force_update = request
            .get("forceUpdate")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let result = match mediacore_publish_descriptor(descriptor, force_update, state).await {
            Ok(result) => result,
            Err(error) => return Some(routing::service_unavailable_response(&error)),
        };
        return Some(if result["success"] == serde_json::Value::Bool(true) {
            let _ = record_mediacore_metric(
                state,
                "publish",
                serde_json::json!({
                    "count": 1,
                    "updated": usize::from(result["wasUpdated"].as_bool().unwrap_or(false)),
                }),
            )
            .await;
            routing::ok_response(result.to_string())
        } else {
            routing::bad_request_response("Failed to publish descriptor")
        });
    }
    if path == "/api/mediacore/publish/batch" {
        if json_array_field_exceeds_limit(body, "descriptors", MAX_MEDIACORE_BATCH_ITEMS) {
            return Some(routing::bad_request_response(
                "descriptors must contain at most 100 items",
            ));
        }
        let descriptors = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|value| {
                value
                    .get("descriptors")
                    .and_then(serde_json::Value::as_array)
                    .cloned()
            })
            .unwrap_or_default();
        if descriptors.is_empty() {
            return Some(routing::bad_request_response(
                "At least one descriptor is required",
            ));
        }
        if descriptors.iter().any(|descriptor| {
            descriptor
                .get("contentId")
                .and_then(serde_json::Value::as_str)
                .is_none_or(|value| value.trim().is_empty())
        }) {
            return Some(routing::bad_request_response(
                "Each descriptor requires a ContentID",
            ));
        }
        let started_at = Instant::now();
        let mut successfully_published = 0_usize;
        let mut failed_to_publish = 0_usize;
        let mut skipped = 0_usize;
        let mut results = Vec::with_capacity(descriptors.len());
        for descriptor in descriptors {
            let result = match mediacore_publish_descriptor(descriptor, false, state).await {
                Ok(result) => result,
                Err(error) => return Some(routing::service_unavailable_response(&error)),
            };
            if result["success"].as_bool().unwrap_or(false) {
                successfully_published = successfully_published.saturating_add(1);
            } else if result["errorMessage"]
                .as_str()
                .is_some_and(|error| error.contains("not newer"))
            {
                skipped = skipped.saturating_add(1);
            } else {
                failed_to_publish = failed_to_publish.saturating_add(1);
            }
            results.push(result);
        }
        let _ = record_mediacore_metric(
            state,
            "publish",
            serde_json::json!({"count": successfully_published, "updated": 0}),
        )
        .await;
        return Some(routing::ok_response(
            serde_json::json!({
                "totalRequested": results.len(),
                "successfullyPublished": successfully_published,
                "failedToPublish": failed_to_publish,
                "skipped": skipped,
                "totalDuration": format_timespan_millis(started_at.elapsed().as_millis() as u64),
                "results": results,
            })
            .to_string(),
        ));
    }
    if path == "/api/mediacore/publish/republish" {
        if json_array_field_exceeds_limit(body, "contentIds", MAX_MEDIACORE_BATCH_ITEMS) {
            return Some(routing::bad_request_response(
                "contentIds must contain at most 100 items",
            ));
        }
        let started_at = Instant::now();
        let request = serde_json::from_str::<serde_json::Value>(body).unwrap_or_default();
        let requested = request.get("contentIds");
        let requested_ids = requested.and_then(serde_json::Value::as_array).map(|ids| {
            let mut seen = HashSet::new();
            ids.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .filter(|id| seen.insert(id.to_ascii_lowercase()))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        });
        if requested.is_some() && requested_ids.as_ref().is_none_or(Vec::is_empty) {
            return Some(routing::bad_request_response(
                "At least one non-empty ContentID is required",
            ));
        }
        let descriptors = state
            .controller_features
            .read()
            .await
            .values_with_prefix("mediacore/descriptor/");
        let publication_records = descriptors
            .iter()
            .filter(|descriptor| descriptor.get("publishedAt").is_some())
            .collect::<Vec<_>>();
        let ids = requested_ids.unwrap_or_else(|| {
            publication_records
                .iter()
                .filter_map(|descriptor| {
                    descriptor
                        .get("contentId")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        });
        let now = chrono::Utc::now();
        let expiring_threshold = now + chrono::Duration::minutes(30);
        let mut still_valid = 0_usize;
        let mut failed = 0_usize;
        for content_id in &ids {
            let Some(descriptor) = publication_records.iter().find(|descriptor| {
                descriptor
                    .get("contentId")
                    .and_then(serde_json::Value::as_str)
                    == Some(content_id.as_str())
            }) else {
                continue;
            };
            let expires_at = descriptor
                .get("expiresAt")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&chrono::Utc))
                .unwrap_or(now);
            if expires_at <= expiring_threshold {
                failed = failed.saturating_add(1);
            } else {
                still_valid = still_valid.saturating_add(1);
            }
        }
        let _ = record_mediacore_metric(
            state,
            "publish",
            serde_json::json!({"count": 0, "republished": 0, "failed": failed}),
        )
        .await;
        return Some(routing::ok_response(
            serde_json::json!({
                "totalChecked": ids.len(),
                "republished": 0,
                "failed": failed,
                "stillValid": still_valid,
                "duration": format_timespan_millis(started_at.elapsed().as_millis() as u64),
            })
            .to_string(),
        ));
    }
    if path == "/api/mediacore/retrieve/batch" {
        if json_array_field_exceeds_limit(body, "contentIds", MAX_MEDIACORE_BATCH_ITEMS) {
            return Some(routing::bad_request_response(
                "contentIds must contain at most 100 items",
            ));
        }
        let ids = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|value| {
                value
                    .get("contentIds")
                    .and_then(serde_json::Value::as_array)
                    .map(|ids| {
                        let mut seen = HashSet::new();
                        ids.iter()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::trim)
                            .filter(|id| !id.is_empty())
                            .filter(|id| seen.insert(id.to_ascii_lowercase()))
                            .map(str::to_owned)
                            .collect::<Vec<_>>()
                    })
            })
            .unwrap_or_default();
        if ids.is_empty() {
            return Some(routing::bad_request_response(
                "At least one ContentID is required",
            ));
        }
        let started_at = Instant::now();
        let mut found = 0_usize;
        let mut failed = 0_usize;
        let mut results = Vec::with_capacity(ids.len());
        for content_id in &ids {
            match mediacore_retrieve_descriptor(content_id, false, state).await {
                Ok(result) => {
                    found += usize::from(result.found);
                    results.push(result.value);
                }
                Err(error) => {
                    failed = failed.saturating_add(1);
                    results.push(serde_json::json!({
                        "found": false,
                        "retrievedAt": chrono::Utc::now().to_rfc3339(),
                        "retrievalDuration": "00:00:00",
                        "fromCache": false,
                        "errorMessage": error,
                    }));
                }
            }
        }
        return Some(routing::ok_response(
            serde_json::json!({
                "requested": ids.len(),
                "found": found,
                "failed": failed,
                "totalDuration": format_timespan_millis(started_at.elapsed().as_millis() as u64),
                "results": results,
            })
            .to_string(),
        ));
    }
    if path == "/api/mediacore/retrieve/verify" {
        let request = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(request @ serde_json::Value::Object(_)) => request,
            Ok(_) => {
                return Some(routing::bad_request_response(
                    "descriptor must be an object",
                ));
            }
            Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
        };
        let Some(descriptor) = request.get("descriptor").filter(|value| value.is_object()) else {
            return Some(routing::bad_request_response("Descriptor is required"));
        };
        let retrieved_at = request
            .get("retrievedAt")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|timestamp| timestamp.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);
        let age_ms = chrono::Utc::now()
            .signed_duration_since(retrieved_at)
            .num_milliseconds()
            .max(0);
        let has_hashes = descriptor
            .get("hashes")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|hashes| !hashes.is_empty());
        let signature = descriptor
            .get("signature")
            .filter(|value| value.is_object());
        let signature_valid = signature.is_some_and(|signature| {
            signature
                .get("publicKey")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| !value.trim().is_empty())
                && signature
                    .get("signature")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| {
                        !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_hexdigit())
                    })
        });
        let _ = record_mediacore_metric(
            state,
            "retrieve",
            serde_json::json!({
                "count": 1,
                "hits": usize::from(has_hashes && signature_valid),
                "misses": usize::from(!(has_hashes && signature_valid)),
            }),
        )
        .await;
        return Some(routing::ok_response(
            serde_json::json!({
                "isValid": has_hashes && signature_valid,
                "signatureValid": signature_valid,
                "freshnessValid": has_hashes,
                "age": format!("00:00:{:02}.{:07}", age_ms / 1_000, (age_ms % 1_000) * 10_000),
                "validationError": if has_hashes { serde_json::Value::Null } else { serde_json::json!("At least one hash is required") },
            })
            .to_string(),
        ));
    }
    if path == "/api/mediacore/retrieve/cache/clear" {
        if json_array_field_exceeds_limit(body, "keys", MAX_MEDIACORE_BATCH_ITEMS) {
            return Some(routing::bad_request_response(
                "keys must contain at most 100 items",
            ));
        }
        let requested_keys = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|value| value.get("keys").cloned())
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        if requested_keys.len() > 100 {
            return Some(routing::bad_request_response(
                "At most 100 cache keys may be invalidated",
            ));
        }
        let result = state
            .controller_features
            .mutate(move |features| {
                let mut cache_keys = requested_keys
                    .into_iter()
                    .filter_map(|value| value.as_str().map(str::trim).map(ToOwned::to_owned))
                    .filter(|key| !key.is_empty())
                    .map(|key| {
                        if key.starts_with("mediacore/cache/") {
                            key.to_owned()
                        } else {
                            format!("mediacore/cache/{key}")
                        }
                    })
                    .collect::<Vec<_>>();
                cache_keys.sort_unstable();
                cache_keys.dedup();
                let cache_keys = if cache_keys.is_empty() {
                    features
                        .entries_with_prefix("mediacore/cache/")
                        .into_iter()
                        .map(|(key, _)| key)
                        .collect::<Vec<_>>()
                } else {
                    cache_keys
                };
                let bytes_freed = cache_keys
                    .iter()
                    .filter_map(|key| features.get(key))
                    .filter_map(|record| record.get("descriptor"))
                    .filter_map(|descriptor| serde_json::to_vec(descriptor).ok())
                    .map(|descriptor| descriptor.len() as u64)
                    .sum::<u64>();
                features
                    .remove_keys(&cache_keys)
                    .map(|entries_cleared| (entries_cleared, bytes_freed))
            })
            .await;
        return Some(match result {
            Ok((entries_cleared, bytes_freed)) => routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "entriesCleared": entries_cleared,
                    "bytesFreed": bytes_freed,
                })
                .to_string(),
            ),
            Err(error) => routing::service_unavailable_response(&error),
        });
    }
    if path.starts_with("/api/mediacore/portability/") {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload) => payload,
            Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
        };
        let action = path.rsplit('/').next().unwrap_or_default();
        return Some(match action {
            "analyze" => {
                let Some(package) = payload.get("package").filter(|value| value.is_object()) else {
                    return Some(routing::bad_request_response(
                        "Metadata package is required",
                    ));
                };
                if package.get("entries").is_some_and(|entries| {
                    json_array_exceeds_limit(entries, MAX_MEDIACORE_PORTABILITY_ENTRIES)
                }) {
                    return Some(routing::bad_request_response(
                        "entries must contain at most 1000 items",
                    ));
                }
                let entries = package
                    .get("entries")
                    .and_then(serde_json::Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let features = state.controller_features.read().await;
                let conflicts = entries
                    .iter()
                    .filter_map(|entry| entry.get("contentId").and_then(serde_json::Value::as_str))
                    .filter(|content_id| {
                        features
                            .get(&format!("mediacore/descriptor/{content_id}"))
                            .is_some()
                    })
                    .count();
                routing::ok_response(
                    serde_json::json!({
                        "totalEntries": entries.len(),
                        "conflictingEntries": conflicts,
                        "cleanEntries": entries.len().saturating_sub(conflicts),
                        "conflicts": [],
                        "recommendedStrategies": {
                            "Merge": 0,
                            "Overwrite": 0,
                            "Skip": 0,
                        },
                    })
                    .to_string(),
                )
            }
            "export" => {
                if payload.get("contentIds").is_some_and(|content_ids| {
                    json_array_exceeds_limit(content_ids, MAX_MEDIACORE_PORTABILITY_ENTRIES)
                }) {
                    return Some(routing::bad_request_response(
                        "contentIds must contain at most 1000 items",
                    ));
                }
                let content_ids = payload
                    .get("contentIds")
                    .and_then(serde_json::Value::as_array)
                    .map(|ids| {
                        ids.iter()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::trim)
                            .filter(|id| !id.is_empty())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if content_ids.is_empty() {
                    return Some(routing::bad_request_response(
                        "At least one ContentID is required for export",
                    ));
                }
                let source_name = if state.config.controller_profile == ControllerProfile::Native {
                    "slskr"
                } else {
                    "slskR"
                };
                let features = state.controller_features.read().await;
                let entries = content_ids
                    .iter()
                    .filter_map(|content_id| {
                        features
                            .get(&format!("mediacore/descriptor/{content_id}"))
                            .cloned()
                            .map(|descriptor| {
                                serde_json::json!({
                                    "contentId": content_id,
                                    "descriptor": descriptor,
                                    "sourceInfo": {
                                        "name": source_name,
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                        "version": "1.0.0",
                                        "properties": {"exported":"true", "from_cache":"false"},
                                    },
                                })
                            })
                    })
                    .collect::<Vec<_>>();
                let exported_count = entries.len();
                drop(features);
                let checksum = hex::encode(Sha256::digest(
                    serde_json::to_vec(&entries).unwrap_or_default(),
                ));
                routing::ok_response(
                    serde_json::json!({
                        "version": "1.0",
                        "exportedAt": chrono::Utc::now().to_rfc3339(),
                        "source": source_name,
                        "entries": entries,
                        "links": [],
                        "metadata": {
                            "totalEntries": exported_count,
                            "totalLinks": 0,
                            "entriesByDomain": {},
                            "checksum": checksum,
                        },
                    })
                    .to_string(),
                )
            }
            "import" => {
                let Some(package) = payload.get("package").filter(|value| value.is_object()) else {
                    return Some(routing::bad_request_response(
                        "Metadata package is required",
                    ));
                };
                if package.get("entries").is_some_and(|entries| {
                    json_array_exceeds_limit(entries, MAX_MEDIACORE_PORTABILITY_ENTRIES)
                }) {
                    return Some(routing::bad_request_response(
                        "entries must contain at most 1000 items",
                    ));
                }
                let entries = package
                    .get("entries")
                    .and_then(serde_json::Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                if entries.is_empty() {
                    return Some(routing::ok_response(
                        serde_json::json!({
                            "success": true,
                            "entriesProcessed": 0,
                            "entriesImported": 0,
                            "entriesSkipped": 0,
                            "conflictsResolved": 0,
                            "conflicts": [],
                            "errors": [],
                            "duration": "00:00:00",
                        })
                        .to_string(),
                    ));
                }
                let strategy = payload
                    .get("strategy")
                    .or_else(|| package.get("strategy"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Merge")
                    .trim()
                    .to_ascii_lowercase();
                if !matches!(
                    strategy.as_str(),
                    "merge" | "overwrite" | "skip" | "keepexisting"
                ) {
                    return Some(routing::bad_request_response("Unsupported import strategy"));
                }
                let entries_processed = entries.len().min(MAX_MEDIACORE_PORTABILITY_ENTRIES);
                let import_result = state
                    .controller_features
                    .mutate(move |features| {
                        let mut imported = 0_usize;
                        let mut skipped = 0_usize;
                        let mut conflicts = 0_usize;
                        let mut errors = Vec::new();
                        for entry in entries.iter().take(MAX_MEDIACORE_PORTABILITY_ENTRIES) {
                            let descriptor = entry
                                .get("descriptor")
                                .filter(|value| value.is_object())
                                .unwrap_or(entry);
                            let Some(content_id) = descriptor
                                .get("contentId")
                                .or_else(|| entry.get("contentId"))
                                .and_then(serde_json::Value::as_str)
                                .map(str::trim)
                                .filter(|value| !value.is_empty())
                            else {
                                errors.push("Each import entry requires a ContentID".to_owned());
                                continue;
                            };
                            let key = format!("mediacore/descriptor/{content_id}");
                            let existing = features.get(&key).cloned();
                            let mut candidate = descriptor.clone();
                            if let Some(existing) = existing {
                                conflicts = conflicts.saturating_add(1);
                                match strategy.as_str() {
                                    "skip" | "keepexisting" => {
                                        skipped = skipped.saturating_add(1);
                                        continue;
                                    }
                                    "merge" => {
                                        let Some(existing) = existing.as_object() else {
                                            errors.push(format!(
                                                "Descriptor {content_id} is not a JSON object"
                                            ));
                                            continue;
                                        };
                                        let Some(candidate_object) = candidate.as_object() else {
                                            errors.push(format!(
                                                "Descriptor {content_id} is not a JSON object"
                                            ));
                                            continue;
                                        };
                                        let mut merged = existing.clone();
                                        for (field, value) in candidate_object {
                                            merged.insert(field.clone(), value.clone());
                                        }
                                        candidate = serde_json::Value::Object(merged);
                                    }
                                    "overwrite" => {}
                                    _ => unreachable!("import strategy validated above"),
                                }
                            }
                            if let Some(candidate) = candidate.as_object_mut() {
                                candidate
                                    .insert("contentId".to_owned(), serde_json::json!(content_id));
                                candidate.insert(
                                    "importedAt".to_owned(),
                                    serde_json::json!(chrono::Utc::now().to_rfc3339()),
                                );
                            }
                            features.upsert(key, candidate)?;
                            imported = imported.saturating_add(1);
                        }
                        Ok((imported, skipped, conflicts, errors))
                    })
                    .await;
                let (imported, skipped, conflicts, errors) = match import_result {
                    Ok(result) => result,
                    Err(error) => return Some(routing::service_unavailable_response(&error)),
                };
                routing::ok_response(
                    serde_json::json!({
                        "success": errors.is_empty(),
                        "entriesProcessed": entries_processed,
                        "entriesImported": imported,
                        "entriesSkipped": skipped,
                        "conflictsResolved": conflicts.saturating_sub(skipped),
                        "conflicts": [],
                        "errors": errors,
                        "duration": "00:00:00",
                    })
                    .to_string(),
                )
            }
            _ => return None,
        });
    }
    if path == "/api/mediacore/stats/reset" {
        return Some(
            match state
                .controller_features
                .remove_prefix("mediacore/metrics/")
                .await
            {
                Ok(_) => routing::ok_response(
                    serde_json::json!({"message": "Statistics reset successfully"}).to_string(),
                ),
                Err(error) => routing::service_unavailable_response(&error),
            },
        );
    }
    None
}
