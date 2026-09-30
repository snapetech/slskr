async fn route_dispatch_group_7_podcore(
    context: &RouteDispatchContext<'_, '_>,
) -> RouteDispatchResult {
    let RouteDispatchContext {
        method,
        normalized_path,
        authorization,
        body,
        state,
        route,
        headers,
        state_arc,
        extended_mutation,
        request_is_versioned_v0,
    } = context.clone();
    match (method, normalized_path) {
        ("GET", "/api/podcore/content/search") => {
            let params = route.query.map(query_params).unwrap_or_default();
            let query = params
                .iter()
                .find(|(key, _)| key == "query" || key == "q")
                .map(|(_, value)| value.trim().to_owned())
                .unwrap_or_default();
            if query.is_empty() {
                return Ok(routing::bad_request_response("Search query is required"));
            }
            // The oracle's real backend is a live MusicBrainz recording
            // search. Keep the result as the flat `ContentSearchResult[]`
            // contract rather than the old {query, results, count} wrapper.
            let domain = params
                .iter()
                .find(|(key, _)| key == "domain")
                .map(|(_, value)| value.trim().to_owned())
                .filter(|value| !value.is_empty());
            if domain.is_some_and(|domain| !domain.eq_ignore_ascii_case("audio")) {
                return Ok(routing::ok_response("[]".to_owned()));
            }
            let limit = params
                .iter()
                .find(|(key, _)| key == "limit")
                .and_then(|(_, value)| value.parse::<i64>().ok())
                .unwrap_or(20)
                .clamp(1, 100) as usize;
            let settings = state.integration_settings.read().await.musicbrainz.clone();
            let mut hits = musicbrainz_search_recordings(&settings, &query, limit)
                .await
                .unwrap_or_default();
            if hits.is_empty() {
                // A disconnected or empty MusicBrainz backend must not erase
                // local content search results. The compatibility controller
                // has a real local library, so use it as the bounded fallback
                // while preserving the MusicBrainz-backed result shape.
                let query = query.to_ascii_lowercase();
                let library = state.library.read().await;
                hits = library
                    .records
                    .iter()
                    .filter(|record| {
                        record.kind.eq_ignore_ascii_case("audio")
                            && (record.title.to_ascii_lowercase().contains(&query)
                                || record.artist.to_ascii_lowercase().contains(&query))
                    })
                    .take(limit)
                    .map(|record| MusicBrainzRecordingHit {
                        recording_id: record.id.clone(),
                        title: record.title.clone(),
                        artist: record.artist.clone(),
                        artist_id: None,
                    })
                    .collect();
            }
            let results = hits
                .into_iter()
                .filter(|hit| !hit.recording_id.is_empty())
                .map(|hit| {
                    serde_json::json!({
                        "contentId": format!("content:audio:track:{}", hit.recording_id),
                        "title": hit.title,
                        "subtitle": hit.artist,
                        "type": "track",
                        "domain": "audio",
                        "metadata": {
                            "musicbrainz_recording_id": hit.recording_id,
                            "artist": hit.artist,
                            "title": hit.title,
                            "musicbrainz_artist_id": hit.artist_id.unwrap_or_default(),
                        },
                    })
                })
                .collect::<Vec<_>>();
            Ok(routing::ok_response(
                serde_json::Value::Array(results).to_string(),
            ))
        }

        ("POST", "/api/podcore/membership/join") => {
            let input = match PodJoinSignatureInput::from_json(body) {
                Ok(input) => input,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let mode = state
                .advanced_networking
                .read()
                .await
                .pod_join_signature_mode;
            let now = unix_timestamp();
            let verified = match verify_pod_join_signature(mode, &input, now.saturating_mul(1_000))
            {
                Ok(verified) => verified,
                Err(error) => {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        &error
                    }))
                }
            };
            if mode == PodSignatureMode::Warn && !verified {
                record_daemon_log(
                    state,
                    logging::LogLevel::Warn,
                    "podcore",
                    "accepted unsigned or legacy pod join in warn mode".to_owned(),
                )
                .await;
            }
            if mode == PodSignatureMode::Enforce {
                if let Err(error) = state.pod_join_replays.write().await.reserve(&input, now) {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        &error
                    }));
                }
            }
            let (pod_exists_and_is_not_member, add_result) = {
                let rooms = state.rooms.read().await;
                let pods = state.pods.read().await;
                let mut workflow = state.pod_membership_workflow.write().await;
                let pod_state = pods.get(&input.pod_id).map(|_| {
                    pods.members(&input.pod_id).is_none_or(|members| {
                        !members
                            .iter()
                            .any(|member| member.peer_id.eq_ignore_ascii_case(&input.peer_id))
                    })
                });
                let pod_state = pod_state.or_else(|| {
                    rooms
                        .records
                        .iter()
                        .find(|room| room.name == input.pod_id)
                        .map(|room| {
                            !room
                                .members
                                .iter()
                                .any(|member| member.eq_ignore_ascii_case(&input.peer_id))
                        })
                });
                let add_result = if pod_state == Some(true) {
                    workflow.add_join(input.clone())
                } else {
                    Ok(())
                };
                (pod_state, add_result)
            };
            match pod_exists_and_is_not_member {
                None => {
                    if mode == PodSignatureMode::Enforce {
                        state.pod_join_replays.write().await.release(&input);
                    }
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        "Pod not found"
                    }));
                }
                Some(false) => {
                    if mode == PodSignatureMode::Enforce {
                        state.pod_join_replays.write().await.release(&input);
                    }
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        "Already a member of this pod"
                    }));
                }
                Some(true) => {
                    if let Err(error) = add_result {
                        if mode == PodSignatureMode::Enforce {
                            state.pod_join_replays.write().await.release(&input);
                        }
                        return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                            "Join request could not be processed"
                        } else {
                            error
                        }));
                    }
                }
            }
            let body = serde_json::json!({
                "success": true,
                "podId": input.pod_id,
                "peerId": input.peer_id,
                "joinRequest": input,
                "signatureMode": mode.as_str(),
                "signatureVerified": verified,
            })
            .to_string();
            Ok(routing::ok_response(body))
        }

        ("POST", "/api/podcore/membership/join/accept") => {
            let input = match PodJoinAcceptanceInput::from_json(body) {
                Ok(input) => input,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let mode = state
                .advanced_networking
                .read()
                .await
                .pod_join_signature_mode;
            let verified = match verify_pod_signed_payload(
                mode,
                &input.signature,
                &input.acceptor_public_key,
                input.timestamp_unix_ms,
                unix_timestamp().saturating_mul(1_000),
                &input.canonical_payload(),
                "pod join acceptance",
            ) {
                Ok(verified) => verified,
                Err(error) => {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join acceptance could not be processed"
                    } else {
                        &error
                    }))
                }
            };
            if !pod_acceptor_has_permission(state, &input.pod_id, &input.acceptor_peer_id).await {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Join acceptance could not be processed"
                } else {
                    "Acceptor does not have permission to accept join requests"
                }));
            }
            // Wait for the room store before taking the workflow lock. Pending
            // request reads can continue while this acceptance waits.
            let mut rooms = state.rooms.write().await;
            let mut workflow = state.pod_membership_workflow.write().await;
            let acceptor_still_authorized = matches!(
                workflow.role(&input.pod_id, &input.acceptor_peer_id),
                "owner" | "moderator"
            ) || rooms
                .records
                .iter()
                .find(|room| room.name == input.pod_id)
                .is_some_and(|room| {
                    room.operated
                        && input.acceptor_peer_id.eq_ignore_ascii_case(
                            state.config.username.as_deref().unwrap_or_default(),
                        )
                });
            if !acceptor_still_authorized {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Join acceptance could not be processed"
                } else {
                    "Acceptor does not have permission to accept join requests"
                }));
            }
            let Some(request) = workflow.remove_join(&input.pod_id, &input.peer_id) else {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Join acceptance could not be processed"
                } else {
                    "No pending join request found"
                }));
            };
            let added = rooms.add_member(&input.pod_id, input.peer_id.clone());
            match added {
                Ok(Some(_)) => {}
                Ok(None) => {
                    if let Err(error) = workflow.add_join(request) {
                        eprintln!("pod join acceptance rollback failed: {error}");
                        return Ok(routing::service_unavailable_response(
                            "pod membership rollback failed",
                        ));
                    }
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join acceptance could not be processed"
                    } else {
                        "Pod not found"
                    }));
                }
                Err(()) => {
                    if let Err(error) = workflow.add_join(request) {
                        eprintln!("pod join acceptance rollback failed: {error}");
                        return Ok(routing::service_unavailable_response(
                            "pod membership rollback failed",
                        ));
                    }
                    return Ok(if request_is_versioned_v0 {
                        routing::bad_request_response("Join acceptance could not be processed")
                    } else {
                        routing::service_unavailable_response("pod member capacity is full")
                    });
                }
            }
            workflow.set_role(&input.pod_id, &input.peer_id, input.accepted_role.clone());
            drop(workflow);
            drop(rooms);
            Ok(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "podId": input.pod_id,
                    "peerId": input.peer_id,
                    "operation": "join_acceptance",
                    "request": request,
                    "response": input,
                    "signatureMode": mode.as_str(),
                    "signatureVerified": verified,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/podcore/membership/leave") => {
            let input = match PodLeaveRequestInput::from_json(body) {
                Ok(input) => input,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let mode = state
                .advanced_networking
                .read()
                .await
                .pod_join_signature_mode;
            let verified = match verify_pod_signed_payload(
                mode,
                &input.signature,
                &input.public_key,
                input.timestamp_unix_ms,
                unix_timestamp().saturating_mul(1_000),
                &input.canonical_payload(),
                "pod leave",
            ) {
                Ok(verified) => verified,
                Err(error) => {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Leave request could not be processed"
                    } else {
                        &error
                    }))
                }
            };
            // Decide and apply the leave operation under a room-then-workflow
            // lock order, so membership and role cannot change between the
            // authorization check and the mutation.
            let privileged = {
                let mut rooms = state.rooms.write().await;
                let mut workflow = state.pod_membership_workflow.write().await;
                let is_member = rooms
                    .records
                    .iter()
                    .find(|room| room.name == input.pod_id)
                    .is_some_and(|room| {
                        room.members
                            .iter()
                            .any(|member| member.eq_ignore_ascii_case(&input.peer_id))
                    });
                if !is_member {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Leave request could not be processed"
                    } else {
                        "Not a member of this pod"
                    }));
                }
                let privileged = matches!(
                    workflow.role(&input.pod_id, &input.peer_id),
                    "owner" | "moderator"
                );
                if privileged {
                    if let Err(error) = workflow.add_leave(input.clone()) {
                        return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                            "Leave request could not be processed"
                        } else {
                            error
                        }));
                    }
                } else {
                    rooms.remove_member(&input.pod_id, &input.peer_id);
                    workflow.remove_role(&input.pod_id, &input.peer_id);
                }
                privileged
            };
            Ok(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "podId": input.pod_id,
                    "peerId": input.peer_id,
                    "leaveRequest": input,
                    "pending": privileged,
                    "signatureMode": mode.as_str(),
                    "signatureVerified": verified,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/podcore/membership/leave/accept") => {
            let input = match PodLeaveAcceptanceInput::from_json(body) {
                Ok(input) => input,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let mode = state
                .advanced_networking
                .read()
                .await
                .pod_join_signature_mode;
            let verified = match verify_pod_signed_payload(
                mode,
                &input.signature,
                &input.acceptor_public_key,
                input.timestamp_unix_ms,
                unix_timestamp().saturating_mul(1_000),
                &input.canonical_payload(),
                "pod leave acceptance",
            ) {
                Ok(verified) => verified,
                Err(error) => {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Leave acceptance could not be processed"
                    } else {
                        &error
                    }))
                }
            };
            if !pod_acceptor_has_permission(state, &input.pod_id, &input.acceptor_peer_id).await {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Leave acceptance could not be processed"
                } else {
                    "Acceptor does not have permission to accept leave requests"
                }));
            }
            // Keep the same room-then-workflow order as join acceptance and
            // avoid retaining the workflow lock while room storage is busy.
            let mut rooms = state.rooms.write().await;
            let mut workflow = state.pod_membership_workflow.write().await;
            let acceptor_still_authorized = matches!(
                workflow.role(&input.pod_id, &input.acceptor_peer_id),
                "owner" | "moderator"
            ) || rooms
                .records
                .iter()
                .find(|room| room.name == input.pod_id)
                .is_some_and(|room| {
                    room.operated
                        && input.acceptor_peer_id.eq_ignore_ascii_case(
                            state.config.username.as_deref().unwrap_or_default(),
                        )
                });
            if !acceptor_still_authorized {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Leave acceptance could not be processed"
                } else {
                    "Acceptor does not have permission to accept leave requests"
                }));
            }
            let Some(request) = workflow.remove_leave(&input.pod_id, &input.peer_id) else {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Leave acceptance could not be processed"
                } else {
                    "No pending leave request found"
                }));
            };
            let removed = rooms.remove_member(&input.pod_id, &input.peer_id);
            if removed.is_none() {
                if let Err(error) = workflow.add_leave(request) {
                    eprintln!("pod leave acceptance rollback failed: {error}");
                    return Ok(routing::service_unavailable_response(
                        "pod membership rollback failed",
                    ));
                }
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Leave acceptance could not be processed"
                } else {
                    "Not a member of this pod"
                }));
            }
            workflow.remove_role(&input.pod_id, &input.peer_id);
            drop(workflow);
            drop(rooms);
            Ok(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "podId": input.pod_id,
                    "peerId": input.peer_id,
                    "operation": "leave_acceptance",
                    "request": request,
                    "response": input,
                    "signatureMode": mode.as_str(),
                    "signatureVerified": verified,
                })
                .to_string(),
            ))
        }

        ("GET", path) if pod_pending_request_has_blank_id(path, "join") => {
            Ok(routing::bad_request_response("PodId is required"))
        }

        ("GET", path) if pod_pending_request_has_blank_id(path, "leave") => {
            Ok(routing::bad_request_response("PodId is required"))
        }

        ("GET", path) if pod_pending_request_path(path, "join").is_some() => {
            let pod_id = pod_pending_request_path(path, "join").unwrap_or_default();
            let workflow = state.pod_membership_workflow.read().await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "pendingJoinRequests": workflow.pending_joins(&pod_id),
                })
                .to_string(),
            ))
        }

        ("GET", path) if pod_pending_request_path(path, "leave").is_some() => {
            let pod_id = pod_pending_request_path(path, "leave").unwrap_or_default();
            let workflow = state.pod_membership_workflow.read().await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "pendingLeaveRequests": workflow.pending_leaves(&pod_id),
                })
                .to_string(),
            ))
        }

        ("DELETE", path) if pod_cancel_request_path(path, "join").is_some() => {
            let (pod_id, peer_id) = pod_cancel_request_path(path, "join").unwrap_or_default();
            if state
                .pod_membership_workflow
                .write()
                .await
                .remove_join(&pod_id, &peer_id)
                .is_some()
            {
                Ok(routing::ok_response(r#"{"cancelled":true}"#.to_owned()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        ("DELETE", path) if pod_cancel_request_path(path, "leave").is_some() => {
            let (pod_id, peer_id) = pod_cancel_request_path(path, "leave").unwrap_or_default();
            if state
                .pod_membership_workflow
                .write()
                .await
                .remove_leave(&pod_id, &peer_id)
                .is_some()
            {
                Ok(routing::ok_response(r#"{"cancelled":true}"#.to_owned()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
