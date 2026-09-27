use super::*;

pub(super) async fn quarantine_mutation_response(
    path: &str,
    body: &str,
    state: &AppState,
) -> HttpResponse {
    if path == "/api/quarantine-jury/requests" {
        let mut request = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(request @ serde_json::Value::Object(_)) => request,
            Ok(_) => return routing::bad_request_response("request must be an object"),
            Err(_) => return routing::bad_request_response("invalid JSON body"),
        };
        if request
            .get("jurors")
            .is_some_and(|jurors| json_array_exceeds_limit(jurors, MAX_QUARANTINE_JURY_ITEMS))
            || request.get("evidence").is_some_and(|evidence| {
                json_array_exceeds_limit(evidence, MAX_QUARANTINE_JURY_ITEMS)
            })
        {
            return routing::bad_request_response(
                "quarantine jury arrays must contain at most 100 items",
            );
        }
        if request.as_object().is_none_or(serde_json::Map::is_empty) {
            return routing::bad_request_response("request fields are required");
        }
        let mut errors = Vec::new();
        let local_reason = request
            .get("localReason")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        if local_reason.is_empty() {
            errors.push("Local quarantine reason is required.");
        }
        let jurors = request
            .get("jurors")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let evidence_items = request
            .get("evidence")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        if jurors.is_empty() {
            errors.push("At least one trusted juror is required.");
        }
        if evidence_items.is_empty() {
            errors.push("At least one minimal evidence item is required.");
        }
        // Matches the oracle's real ValidateRequest: every evidence item
        // and juror identifier is checked individually, not just counted.
        for evidence in &evidence_items {
            quarantine_add_evidence_errors(&mut errors, evidence);
        }
        for juror in jurors.iter().filter_map(serde_json::Value::as_str) {
            if !is_safe_opaque_reference(juror) {
                errors.push("Juror identifiers must be opaque and safe.");
            }
        }
        if !errors.is_empty() {
            return HttpResponse {
                status: "400 Bad Request",
                content_type: "application/json",
                body: serde_json::json!({"isValid": false, "errors": errors}).to_string(),
            };
        }
        let id = request
            .get("requestId")
            .or_else(|| request.get("id"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        request["requestId"] = serde_json::json!(id);
        request["localReason"] = serde_json::json!(local_reason);
        request["createdAt"] = serde_json::json!(unix_timestamp());
        request["status"] = serde_json::json!("Pending");
        return match state
            .controller_features
            .upsert(format!("quarantine/request/{id}"), request.clone())
            .await
        {
            Ok(()) => routing::ok_response(
                serde_json::json!({"isValid": true, "request": request, "errors": []}).to_string(),
            ),
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    if path == "/api/quarantine-jury/verdicts" {
        let mut verdict = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(verdict @ serde_json::Value::Object(_)) => verdict,
            Ok(_) => return routing::bad_request_response("verdict must be an object"),
            Err(_) => return routing::bad_request_response("invalid JSON body"),
        };
        if verdict
            .get("evidence")
            .is_some_and(|evidence| json_array_exceeds_limit(evidence, MAX_QUARANTINE_JURY_ITEMS))
        {
            return routing::bad_request_response(
                "quarantine jury arrays must contain at most 100 items",
            );
        }
        let request_id = verdict
            .get("requestId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let Some(request) = state
            .controller_features
            .read()
            .await
            .get(&format!("quarantine/request/{request_id}"))
            .cloned()
        else {
            return HttpResponse {
                status: "400 Bad Request",
                content_type: "application/json",
                body: r#"{"isValid":false,"errors":["Request not found."]}"#.to_owned(),
            };
        };
        let mut errors = Vec::new();
        let juror = verdict
            .get("juror")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let jurors_listed = request
            .get("jurors")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|jurors| {
                jurors
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .any(|listed| listed.eq_ignore_ascii_case(&juror))
            });
        if !jurors_listed {
            errors.push("Juror is not selected for this request.");
        }
        if !matches!(
            verdict.get("verdict").and_then(serde_json::Value::as_str),
            Some("NeedsManualReview" | "UpholdQuarantine" | "ReleaseCandidate")
        ) {
            errors
                .push("Verdict must be NeedsManualReview, UpholdQuarantine, or ReleaseCandidate.");
        }
        // Matches the oracle's real ValidateVerdict: a verdict must carry a
        // signature whose payloadHash actually matches the verdict's own
        // contents -- otherwise nothing stops a caller from replaying or
        // hand-editing a verdict while claiming it was signed as-is. This
        // is a content-integrity check (the oracle never resolves a real
        // public key for `Signer` here either), not full cryptographic
        // authentication, but it's exactly what the oracle enforces.
        verdict["requestId"] = serde_json::json!(request_id);
        verdict["juror"] = serde_json::json!(juror);
        let signature = verdict.get("signature").cloned().unwrap_or_default();
        let signer = signature
            .get("signer")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let payload_hash = signature
            .get("payloadHash")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let signature_value = signature
            .get("value")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if signer.trim().is_empty()
            || payload_hash.trim().is_empty()
            || signature_value.trim().is_empty()
        {
            errors.push("Signed juror verdict is required.");
        } else if !payload_hash.eq_ignore_ascii_case(&quarantine_verdict_payload_hash(&verdict)) {
            errors.push("Signature payload hash does not match verdict contents.");
        }
        if let Some(evidence_items) = verdict
            .get("evidence")
            .and_then(serde_json::Value::as_array)
        {
            for evidence in evidence_items {
                quarantine_add_evidence_errors(&mut errors, evidence);
            }
        }
        if !errors.is_empty() {
            return HttpResponse {
                status: "400 Bad Request",
                content_type: "application/json",
                body: serde_json::json!({"isValid": false, "errors": errors}).to_string(),
            };
        }
        // One verdict per juror per request: a resubmission replaces the
        // prior verdict rather than double-counting toward quorum.
        let id = format!(
            "quarantine-jury-verdict:{}:{}",
            request_id.trim().to_ascii_lowercase(),
            juror.trim().to_ascii_lowercase()
        );
        verdict["verdictId"] = serde_json::json!(id);
        verdict["submittedAt"] = serde_json::json!(unix_timestamp());
        return match state
            .controller_features
            .upsert(
                format!("quarantine/verdict/{request_id}/{id}"),
                verdict.clone(),
            )
            .await
        {
            Ok(()) => routing::ok_response(
                serde_json::json!({"isValid": true, "verdict": verdict, "errors": []}).to_string(),
            ),
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    if path.ends_with("/accept-release-candidate") {
        return quarantine_accept_release_candidate_response(path, body, state).await;
    }
    if path.ends_with("/routes") {
        return quarantine_route_request_response(path, body, state).await;
    }
    routing::not_found_response()
}

fn quarantine_verdict_rank(verdict: &str) -> u8 {
    match verdict {
        "UpholdQuarantine" => 1,
        "ReleaseCandidate" => 2,
        _ => 0,
    }
}

/// Builds the real quorum-based aggregate for a quarantine-jury request,
/// matching the oracle's `BuildAggregate`: groups stored verdicts by type
/// and requires a 2/3 supermajority once `minJurorVotes` real verdicts have
/// been recorded (ties broken toward the lowest-ranked verdict, i.e.
/// `NeedsManualReview`). Returns `None` only if the request itself does not
/// exist.
async fn quarantine_build_aggregate(
    state: &AppState,
    request_id: &str,
) -> Option<serde_json::Value> {
    let features = state.controller_features.read().await;
    let request = features
        .get(&format!("quarantine/request/{request_id}"))
        .cloned()?;
    let required_votes = request
        .get("minJurorVotes")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(2)
        .max(1) as usize;
    let verdicts = features.values_with_prefix(&format!("quarantine/verdict/{request_id}/"));
    drop(features);

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for verdict in &verdicts {
        let value = verdict
            .get("verdict")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("NeedsManualReview")
            .to_owned();
        *counts.entry(value).or_insert(0) += 1;
    }
    let total_verdicts = verdicts.len();
    let quorum_reached = total_verdicts >= required_votes;
    let recommendation = if !quorum_reached {
        "NeedsManualReview".to_owned()
    } else {
        let required_agreement = total_verdicts.saturating_mul(2).div_ceil(3);
        let mut ranked = counts.iter().collect::<Vec<_>>();
        ranked.sort_by(|(a_verdict, a_count), (b_verdict, b_count)| {
            b_count.cmp(a_count).then_with(|| {
                quarantine_verdict_rank(a_verdict).cmp(&quarantine_verdict_rank(b_verdict))
            })
        });
        ranked
            .first()
            .filter(|(_, count)| **count >= required_agreement)
            .map(|(verdict, _)| (*verdict).clone())
            .unwrap_or_else(|| "NeedsManualReview".to_owned())
    };
    let dissenting_jurors = verdicts
        .iter()
        .filter(|verdict| {
            verdict.get("verdict").and_then(serde_json::Value::as_str)
                != Some(recommendation.as_str())
        })
        .filter_map(|verdict| {
            verdict
                .get("juror")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    let reason = if !quorum_reached {
        format!("Waiting for trusted juror quorum: {total_verdicts}/{required_votes}.")
    } else if recommendation == "NeedsManualReview" {
        "Trusted jurors did not reach a supermajority.".to_owned()
    } else {
        "Trusted jurors reached a supermajority recommendation.".to_owned()
    };

    Some(serde_json::json!({
        "requestId": request_id,
        "recommendation": recommendation,
        "totalVerdicts": total_verdicts,
        "requiredVotes": required_votes,
        "verdictCounts": counts,
        "dissentingJurors": dissenting_jurors,
        "quorumReached": quorum_reached,
        "reason": reason,
    }))
}

fn quarantine_can_accept(aggregate: &serde_json::Value, already_accepted: bool) -> bool {
    !already_accepted
        && aggregate["quorumReached"] == serde_json::json!(true)
        && aggregate["recommendation"] == "ReleaseCandidate"
}

/// Matches the oracle's `QuarantineJuryService.AddEvidenceErrors`: an
/// evidence reference must be present and pass the same opaque/safe
/// reference check used for pod content references, and a summary is
/// capped at 512 characters. Deliberately pushes both errors when the
/// reference is empty (an empty reference also fails the safety check),
/// matching the oracle's literal (if slightly redundant) behavior.
fn quarantine_add_evidence_errors(errors: &mut Vec<&'static str>, evidence: &serde_json::Value) {
    let reference = evidence
        .get("reference")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let summary = evidence
        .get("summary")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    if reference.is_empty() {
        errors.push("Evidence reference is required.");
    }
    if !is_safe_opaque_reference(reference) {
        errors.push(
            "Evidence references must not include paths, raw hashes, endpoints, or private identifiers.",
        );
    }
    if summary.len() > 512 {
        errors.push("Evidence summary must be 512 characters or fewer.");
    }
}

/// Matches the oracle's `QuarantineJuryVerdictRecord.ComputePayloadHash`:
/// a canonical hash over the verdict's own contents (request id, juror,
/// verdict, reason, and evidence, evidence sorted by type then
/// reference), used to verify a submitted signature's `payloadHash`
/// actually covers what was submitted rather than trusting the claim.
pub(super) fn quarantine_verdict_payload_hash(verdict: &serde_json::Value) -> String {
    let request_id = verdict
        .get("requestId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let juror = verdict
        .get("juror")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let verdict_value = verdict
        .get("verdict")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let reason = verdict
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let mut evidence_parts = verdict
        .get("evidence")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|item| {
            let evidence_type = item
                .get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let reference = item
                .get("reference")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_owned();
            let summary = item
                .get("summary")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_owned();
            (evidence_type, reference, summary)
        })
        .collect::<Vec<_>>();
    evidence_parts.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let evidence_joined = evidence_parts
        .iter()
        .map(|(evidence_type, reference, summary)| format!("{evidence_type}:{reference}:{summary}"))
        .collect::<Vec<_>>()
        .join("|");
    let payload = [request_id, juror, verdict_value, reason, &evidence_joined].join("\n");
    hex::encode(Sha256::digest(payload.as_bytes()))
}

fn quarantine_acceptance_reason(aggregate: &serde_json::Value, already_accepted: bool) -> String {
    if already_accepted {
        return "Release-candidate recommendation has already been accepted.".to_owned();
    }
    if aggregate["quorumReached"] != serde_json::json!(true) {
        return "Waiting for trusted juror quorum.".to_owned();
    }
    if aggregate["recommendation"] == "ReleaseCandidate" {
        "Release-candidate recommendation can be accepted manually.".to_owned()
    } else {
        "Only a release-candidate supermajority can be accepted.".to_owned()
    }
}

/// Matches the oracle's `QuarantineJuryService.BuildAuditStatus`.
fn quarantine_audit_status(aggregate: &serde_json::Value, has_acceptance: bool) -> &'static str {
    if has_acceptance {
        return "accepted-release-candidate";
    }
    if aggregate["recommendation"] == "ReleaseCandidate" && aggregate["quorumReached"] == true {
        return "pending-release-acceptance";
    }
    if aggregate["recommendation"] == "UpholdQuarantine" && aggregate["quorumReached"] == true {
        return "uphold-quarantine";
    }
    "manual-review"
}

/// Matches the oracle's `QuarantineJuryService.BuildAuditEntry`: real
/// per-request quorum/status/staleness derived from the same stored
/// verdicts, route attempts, and acceptance decisions the review/aggregate
/// endpoints already use -- not hardcoded zeros standing in for every
/// request regardless of its real state.
pub(super) async fn quarantine_build_audit_entry(
    state: &AppState,
    request: &serde_json::Value,
    generated_at: u64,
    stale_after_seconds: u64,
) -> serde_json::Value {
    let request_id = request
        .get("requestId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let aggregate = quarantine_build_aggregate(state, &request_id)
        .await
        .unwrap_or_else(|| {
            serde_json::json!({
                "requestId": request_id,
                "recommendation": "NeedsManualReview",
                "totalVerdicts": 0,
                "requiredVotes": 1,
                "verdictCounts": {},
                "dissentingJurors": [],
                "quorumReached": false,
                "reason": "Request not found.",
            })
        });
    let features = state.controller_features.read().await;
    let verdict_count = features
        .values_with_prefix(&format!("quarantine/verdict/{request_id}/"))
        .len();
    let route_attempts = features.values_with_prefix(&format!("quarantine/routes/{request_id}/"));
    let acceptance = features
        .get(&format!("quarantine/acceptance/{request_id}"))
        .cloned();
    drop(features);
    let has_acceptance = acceptance.is_some();
    let can_accept = quarantine_can_accept(&aggregate, has_acceptance);
    let status = quarantine_audit_status(&aggregate, has_acceptance);
    let created_at = request
        .get("createdAt")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let is_stale = !has_acceptance
        && generated_at.saturating_sub(created_at) >= stale_after_seconds
        && matches!(status, "manual-review" | "pending-release-acceptance");
    let has_failed_route_attempts = route_attempts.iter().any(|attempt| {
        attempt.get("success") != Some(&serde_json::json!(true))
            || attempt
                .get("failedJurors")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|jurors| !jurors.is_empty())
    });
    let evidence_count = request
        .get("evidence")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let juror_count = request
        .get("jurors")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    serde_json::json!({
        "requestId": request_id,
        "localReason": request.get("localReason").cloned().unwrap_or(serde_json::Value::Null),
        "createdAt": created_at,
        "evidenceCount": evidence_count,
        "jurorCount": juror_count,
        "verdictCount": verdict_count,
        "requiredVotes": aggregate["requiredVotes"],
        "recommendation": aggregate["recommendation"],
        "quorumReached": aggregate["quorumReached"],
        "hasAcceptance": has_acceptance,
        "canAcceptReleaseCandidate": can_accept,
        "hasRouteAttempts": !route_attempts.is_empty(),
        "hasFailedRouteAttempts": has_failed_route_attempts,
        "isStale": is_stale,
        "status": status,
        "reason": if has_acceptance {
            "Release-candidate recommendation accepted locally.".to_owned()
        } else {
            aggregate["reason"].as_str().unwrap_or_default().to_owned()
        },
        "dissentingJurors": aggregate["dissentingJurors"],
    })
}

async fn quarantine_accept_release_candidate_response(
    path: &str,
    body: &str,
    state: &AppState,
) -> HttpResponse {
    let Some(request_id) = path_segment_between(
        path,
        "/api/quarantine-jury/requests/",
        "/accept-release-candidate",
    ) else {
        return routing::not_found_response();
    };
    let request_id = decoded_path_segment(request_id);
    let Some(aggregate) = quarantine_build_aggregate(state, &request_id).await else {
        return HttpResponse {
            status: "404 Not Found",
            content_type: "application/json",
            body: serde_json::json!({
                "isAccepted": false,
                "errors": ["Request not found."],
                "decision": null,
            })
            .to_string(),
        };
    };
    let acceptance_key = format!("quarantine/acceptance/{request_id}");
    if let Some(existing) = state
        .controller_features
        .read()
        .await
        .get(&acceptance_key)
        .cloned()
    {
        return routing::ok_response(
            serde_json::json!({"isAccepted": true, "errors": [], "decision": existing}).to_string(),
        );
    }
    if !quarantine_can_accept(&aggregate, false) {
        let reason = quarantine_acceptance_reason(&aggregate, false);
        return HttpResponse {
            status: "400 Bad Request",
            content_type: "application/json",
            body: serde_json::json!({"isAccepted": false, "errors": [reason], "decision": null})
                .to_string(),
        };
    }
    let payload =
        serde_json::from_str::<serde_json::Value>(body).unwrap_or_else(|_| serde_json::json!({}));
    let accepted_by = payload
        .get("acceptedBy")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("local-user")
        .to_owned();
    let note = payload
        .get("note")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let decision = serde_json::json!({
        "id": format!("quarantine-jury-acceptance:{}", uuid::Uuid::new_v4().simple()),
        "requestId": request_id,
        "acceptedBy": accepted_by,
        "acceptedRecommendation": "ReleaseCandidate",
        "note": note,
        "aggregateSnapshot": aggregate,
        "createdAt": unix_timestamp(),
    });
    match state
        .controller_features
        .upsert(acceptance_key, decision.clone())
        .await
    {
        Ok(()) => routing::ok_response(
            serde_json::json!({"isAccepted": true, "errors": [], "decision": decision}).to_string(),
        ),
        Err(error) => routing::service_unavailable_response(&error),
    }
}

fn quarantine_safe_opaque_reference(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || value.len() > 200 {
        return false;
    }
    let lowered = value.to_ascii_lowercase();
    if value.contains(['/', '\\'])
        || [
            "localhost",
            "127.0.0.1",
            "192.168.",
            "10.",
            "172.16.",
            "path",
            "file",
            "private",
            "internal",
        ]
        .iter()
        .any(|fragment| lowered.contains(fragment))
    {
        return false;
    }
    !(value.len() >= 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

async fn quarantine_route_request_response(
    path: &str,
    body: &str,
    state: &AppState,
) -> HttpResponse {
    let Some(request_id) = path_segment_between(path, "/api/quarantine-jury/requests/", "/routes")
    else {
        return routing::not_found_response();
    };
    let request_id = decoded_path_segment(request_id);
    let Some(request) = state
        .controller_features
        .read()
        .await
        .get(&format!("quarantine/request/{request_id}"))
        .cloned()
    else {
        // The oracle stores the failed attempt before returning NotFound.
        // Keep the attempt shape/readback identical to the normal route
        // branches so a missing request is observable rather than discarded.
        let payload = serde_json::from_str::<serde_json::Value>(body)
            .unwrap_or_else(|_| serde_json::json!({}));
        let pod_id = payload
            .get("podId")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("quarantine-jury")
            .to_owned();
        let channel_id = payload
            .get("channelId")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("request:{request_id}"));
        let attempt = serde_json::json!({
            "id": format!("quarantine-jury-route:{}", uuid::Uuid::new_v4().simple()),
            "requestId": request_id,
            "messageId": "",
            "podId": pod_id,
            "channelId": channel_id,
            "targetJurors": [],
            "routedJurors": [],
            "failedJurors": [],
            "success": false,
            "errorMessage": "Request not found.",
            "createdAt": unix_timestamp(),
        });
        let attempt_id = attempt["id"].as_str().unwrap_or_default();
        if let Err(error) = state
            .controller_features
            .upsert(
                format!("quarantine/routes/{request_id}/{attempt_id}"),
                attempt.clone(),
            )
            .await
        {
            return routing::service_unavailable_response(&error);
        }
        return HttpResponse {
            status: "404 Not Found",
            content_type: "application/json",
            body: attempt.to_string(),
        };
    };
    let payload =
        serde_json::from_str::<serde_json::Value>(body).unwrap_or_else(|_| serde_json::json!({}));
    if payload
        .get("targetJurors")
        .is_some_and(|jurors| json_array_exceeds_limit(jurors, MAX_QUARANTINE_JURY_ITEMS))
    {
        return routing::bad_request_response(
            "quarantine jury arrays must contain at most 100 items",
        );
    }
    let unsafe_metadata = ["senderPeerId", "podId", "channelId"]
        .into_iter()
        .filter_map(|field| payload.get(field).and_then(serde_json::Value::as_str))
        .any(|value| !quarantine_safe_opaque_reference(value));
    let juror_names = request
        .get("jurors")
        .and_then(serde_json::Value::as_array)
        .map(|jurors| {
            jurors
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut target_jurors = payload
        .get("targetJurors")
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| juror_names.clone());
    target_jurors.sort_by_key(|juror| juror.to_ascii_lowercase());
    target_jurors.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    let invalid_target = target_jurors.iter().any(|juror| {
        !juror_names
            .iter()
            .any(|listed| listed.eq_ignore_ascii_case(juror))
            || !quarantine_safe_opaque_reference(juror)
    });

    let id = format!("quarantine-jury-route:{}", uuid::Uuid::new_v4().simple());
    let (success, error_message, routed_jurors, failed_jurors, message_id, pod_id, channel_id) =
        if unsafe_metadata {
            (
                false,
                Some("Route metadata must be opaque and safe.".to_owned()),
                Vec::<String>::new(),
                Vec::<String>::new(),
                String::new(),
                String::new(),
                String::new(),
            )
        } else if invalid_target {
            (
                false,
                Some("Route targets must be selected safe jurors.".to_owned()),
                Vec::<String>::new(),
                target_jurors.clone(),
                String::new(),
                String::new(),
                String::new(),
            )
        } else {
            let pod_id = payload
                .get("podId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("quarantine-jury")
                .to_owned();
            let channel_id = payload
                .get("channelId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("request:{request_id}"));
            let message_id = format!("quarantine-jury-request:{}", uuid::Uuid::new_v4().simple());
            let body = serde_json::json!({
            "type": "slskdn.quarantine-jury.request.v1",
            "requestId": request_id,
            "localReason": request.get("localReason").cloned().unwrap_or(serde_json::Value::Null),
            "evidence": request.get("evidence").cloned().unwrap_or_else(|| serde_json::json!([])),
            "requiredVotes": request.get("minJurorVotes").cloned().unwrap_or(serde_json::json!(0)),
            "targetJurors": target_jurors,
            "createdAt": chrono::Utc::now().to_rfc3339(),
        })
        .to_string();
            let message = serde_json::json!({
                "messageId": message_id,
                "podId": pod_id,
                "channelId": channel_id,
                "senderPeerId": payload
                    .get("senderPeerId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("local-quarantine-jury"),
                "body": body,
                "signature": "local-quarantine-jury-route",
                "timestampUnixMs": unix_timestamp_millis(),
                "sigVersion": 1,
            });
            let has_configured_backend = target_jurors
                .iter()
                .any(|juror| trusted_mesh_peer_for(state, juror).is_some());
            if !has_configured_backend {
                (
                    false,
                    Some("Routing backend is not available.".to_owned()),
                    Vec::<String>::new(),
                    target_jurors.clone(),
                    message_id,
                    pod_id,
                    channel_id,
                )
            } else {
                let mut routed_jurors = Vec::new();
                let mut failed_jurors = Vec::new();
                for juror in &target_jurors {
                    match route_pod_message_to_peer(state, &message, juror).await {
                        Ok(_) => routed_jurors.push(juror.clone()),
                        Err(_) => failed_jurors.push(juror.clone()),
                    }
                }
                let success = failed_jurors.is_empty();
                (
                    success,
                    (!success).then(|| "One or more jurors could not be reached.".to_owned()),
                    routed_jurors,
                    failed_jurors,
                    message_id,
                    pod_id,
                    channel_id,
                )
            }
        };
    let attempt = serde_json::json!({
        "id": id,
        "requestId": request_id,
        "messageId": message_id,
        "podId": pod_id,
        "channelId": channel_id,
        "targetJurors": target_jurors,
        "routedJurors": routed_jurors,
        "failedJurors": failed_jurors,
        "success": success,
        "errorMessage": error_message,
        "createdAt": unix_timestamp(),
    });
    match state
        .controller_features
        .upsert(
            format!("quarantine/routes/{request_id}/{id}"),
            attempt.clone(),
        )
        .await
    {
        Ok(()) if success => routing::ok_response(attempt.to_string()),
        Ok(()) => HttpResponse {
            status: "400 Bad Request",
            content_type: "application/json",
            body: attempt.to_string(),
        },
        Err(error) => routing::service_unavailable_response(&error),
    }
}

pub(super) async fn quarantine_dynamic_get_response(path: &str, state: &AppState) -> HttpResponse {
    let Some(segments) = decoded_segments_after(path, "/api/quarantine-jury/requests/") else {
        return routing::not_found_response();
    };
    let request_id = &segments[0];
    let features = state.controller_features.read().await;
    if matches!(segments.as_slice(), [_, action] if action == "routes") {
        return routing::ok_response(
            serde_json::Value::Array(
                features.values_with_prefix(&format!("quarantine/routes/{request_id}/")),
            )
            .to_string(),
        );
    }
    let Some(request) = features
        .get(&format!("quarantine/request/{request_id}"))
        .cloned()
    else {
        return routing::not_found_response();
    };
    drop(features);
    match segments.as_slice() {
        [_] => routing::ok_response(request.to_string()),
        [_, action] if action == "aggregate" => {
            let aggregate = quarantine_build_aggregate(state, request_id)
                .await
                .expect("request presence already confirmed above");
            routing::ok_response(aggregate.to_string())
        }
        [_, action] if action == "review" => {
            let aggregate = quarantine_build_aggregate(state, request_id)
                .await
                .expect("request presence already confirmed above");
            let features = state.controller_features.read().await;
            let verdicts =
                features.values_with_prefix(&format!("quarantine/verdict/{request_id}/"));
            let route_attempts =
                features.values_with_prefix(&format!("quarantine/routes/{request_id}/"));
            let acceptance = features
                .get(&format!("quarantine/acceptance/{request_id}"))
                .cloned();
            drop(features);
            let already_accepted = acceptance.is_some();
            routing::ok_response(
                serde_json::json!({
                    "request": request,
                    "aggregate": aggregate.clone(),
                    "verdicts": verdicts,
                    "routeAttempts": route_attempts,
                    "acceptance": acceptance,
                    "canAcceptReleaseCandidate": quarantine_can_accept(&aggregate, already_accepted),
                    "acceptanceReason": quarantine_acceptance_reason(&aggregate, already_accepted),
                })
                .to_string(),
            )
        }
        [_, action] if action == "release-package" => {
            let Some(acceptance) = state
                .controller_features
                .read()
                .await
                .get(&format!("quarantine/acceptance/{request_id}"))
                .cloned()
            else {
                return HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!({
                        "isReady": false,
                        "errors": ["Release candidate has not been accepted locally."],
                        "package": null,
                    })
                    .to_string(),
                };
            };
            let aggregate = quarantine_build_aggregate(state, request_id)
                .await
                .expect("request presence already confirmed above");
            let features = state.controller_features.read().await;
            let verdicts =
                features.values_with_prefix(&format!("quarantine/verdict/{request_id}/"));
            let route_attempts =
                features.values_with_prefix(&format!("quarantine/routes/{request_id}/"));
            drop(features);
            let mut warnings = vec![
                "Release package is evidence-only and does not change local quarantine enforcement."
                    .to_owned(),
            ];
            let snapshot = acceptance
                .get("aggregateSnapshot")
                .cloned()
                .unwrap_or_default();
            if snapshot["recommendation"] != aggregate["recommendation"]
                || snapshot["totalVerdicts"] != aggregate["totalVerdicts"]
            {
                warnings.push(
                    "Current verdict aggregate differs from the aggregate accepted by the operator."
                        .to_owned(),
                );
            }
            let package = serde_json::json!({
                "type": "slskdn.quarantine-jury.release-package.v1",
                "version": "1.0",
                "generatedAt": chrono::Utc::now().to_rfc3339(),
                "requestId": request_id,
                "localReason": request.get("localReason").cloned().unwrap_or(serde_json::Value::Null),
                "requestCreatedAt": request.get("createdAt").cloned().unwrap_or(serde_json::Value::Null),
                "requestEvidence": request.get("evidence").cloned().unwrap_or_else(|| serde_json::json!([])),
                "jurors": request.get("jurors").cloned().unwrap_or_else(|| serde_json::json!([])),
                "currentAggregate": aggregate,
                "acceptance": acceptance,
                "verdicts": verdicts,
                "routeAttempts": route_attempts,
                "warnings": warnings,
                "mutatesLocalQuarantineState": false,
            });
            routing::ok_response(
                serde_json::json!({"isReady": true, "errors": [], "package": package}).to_string(),
            )
        }
        _ => routing::not_found_response(),
    }
}
