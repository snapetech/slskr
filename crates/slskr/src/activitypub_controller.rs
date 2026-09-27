use super::*;

fn activitypub_response(value: serde_json::Value) -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "application/activity+json",
        body: value.to_string(),
    }
}

fn webfinger_response(value: serde_json::Value) -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "application/jrd+json",
        body: value.to_string(),
    }
}

pub(super) fn social_federation_is_active(config: &AppConfig) -> bool {
    config.social_federation.enabled
        && !config.social_federation.mode.eq_ignore_ascii_case("Hermit")
}

fn activitypub_base_url(state: &AppState) -> String {
    state
        .config
        .social_federation
        .base_url
        .as_deref()
        .filter(|value| {
            reqwest::Url::parse(value).is_ok_and(|url| {
                matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
            })
        })
        .map(|value| value.trim_end_matches('/').to_owned())
        .unwrap_or_else(|| format!("http://{}", state.config.http_bind))
}

fn activitypub_domain(state: &AppState) -> String {
    state
        .config
        .social_federation
        .domain
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_owned())
        .or_else(|| {
            reqwest::Url::parse(&activitypub_base_url(state))
                .ok()
                .and_then(|url| url.host_str().map(str::to_owned))
        })
        .unwrap_or_else(|| state.config.http_bind.ip().to_string())
}

pub(super) fn activitypub_public_key_pem(state: &AppState) -> String {
    // SubjectPublicKeyInfo for Ed25519: RFC 8410's fixed algorithm prefix
    // followed by the raw 32-byte verifying key used by slskR's capability
    // signer.  This gives ActivityPub peers a real public key instead of a
    // placeholder while keeping the key stable with the persisted identity.
    const ED25519_SPKI_PREFIX: [u8; 12] = [
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    let mut der = Vec::with_capacity(ED25519_SPKI_PREFIX.len() + 32);
    der.extend_from_slice(&ED25519_SPKI_PREFIX);
    der.extend_from_slice(state.capability_signing_key.verifying_key().as_bytes());
    format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
        STANDARD.encode(der)
    )
}

/// A parsed `Signature` request header (RFC draft "Signing HTTP Messages"),
/// matching the frozen native profile oracle's `TryParseSignature`.
pub(super) struct ActivityPubSignature {
    pub(super) key_id: String,
    pub(super) algorithm: String,
    pub(super) headers: Vec<String>,
    pub(super) signature_b64: String,
    pub(super) created: Option<i64>,
}

pub(super) fn parse_activitypub_signature_header(value: &str) -> Option<ActivityPubSignature> {
    let mut key_id = None;
    let mut algorithm = None;
    let mut headers_list = None;
    let mut signature_b64 = None;
    let mut created = None;
    for part in value.split(',') {
        let Some((key, raw_value)) = part.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let mut v = raw_value.trim();
        if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
            v = &v[1..v.len() - 1];
        }
        match key {
            "keyId" => key_id = Some(v.to_owned()),
            "algorithm" => algorithm = Some(v.to_owned()),
            "headers" => headers_list = Some(v.to_owned()),
            "signature" => signature_b64 = Some(v.to_owned()),
            "created" => created = v.parse::<i64>().ok(),
            _ => {}
        }
    }
    let key_id = key_id.filter(|value| !value.is_empty())?;
    let algorithm = algorithm.filter(|value| !value.is_empty())?;
    let headers_list = headers_list.filter(|value| !value.is_empty())?;
    let signature_b64 = signature_b64.filter(|value| !value.is_empty())?;
    Some(ActivityPubSignature {
        key_id,
        algorithm,
        headers: headers_list
            .split(' ')
            .filter(|segment| !segment.is_empty())
            .map(str::to_owned)
            .collect(),
        signature_b64,
        created,
    })
}

/// Matches the oracle's real `HasRequiredSignedHeaders`: `(request-target)`
/// and `host` are always required, freshness must be provable via either
/// `date` or `(created)`, and a request carrying a body must sign `digest`.
/// No signed header name may repeat.
pub(super) fn activitypub_signature_has_required_headers(
    headers: &[String],
    body_bearing: bool,
) -> bool {
    let mut seen = std::collections::HashSet::new();
    for header in headers {
        if !seen.insert(header.to_ascii_lowercase()) {
            return false;
        }
    }
    let has = |name: &str| {
        headers
            .iter()
            .any(|header| header.eq_ignore_ascii_case(name))
    };
    has("(request-target)")
        && has("host")
        && (has("date") || has("(created)"))
        && (!body_bearing || has("digest"))
}

pub(super) fn activitypub_signature_date_is_fresh(date: &str) -> bool {
    chrono::DateTime::parse_from_rfc2822(date)
        .map(|parsed| {
            (chrono::Utc::now().signed_duration_since(parsed))
                .num_seconds()
                .abs()
                <= 300
        })
        .unwrap_or(false)
}

pub(super) fn activitypub_signature_created_is_fresh(created: i64) -> bool {
    let now = i64::try_from(unix_timestamp()).unwrap_or(i64::MAX);
    (now - created).abs() <= 300
}

/// Matches the oracle's real `BuildSigningString`. Only the pseudo-headers
/// and headers slskR actually threads through from the raw request
/// (`(request-target)`, `host`, `date`, `digest`, `(created)`) can be
/// reconstructed -- a peer that signs any other header name fails closed
/// here rather than silently building an incomplete signing string.
fn activitypub_signing_string(
    parsed: &ActivityPubSignature,
    method: &str,
    path: &str,
    query: Option<&str>,
    headers: &RequestSecurityHeaders,
) -> Option<String> {
    let mut lines = Vec::with_capacity(parsed.headers.len());
    for name in &parsed.headers {
        let line = match name.to_ascii_lowercase().as_str() {
            "(request-target)" => {
                let target = match query {
                    Some(query) if !query.is_empty() => format!("{path}?{query}"),
                    _ => path.to_owned(),
                };
                format!("(request-target): {} {target}", method.to_ascii_lowercase())
            }
            "host" => format!("host: {}", headers.host.as_deref()?),
            "date" => format!("date: {}", headers.date.as_deref()?),
            "digest" => format!("digest: {}", headers.digest.as_deref()?),
            "(created)" => format!("(created): {}", parsed.created?),
            _ => return None,
        };
        lines.push(line);
    }
    Some(lines.join("\n"))
}

/// SSRF-safe fetch of a remote ActivityPub actor's Ed25519 public key,
/// reusing the same DNS-resolve-then-pin pattern and blocked-IP ranges
/// already proven for integration URLs (`validate_integration_base_url`).
async fn fetch_activitypub_remote_public_key(key_id: &str) -> Result<VerifyingKey, String> {
    let actor_url = key_id.split('#').next().unwrap_or(key_id);
    let resolved = validate_integration_base_url(actor_url)?;
    let mut client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy();
    for addr in &resolved.addrs {
        client_builder = client_builder.resolve(&resolved.host, *addr);
    }
    let client = client_builder
        .build()
        .map_err(|error| format!("failed to build ActivityPub key-fetch client: {error}"))?;
    let response = client
        .get(actor_url)
        .header("Accept", "application/activity+json")
        .send()
        .await
        .map_err(|error| format!("ActivityPub actor fetch failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "ActivityPub actor fetch returned HTTP {}",
            response.status()
        ));
    }
    let document = read_bounded_integration_json(response, "ActivityPub actor").await?;
    let pem = document
        .get("publicKey")
        .and_then(|key| key.get("publicKeyPem"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "actor document is missing publicKey.publicKeyPem".to_owned())?;
    decode_ed25519_pkix_public_key(pem)
}

/// Decodes the same SubjectPublicKeyInfo shape `activitypub_public_key_pem`
/// emits: RFC 8410's fixed 12-byte Ed25519 algorithm prefix followed by the
/// raw 32-byte verifying key, PEM-wrapped.
pub(super) fn decode_ed25519_pkix_public_key(pem: &str) -> Result<VerifyingKey, String> {
    const ED25519_SPKI_PREFIX: [u8; 12] = [
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    let base64_body: String = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();
    let der = STANDARD
        .decode(base64_body.trim())
        .map_err(|_| "publicKeyPem is not valid base64".to_owned())?;
    let raw_key = der
        .strip_prefix(&ED25519_SPKI_PREFIX)
        .ok_or_else(|| "publicKeyPem is not an Ed25519 SubjectPublicKeyInfo".to_owned())?;
    let raw_key: [u8; 32] = raw_key
        .try_into()
        .map_err(|_| "Ed25519 public key must be 32 bytes".to_owned())?;
    VerifyingKey::from_bytes(&raw_key)
        .map_err(|error| format!("invalid Ed25519 public key: {error}"))
}

/// Matches the oracle's real `VerifyHttpSignatureAsync`: parses and
/// validates the `Signature` header, enforces freshness (`Date` or
/// `created`, +/-5 minutes) and a real `Digest` match for body-bearing
/// requests, fetches the signer's real public key (SSRF-safe), and
/// verifies the real Ed25519 signature over the reconstructed signing
/// string. Returns the verified `keyId` on success so the caller can bind
/// it to the activity's own declared actor. HARDENING-2026-04-20 H4 in the
/// frozen oracle: signature verification is always enforced regardless of
/// the `federation.verify_signatures` config toggle, matched here by never
/// consulting that flag at all.
pub(super) async fn verify_activitypub_inbox_signature(
    method: &str,
    path: &str,
    query: Option<&str>,
    body: &str,
    headers: &RequestSecurityHeaders,
) -> Result<String, &'static str> {
    use sha2::Digest;
    let signature_header = headers
        .signature
        .as_deref()
        .ok_or("missing Signature header")?;
    let parsed =
        parse_activitypub_signature_header(signature_header).ok_or("malformed Signature header")?;
    if !matches!(
        parsed.algorithm.to_ascii_lowercase().as_str(),
        "ed25519" | "hs2019"
    ) {
        return Err("unsupported signature algorithm");
    }
    let body_bearing = !body.is_empty();
    if !activitypub_signature_has_required_headers(&parsed.headers, body_bearing) {
        return Err("missing required signed headers");
    }
    let signs_date = parsed
        .headers
        .iter()
        .any(|header| header.eq_ignore_ascii_case("date"));
    if signs_date {
        let date = headers.date.as_deref().ok_or("missing Date header")?;
        if !activitypub_signature_date_is_fresh(date) {
            return Err("stale or invalid Date header");
        }
    } else {
        let created = parsed.created.ok_or("missing created parameter")?;
        if !activitypub_signature_created_is_fresh(created) {
            return Err("stale created timestamp");
        }
    }
    let signs_digest = parsed
        .headers
        .iter()
        .any(|header| header.eq_ignore_ascii_case("digest"));
    if body_bearing || signs_digest {
        let digest = headers.digest.as_deref().ok_or("missing Digest header")?;
        let mut hasher = Sha256::new();
        Digest::update(&mut hasher, body.as_bytes());
        let expected = format!("SHA-256={}", STANDARD.encode(Digest::finalize(hasher)));
        if digest != expected {
            return Err("Digest header does not match body");
        }
    }
    let signing_string = activitypub_signing_string(&parsed, method, path, query, headers)
        .ok_or("cannot reconstruct signing string for the signed headers")?;
    let public_key = fetch_activitypub_remote_public_key(&parsed.key_id)
        .await
        .map_err(|_| "failed to fetch signer public key")?;
    let signature_bytes = STANDARD
        .decode(&parsed.signature_b64)
        .map_err(|_| "invalid signature encoding")?;
    let signature_bytes: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| "signature must be 64 bytes")?;
    let signature = Signature::from_bytes(&signature_bytes);
    public_key
        .verify(signing_string.as_bytes(), &signature)
        .map_err(|_| "signature verification failed")?;
    Ok(parsed.key_id)
}

/// Matches the oracle's real `IsActivityActorBoundToSignature`: the
/// activity's own declared `actor` must equal the verified key's actor
/// identity, or the verified key must be scoped under that actor (a
/// `#fragment` or `/`-suffixed key belonging to it).
pub(super) fn activitypub_actor_bound_to_signature(
    activity: &serde_json::Value,
    verified_key_id: &str,
) -> bool {
    let Some(actor) = activity.get("actor").and_then(serde_json::Value::as_str) else {
        return false;
    };
    if actor.is_empty() {
        return false;
    }
    actor.eq_ignore_ascii_case(verified_key_id)
        || verified_key_id
            .to_ascii_lowercase()
            .starts_with(&format!("{}#", actor.to_ascii_lowercase()))
        || verified_key_id
            .to_ascii_lowercase()
            .starts_with(&format!("{}/", actor.to_ascii_lowercase()))
}

pub(super) async fn activitypub_actor_exists(actor: &str, state: &AppState) -> bool {
    if !social_federation_is_active(&state.config) {
        return false;
    }

    // native profile's LibraryActorService always registers its music actor when the
    // music content provider is available.  slskR's share index is that
    // provider boundary, so the native actor is available whenever the
    // federation service is active.  Generic target actors intentionally
    // remain unavailable until their domain provider exists.
    if actor == "music" {
        return true;
    }

    // Preserve the older controller-feature records as a compatibility path
    // for explicitly materialized test/custom actors.  They are not used to
    // fabricate the native library actor above.
    let prefix = format!("activitypub/{actor}/");
    !state
        .controller_features
        .read()
        .await
        .values_with_prefix(&prefix)
        .is_empty()
}

fn activitypub_value_string(value: Option<&serde_json::Value>) -> Option<String> {
    match value {
        Some(serde_json::Value::String(value)) if !value.trim().is_empty() => {
            Some(value.trim().to_owned())
        }
        Some(serde_json::Value::Object(object)) => object
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().to_owned()),
        _ => None,
    }
}

fn activitypub_object_type(value: Option<&serde_json::Value>) -> Option<&str> {
    value
        .and_then(serde_json::Value::as_object)
        .and_then(|object| object.get("type"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn activitypub_object_actor_id(value: Option<&serde_json::Value>) -> Option<String> {
    match value {
        Some(serde_json::Value::String(value)) if !value.trim().is_empty() => {
            Some(value.trim().to_owned())
        }
        Some(serde_json::Value::Object(object)) => object
            .get("actor")
            .or_else(|| object.get("object"))
            .or_else(|| object.get("id"))
            .and_then(|value| activitypub_value_string(Some(value))),
        _ => None,
    }
}

fn activitypub_relationship_key(actor: &str, collection: &str, remote_actor_id: &str) -> String {
    let remote_digest = hex::encode(Sha256::digest(remote_actor_id.as_bytes()));
    format!("activitypub/{actor}/relationships/{collection}/{remote_digest}")
}

fn activitypub_relationship_items(
    features: &ControllerFeatureState,
    actor: &str,
    collection: &str,
    limit: usize,
) -> Vec<String> {
    let prefix = format!("activitypub/{actor}/relationships/{collection}/");
    let mut records = features
        .entries_with_prefix(&prefix)
        .into_iter()
        .filter_map(|(key, value)| {
            let remote_actor_id = value
                .get("remoteActorId")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())?
                .trim()
                .to_owned();
            let updated_at = value
                .get("updatedAt")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default();
            Some((key, remote_actor_id, updated_at))
        })
        .collect::<Vec<_>>();
    records.sort_by(|left, right| right.2.cmp(&left.2).then_with(|| right.0.cmp(&left.0)));
    records
        .into_iter()
        .take(limit)
        .map(|(_, remote_actor_id, _)| remote_actor_id)
        .collect()
}

pub(super) async fn activitypub_apply_relationship(
    actor: &str,
    direction: &str,
    activity_type: &str,
    activity: &serde_json::Value,
    state: &AppState,
) -> Result<(), String> {
    let is_inbox = direction == "inbox";
    let is_outbox = direction == "outbox";
    if !is_inbox && !is_outbox {
        return Ok(());
    }

    let activity_type = activity_type.trim();
    let object = activity.get("object");
    let object_is_follow =
        activitypub_object_type(object).is_some_and(|value| value.eq_ignore_ascii_case("Follow"));
    let mut upserts = Vec::new();
    let mut removals = Vec::new();

    if is_outbox && activity_type.eq_ignore_ascii_case("Follow") {
        if let Some(remote_actor_id) = activitypub_object_actor_id(object) {
            upserts.push(("following", remote_actor_id));
        }
    } else if is_inbox && activity_type.eq_ignore_ascii_case("Follow") {
        if let Some(remote_actor_id) = activitypub_value_string(activity.get("actor")) {
            upserts.push(("followers", remote_actor_id));
        }
    } else if activity_type.eq_ignore_ascii_case("Undo")
        || activity_type.eq_ignore_ascii_case("Remove")
    {
        if object_is_follow {
            let remote_actor_id = activitypub_object_actor_id(object)
                .or_else(|| activitypub_value_string(activity.get("actor")));
            if let Some(remote_actor_id) = remote_actor_id {
                if is_inbox {
                    removals.push(("followers", remote_actor_id.clone()));
                    if activity_type.eq_ignore_ascii_case("Remove") {
                        removals.push(("following", remote_actor_id));
                    }
                } else {
                    removals.push(("following", remote_actor_id));
                }
            }
        }
    } else if is_inbox
        && (activity_type.eq_ignore_ascii_case("Accept")
            || activity_type.eq_ignore_ascii_case("Reject"))
        && object_is_follow
    {
        if let Some(remote_actor_id) = activitypub_value_string(activity.get("actor")) {
            if activity_type.eq_ignore_ascii_case("Accept") {
                upserts.push(("following", remote_actor_id));
            } else {
                removals.push(("following", remote_actor_id));
            }
        }
    }

    if upserts.is_empty() && removals.is_empty() {
        return Ok(());
    }

    let actor = actor.to_owned();
    state
        .controller_features
        .mutate(move |features| {
            for (collection, remote_actor_id) in upserts {
                let key = activitypub_relationship_key(&actor, collection, &remote_actor_id);
                features.upsert(
                    key,
                    serde_json::json!({
                        "remoteActorId": remote_actor_id,
                        "updatedAt": unix_timestamp_millis(),
                    }),
                )?;
            }
            for (collection, remote_actor_id) in removals {
                let key = activitypub_relationship_key(&actor, collection, &remote_actor_id);
                features.remove(&key)?;
            }
            Ok(())
        })
        .await
}

async fn activitypub_music_activities(
    actor_id: &str,
    base_url: &str,
    state: &AppState,
    limit: usize,
) -> Vec<serde_json::Value> {
    let entries = state.shares.read().await.entries.clone();
    entries
        .into_iter()
        .take(limit)
        .map(|entry| {
            let work_key = format!("{}|{}", entry.filename, entry.size);
            let work_id = hex::encode(Sha256::digest(work_key.as_bytes()));
            let work_id = format!("{base_url}/works/music/{work_id}");
            let title = virtual_basename(&entry.filename);
            let work = serde_json::json!({
                "@context": [
                    "https://www.w3.org/ns/activitystreams",
                    "https://w3id.org/federation/workref#"
                ],
                "id": work_id,
                "type": "WorkRef",
                "domain": "music",
                "title": title,
                "attributedTo": actor_id,
            });
            serde_json::json!({
                "id": format!("{actor_id}/activities/{}", hex::encode(Sha256::digest(work_key.as_bytes()))),
                "type": "Create",
                "actor": actor_id,
                "object": work,
                "to": ["https://www.w3.org/ns/activitystreams#Public"],
                "published": chrono::Utc::now().to_rfc3339(),
            })
        })
        .collect()
}

pub(super) async fn activitypub_get_response(
    path: &str,
    query: Option<&str>,
    state: &AppState,
) -> HttpResponse {
    let Some(segments) = decoded_segments_after(path, "/actors/") else {
        return routing::not_found_response();
    };
    let [actor, tail @ ..] = segments.as_slice() else {
        return routing::not_found_response();
    };
    if !activitypub_actor_exists(actor, state).await {
        return routing::not_found_response();
    }

    let origin = activitypub_base_url(state);
    let actor_id = format!("{origin}/actors/{actor}");
    let page_size = usize::try_from(state.config.social_federation.page_size).unwrap_or(20);
    match tail {
        [] => {
            let native_music_actor = actor == "music";
            let mut document = serde_json::json!({
                "@context": [
                    "https://www.w3.org/ns/activitystreams",
                    "https://w3id.org/security/v1",
                    "https://w3id.org/federation/workref#"
                ],
                "id": actor_id.clone(),
                "type": if native_music_actor { "Service" } else { "Person" },
                "preferredUsername": actor,
                "inbox": format!("{actor_id}/inbox"),
                "outbox": format!("{actor_id}/outbox"),
                "followers": format!("{actor_id}/followers"),
                "following": format!("{actor_id}/following"),
            });
            if native_music_actor {
                document["name"] = serde_json::json!("Music Library");
                document["summary"] = serde_json::json!(
                    "A decentralized collection of music shared by community members"
                );
                document["publicKey"] = serde_json::json!({
                    "id": format!("{actor_id}#main-key"),
                    "owner": actor_id,
                    "publicKeyPem": activitypub_public_key_pem(state),
                });
            }
            activitypub_response(document)
        }
        [collection] if matches!(collection.as_str(), "inbox" | "outbox") => {
            if collection == "outbox"
                && query_parameter(query, "page").is_some_and(|value| value.parse::<i32>().is_err())
            {
                return routing::bad_request_response("The page value must be a valid integer");
            }
            let prefix = format!("activitypub/{actor}/{collection}/");
            let records = state
                .controller_features
                .read()
                .await
                .values_with_prefix(&prefix);
            let mut items = records
                .into_iter()
                .filter_map(|record| record.get("activity").cloned())
                .take(page_size)
                .collect::<Vec<_>>();
            if collection == "outbox" && actor == "music" && items.len() < page_size {
                let recent = activitypub_music_activities(
                    &actor_id,
                    &origin,
                    state,
                    page_size.saturating_sub(items.len()),
                )
                .await;
                items.extend(recent);
            }
            activitypub_response(serde_json::json!({
                "@context": "https://www.w3.org/ns/activitystreams",
                "id": format!("{actor_id}/{collection}"),
                "type": "OrderedCollection",
                "totalItems": items.len(),
                "orderedItems": items,
            }))
        }
        [collection] if matches!(collection.as_str(), "followers" | "following") => {
            let items = activitypub_relationship_items(
                &*state.controller_features.read().await,
                actor,
                collection,
                page_size,
            );
            activitypub_response(serde_json::json!({
                "@context": "https://www.w3.org/ns/activitystreams",
                "id": format!("{actor_id}/{collection}"),
                "type": "OrderedCollection",
                "totalItems": items.len(),
                "orderedItems": items,
            }))
        }
        _ => routing::not_found_response(),
    }
}

pub(super) async fn activitypub_webfinger_response(
    query: Option<&str>,
    state: &AppState,
) -> HttpResponse {
    let Some(resource) = query_parameter(query, "resource") else {
        return routing::bad_request_response("resource is required");
    };
    let resource = resource.trim().to_owned();
    let parsed = if resource.len() >= 5 && resource[..5].eq_ignore_ascii_case("acct:") {
        resource[5..]
            .rsplit_once('@')
            .map(|(actor, domain)| (actor.trim().to_owned(), domain.trim().to_owned()))
    } else if resource.len() >= 8 && resource[..8].eq_ignore_ascii_case("https://") {
        reqwest::Url::parse(&resource).ok().and_then(|url| {
            let domain = url.host_str()?.to_owned();
            let segments = url
                .path_segments()?
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>();
            let actor = match segments.as_slice() {
                [segment] if segment.starts_with('@') => segment.trim_start_matches('@'),
                [prefix, actor] if prefix.eq_ignore_ascii_case("actors") => actor,
                _ => return None,
            };
            (!actor.trim().is_empty()).then(|| (actor.trim().to_owned(), domain))
        })
    } else {
        None
    };
    let Some((actor, domain)) =
        parsed.filter(|(actor, domain)| !actor.is_empty() && !domain.is_empty())
    else {
        return routing::not_found_response();
    };
    if !domain.eq_ignore_ascii_case(&activitypub_domain(state))
        || !activitypub_actor_exists(&actor, state).await
    {
        return routing::not_found_response();
    }
    let actor_url = format!("{}/actors/{actor}", activitypub_base_url(state));
    let rel = query_parameter(query, "rel").filter(|value| !value.trim().is_empty());
    let mut links = vec![
        serde_json::json!({
            "rel": "self",
            "type": "application/activity+json",
            "href": actor_url,
        }),
        serde_json::json!({
            "rel": "http://webfinger.net/rel/profile-page",
            "type": "text/html",
            "href": format!("{}/@{actor}", activitypub_base_url(state)),
        }),
    ];
    if let Some(rel) = rel {
        links.retain(|link| link["rel"].as_str() == Some(rel.trim()));
    }
    webfinger_response(serde_json::json!({
        "subject": resource,
        "links": links,
    }))
}
