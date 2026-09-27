use super::*;

#[path = "mediacore_extended.rs"]
mod extended;
#[path = "mediacore_mutations.rs"]
mod mutations;

pub(super) use self::extended::mediacore_extended_response;
pub(super) use self::mutations::mediacore_mutation_response;

async fn record_mediacore_metric(
    state: &AppState,
    operation: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    let id = uuid::Uuid::new_v4().simple().to_string();
    let record = serde_json::json!({
        "operation": operation,
        "recordedAt": unix_timestamp_millis(),
        "value": value,
    });
    state
        .controller_features
        .upsert(format!("mediacore/metrics/{id}"), record)
        .await
}

struct MediaCoreRetrievalResult {
    found: bool,
    value: serde_json::Value,
}

fn mediacore_descriptor_verification(
    descriptor: &serde_json::Value,
    retrieved_at: chrono::DateTime<chrono::Utc>,
) -> serde_json::Value {
    let age_ms = chrono::Utc::now()
        .signed_duration_since(retrieved_at)
        .num_milliseconds()
        .max(0) as u64;
    let has_hashes = descriptor
        .get("hashes")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|hashes| !hashes.is_empty());
    let signature_valid = descriptor
        .get("signature")
        .filter(|value| value.is_object())
        .is_some_and(|signature| {
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
    serde_json::json!({
        "isValid": has_hashes && signature_valid,
        "signatureValid": signature_valid,
        "freshnessValid": has_hashes,
        "age": format_timespan_millis(age_ms),
        "validationError": if has_hashes { serde_json::Value::Null } else { serde_json::json!("At least one hash is required") },
    })
}

async fn mediacore_retrieve_descriptor(
    content_id: &str,
    bypass_cache: bool,
    state: &AppState,
) -> Result<MediaCoreRetrievalResult, String> {
    let started_at = Instant::now();
    let now = chrono::Utc::now();
    let cache_key = format!("mediacore/cache/{content_id}");
    if !bypass_cache {
        let cached = state
            .controller_features
            .read()
            .await
            .get(&cache_key)
            .cloned();
        if let Some(cached) = cached {
            let expires_at = cached
                .get("expiresAt")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&chrono::Utc));
            if let (Some(descriptor), Some(expires_at)) =
                (cached.get("descriptor").cloned(), expires_at)
            {
                if expires_at > now {
                    let retrieved_at = cached
                        .get("retrievedAt")
                        .and_then(serde_json::Value::as_str)
                        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                        .map(|value| value.with_timezone(&chrono::Utc))
                        .unwrap_or(now);
                    let _ = record_mediacore_metric(
                        state,
                        "retrieve",
                        serde_json::json!({"count": 1, "hits": 1, "misses": 0}),
                    )
                    .await;
                    return Ok(MediaCoreRetrievalResult {
                        found: true,
                        value: serde_json::json!({
                            "found": true,
                            "descriptor": descriptor.clone(),
                            "retrievedAt": retrieved_at.to_rfc3339(),
                            "retrievalDuration": format_timespan_millis(started_at.elapsed().as_millis() as u64),
                            "fromCache": true,
                            "verification": mediacore_descriptor_verification(&descriptor, retrieved_at),
                        }),
                    });
                }
            }
        }
    }

    let descriptor = state
        .controller_features
        .read()
        .await
        .get(&format!("mediacore/descriptor/{content_id}"))
        .cloned();
    let retrieved_at = chrono::Utc::now();
    let Some(descriptor) = descriptor else {
        let _ = record_mediacore_metric(
            state,
            "retrieve",
            serde_json::json!({"count": 1, "hits": 0, "misses": 1}),
        )
        .await;
        return Ok(MediaCoreRetrievalResult {
            found: false,
            value: serde_json::json!({
                "found": false,
                "retrievedAt": retrieved_at.to_rfc3339(),
                "retrievalDuration": format_timespan_millis(started_at.elapsed().as_millis() as u64),
                "fromCache": false,
            }),
        });
    };

    let expires_at = retrieved_at + chrono::Duration::hours(1);
    let cache_record = serde_json::json!({
        "contentId": content_id,
        "descriptor": descriptor,
        "retrievedAt": retrieved_at.to_rfc3339(),
        "expiresAt": expires_at.to_rfc3339(),
    });
    state
        .controller_features
        .upsert(cache_key, cache_record)
        .await?;
    let _ = record_mediacore_metric(
        state,
        "retrieve",
        serde_json::json!({"count": 1, "hits": 0, "misses": 1}),
    )
    .await;
    Ok(MediaCoreRetrievalResult {
        found: true,
        value: serde_json::json!({
            "found": true,
            "descriptor": descriptor,
            "retrievedAt": retrieved_at.to_rfc3339(),
            "retrievalDuration": format_timespan_millis(started_at.elapsed().as_millis() as u64),
            "fromCache": false,
            "verification": mediacore_descriptor_verification(&descriptor, retrieved_at),
        }),
    })
}

pub(super) fn levenshtein_similarity(left: &str, right: &str) -> f64 {
    let left = left.to_lowercase().chars().collect::<Vec<_>>();
    let right = right.to_lowercase().chars().collect::<Vec<_>>();
    if left.is_empty() || right.is_empty() {
        return f64::from(left.is_empty() && right.is_empty());
    }
    let max_length = left.len().max(right.len());
    if left == right {
        return 1.0;
    }
    let shared_prefix = left
        .iter()
        .zip(&right)
        .take_while(|(left, right)| left == right)
        .count();
    let mut left = &left[shared_prefix..];
    let mut right = &right[shared_prefix..];
    let shared_suffix = left
        .iter()
        .rev()
        .zip(right.iter().rev())
        .take_while(|(left, right)| left == right)
        .count();
    if shared_suffix > 0 {
        left = &left[..left.len() - shared_suffix];
        right = &right[..right.len() - shared_suffix];
    }
    if left.is_empty() || right.is_empty() {
        return 1.0 - left.len().max(right.len()) as f64 / max_length as f64;
    }
    if right.len() > left.len() {
        std::mem::swap(&mut left, &mut right);
    }
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    for (left_index, left_character) in left.iter().enumerate() {
        let mut current = Vec::with_capacity(right.len() + 1);
        current.push(left_index + 1);
        for (right_index, right_character) in right.iter().enumerate() {
            current.push(if left_character == right_character {
                previous[right_index]
            } else {
                1 + previous[right_index]
                    .min(current[right_index])
                    .min(previous[right_index + 1])
            });
        }
        previous = current;
    }
    1.0 - previous[right.len()] as f64 / max_length as f64
}

fn soundex(value: &str) -> [char; 4] {
    let letters = value
        .to_uppercase()
        .chars()
        .filter(|character| character.is_alphabetic())
        .collect::<Vec<_>>();
    let Some(first) = letters.first().copied() else {
        return ['0'; 4];
    };
    let code = |character| match character {
        'B' | 'F' | 'P' | 'V' => '1',
        'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => '2',
        'D' | 'T' => '3',
        'L' => '4',
        'M' | 'N' => '5',
        'R' => '6',
        _ => '0',
    };
    let mut result = ['0'; 4];
    result[0] = first;
    let mut previous = code(first);
    let mut output = 1;
    for character in letters.into_iter().skip(1) {
        let current = code(character);
        if current != '0' && current != previous && output < result.len() {
            result[output] = current;
            output += 1;
        }
        previous = current;
    }
    result
}

fn phonetic_similarity(left: &str, right: &str) -> f64 {
    if left.is_empty() || right.is_empty() {
        return f64::from(left.is_empty() && right.is_empty());
    }
    let left = soundex(left);
    let right = soundex(right);
    if left == right {
        1.0
    } else if left[0] == right[0] {
        0.5
    } else {
        0.0
    }
}

fn normalized_content_domain(content_id: &str) -> Option<String> {
    let mut parts = content_id.trim().splitn(4, ':');
    if !parts.next()?.eq_ignore_ascii_case("content") {
        return None;
    }
    let domain = parts.next()?.trim();
    let content_type = parts.next()?.trim();
    let identifier = parts.next()?.trim();
    if domain.is_empty() || content_type.is_empty() || identifier.is_empty() {
        return None;
    }
    if domain.eq_ignore_ascii_case("mb")
        && matches!(
            content_type.to_ascii_lowercase().as_str(),
            "recording" | "release" | "artist"
        )
    {
        Some("audio".to_owned())
    } else {
        Some(domain.to_ascii_lowercase())
    }
}

fn descriptor_numeric_hash(descriptor: &serde_json::Value) -> Option<u64> {
    let hashes = descriptor
        .get("perceptualHashes")
        .and_then(serde_json::Value::as_array)?;
    let numeric_hash =
        |hash: &serde_json::Value| hash.get("numericHash").and_then(serde_json::Value::as_u64);
    hashes
        .iter()
        .find(|hash| {
            hash.get("algorithm").and_then(serde_json::Value::as_str) == Some("Chromaprint")
                && numeric_hash(hash).is_some()
        })
        .and_then(numeric_hash)
        .or_else(|| hashes.iter().find_map(numeric_hash))
}

fn descriptor_perceptual_similarity(
    content_id_a: &str,
    content_id_b: &str,
    descriptors: &[serde_json::Value],
) -> f64 {
    let (Some(domain_a), Some(domain_b)) = (
        normalized_content_domain(content_id_a),
        normalized_content_domain(content_id_b),
    ) else {
        return 0.0;
    };
    if !domain_a.eq_ignore_ascii_case(&domain_b) {
        return 0.0;
    }
    let descriptor_for = |content_id: &str| {
        descriptors.iter().find(|descriptor| {
            descriptor
                .get("contentId")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.eq_ignore_ascii_case(content_id))
        })
    };
    let (Some(left), Some(right)) = (descriptor_for(content_id_a), descriptor_for(content_id_b))
    else {
        return 0.0;
    };
    let (Some(left), Some(right)) = (
        descriptor_numeric_hash(left),
        descriptor_numeric_hash(right),
    ) else {
        return 0.0;
    };
    1.0 - f64::from((left ^ right).count_ones()) / 64.0
}

/// Persist bounded MediaCore operation counters alongside the feature state.
/// The sibling keeps these counters in its MediaCore services; using the
/// existing atomic feature store gives slskR the same restart-visible API
/// behavior without introducing a second persistence backend.
fn mediacore_publish_version(
    descriptor: &serde_json::Value,
    published_at: chrono::DateTime<chrono::Utc>,
) -> String {
    let content_id = descriptor
        .get("contentId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let codec = descriptor
        .get("codec")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let size = descriptor
        .get("sizeBytes")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();
    let version_seed = descriptor
        .get("bitrateKbps")
        .and_then(serde_json::Value::as_i64)
        .map_or_else(
            || format!("{content_id}:{codec}:{size}"),
            |bitrate| format!("{content_id}:{codec}:{bitrate}:{size}"),
        );
    format!(
        "{}-{}",
        published_at.timestamp_millis(),
        &hex::encode(Sha256::digest(version_seed.as_bytes()))[..8]
    )
}

fn mediacore_descriptor_passes_base_validation(
    descriptor: &serde_json::Value,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    let bitrate_is_valid = descriptor.get("bitrateKbps").is_none_or(|value| {
        value
            .as_i64()
            .is_some_and(|bitrate| (1..=10_000).contains(&bitrate))
    });
    let has_hashes = descriptor
        .get("hashes")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|hashes| !hashes.is_empty());
    let signature_is_fresh = descriptor
        .get("signature")
        .filter(|value| value.is_object())
        .and_then(|signature| signature.get("timestampUnixMs"))
        .and_then(serde_json::Value::as_i64)
        .is_some_and(|timestamp| now.timestamp_millis().saturating_sub(timestamp) <= 3_600_000);
    let descriptor_size_is_bounded = serde_json::to_vec(descriptor)
        .map(|bytes| bytes.len() <= 10 * 1024)
        .unwrap_or(false);
    bitrate_is_valid && has_hashes && signature_is_fresh && descriptor_size_is_bounded
}

async fn mediacore_publish_descriptor(
    mut descriptor: serde_json::Value,
    force_update: bool,
    state: &AppState,
) -> Result<serde_json::Value, String> {
    let content_id = descriptor
        .get("contentId")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_default();
    let published_at = chrono::Utc::now();
    if descriptor.get("isAdvertisable") == Some(&serde_json::Value::Bool(false)) {
        return Ok(serde_json::json!({
            "success": false,
            "contentId": content_id,
            "version": "0",
            "publishedAt": published_at.to_rfc3339(),
            "ttl": "00:00:00",
            "errorMessage": "Content is not advertisable",
            "wasUpdated": false,
            "previousVersion": null,
        }));
    }

    let version = mediacore_publish_version(&descriptor, published_at);
    let key = format!("mediacore/descriptor/{content_id}");
    let existing = state.controller_features.read().await.get(&key).cloned();
    let previous_version = existing
        .as_ref()
        .filter(|value| value.get("publishedAt").is_some())
        .and_then(|value| value.get("version"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    if !force_update
        && previous_version
            .as_deref()
            .is_some_and(|previous| version.as_str() <= previous)
    {
        return Ok(serde_json::json!({
            "success": false,
            "contentId": content_id,
            "version": version,
            "publishedAt": published_at.to_rfc3339(),
            "ttl": "01:00:00",
            "errorMessage": format!("Version {version} is not newer than existing {}", previous_version.as_deref().unwrap_or_default()),
            "wasUpdated": false,
            "previousVersion": previous_version,
        }));
    }
    if descriptor
        .get("signature")
        .is_none_or(serde_json::Value::is_null)
    {
        return Ok(serde_json::json!({
            "success": false,
            "contentId": content_id,
            "version": version,
            "publishedAt": published_at.to_rfc3339(),
            "ttl": "01:00:00",
            "errorMessage": "Descriptor signature is required; provide a signed descriptor before publishing.",
            "wasUpdated": false,
            "previousVersion": previous_version,
        }));
    }
    if !mediacore_descriptor_passes_base_validation(&descriptor, published_at) {
        return Ok(serde_json::json!({
            "success": false,
            "contentId": content_id,
            "version": version,
            "publishedAt": published_at.to_rfc3339(),
            "ttl": "01:00:00",
            "errorMessage": "Base publisher failed",
            "wasUpdated": false,
            "previousVersion": previous_version,
        }));
    }

    descriptor["contentId"] = serde_json::json!(content_id);
    descriptor["version"] = serde_json::json!(version);
    descriptor["publishedAt"] = serde_json::json!(published_at.to_rfc3339());
    descriptor["expiresAt"] =
        serde_json::json!((published_at + chrono::Duration::hours(1)).to_rfc3339());
    state.controller_features.upsert(key, descriptor).await?;
    Ok(serde_json::json!({
        "success": true,
        "contentId": content_id,
        "version": version,
        "publishedAt": published_at.to_rfc3339(),
        "ttl": "01:00:00",
        "wasUpdated": previous_version.is_some(),
        "previousVersion": previous_version,
    }))
}
