async fn route_dispatch_group_4_contacts_sharegroups(
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
        ("GET", "/api/contacts/nearby") => {
            let contacts = state.contacts.read().await;
            let json = contacts.nearby_json(route.query);
            drop(contacts);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/contacts") => {
            let contacts = state.contacts.read().await;
            let json = contacts.json_array(route.query);
            drop(contacts);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/contacts") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::conflict_response("username is required"));
            }
            let _contact_persistence = state.contact_persistence_lock.lock().await;
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            let (record, created) = match contacts.create(username) {
                Ok(result) => result,
                Err(()) => {
                    return Ok(routing::service_unavailable_response(
                        "contact capacity is full",
                    ));
                }
            };
            let mutated = contacts.clone();
            let json = record.json();
            drop(contacts);
            if created {
                if let Err(error) = persist_contact_checked(state, &record).await {
                    let mut contacts = state.contacts.write().await;
                    if *contacts == mutated {
                        *contacts = previous;
                    }
                    drop(contacts);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::created_response(json))
            } else {
                Ok(routing::conflict_response("contact already exists"))
            }
        }
        ("POST", "/api/contacts/from-discovery") => {
            if route.path.starts_with("/api/v0/") {
                let peer_id = extract_json_string_field(body, "peerId")
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                let nickname = extract_json_string_field(body, "nickname")
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                if peer_id.trim().is_empty() {
                    return Ok(routing::bad_request_response("PeerId is required."));
                }
                if nickname.trim().is_empty() {
                    return Ok(routing::bad_request_response("Nickname is required."));
                }
                // The profile service always has the local signed profile
                // available.  Unknown peers still require a cached/fetched
                // profile, which this in-process compatibility layer does
                // not synthesize from a username alone.
                if !peer_id.eq_ignore_ascii_case(&local_profile_peer_id(state)) {
                    return Ok(HttpResponse {
                        status: "404 Not Found",
                        content_type: "application/json",
                        body: serde_json::json!("Profile not found.").to_string(),
                    });
                }
                let _contact_persistence = state.contact_persistence_lock.lock().await;
                let mut contacts = state.contacts.write().await;
                let previous = contacts.clone();
                let (record, added) = match contacts
                    .create_with_contract(Some(uuid::Uuid::new_v4().to_string()), nickname)
                {
                    Ok(result) => result,
                    Err(()) => {
                        return Ok(routing::service_unavailable_response(
                            "contact capacity is full",
                        ));
                    }
                };
                let mutated = contacts.clone();
                let json = record.native_json(&peer_id);
                drop(contacts);
                if added {
                    if let Err(error) = persist_contact_checked(state, &record).await {
                        let mut contacts = state.contacts.write().await;
                        if *contacts == mutated {
                            *contacts = previous;
                        }
                        drop(contacts);
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
                return Ok(if added {
                    routing::created_response(json)
                } else {
                    routing::ok_response(json)
                });
            }
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            let _contact_persistence = state.contact_persistence_lock.lock().await;
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            let (record, added) = match contacts.create(username.clone()) {
                Ok(result) => result,
                Err(()) => {
                    return Ok(routing::service_unavailable_response(
                        "contact capacity is full",
                    ));
                }
            };
            let mutated = contacts.clone();
            drop(contacts);
            if added {
                if let Err(error) = persist_contact_checked(state, &record).await {
                    let mut contacts = state.contacts.write().await;
                    if *contacts == mutated {
                        *contacts = previous;
                    }
                    drop(contacts);
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            let json = format!(
                "{{\"username\":\"{}\",\"discovered\":true,\"added\":{added}}}",
                json_escape(&username),
            );
            Ok(if added {
                routing::created_response(json)
            } else {
                routing::ok_response(json)
            })
        }
        ("POST", "/api/contacts/from-invite") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            let id = route
                .path
                .starts_with("/api/v0/")
                .then(|| uuid::Uuid::new_v4().to_string());
            let _contact_persistence = state.contact_persistence_lock.lock().await;
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            let (record, added) = match contacts.create_with_contract(id, username.clone()) {
                Ok(result) => result,
                Err(()) => {
                    return Ok(routing::service_unavailable_response(
                        "contact capacity is full",
                    ));
                }
            };
            let mutated = contacts.clone();
            drop(contacts);
            if added {
                if let Err(error) = persist_contact_checked(state, &record).await {
                    let mut contacts = state.contacts.write().await;
                    if *contacts == mutated {
                        *contacts = previous;
                    }
                    drop(contacts);
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            let json = format!(
                "{{\"username\":\"{}\",\"invited\":true,\"accepted\":true,\"added\":{added}}}",
                json_escape(&username),
            );
            Ok(if added {
                routing::created_response(json)
            } else {
                routing::ok_response(json)
            })
        }
        ("GET", path) if path.starts_with("/api/contacts/") => {
            let Some(id) = path_segment_after(path, "/api/contacts/") else {
                return Ok(routing::not_found_response());
            };
            let contacts = state.contacts.read().await;
            if let Some(record) = contacts.get(id) {
                let json = record.json();
                drop(contacts);
                Ok(routing::ok_response(json))
            } else {
                drop(contacts);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/contacts/") => {
            let Some(id) = path_segment_after(path, "/api/contacts/") else {
                return Ok(routing::not_found_response());
            };
            let username = extract_json_string_field(body, "username");
            let online = extract_json_bool_field(body, "online");
            let _contact_persistence = state.contact_persistence_lock.lock().await;
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            match contacts.update(id, username, online) {
                Ok(record) => {
                    let mutated = contacts.clone();
                    let json = record.json();
                    drop(contacts);
                    if let Err(error) = persist_contact_checked(state, &record).await {
                        let mut contacts = state.contacts.write().await;
                        if *contacts == mutated {
                            *contacts = previous;
                        }
                        drop(contacts);
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::ok_response(json))
                }
                Err(ContactUpdateError::DuplicateUsername) => {
                    drop(contacts);
                    Ok(routing::conflict_response(
                        "contact username already exists",
                    ))
                }
                Err(ContactUpdateError::NotFound) => {
                    drop(contacts);
                    Ok(routing::not_found_response())
                }
            }
        }
        ("DELETE", path) if path.starts_with("/api/contacts/") => {
            let Some(id) = path_segment_after(path, "/api/contacts/") else {
                return Ok(routing::not_found_response());
            };
            let _contact_persistence = state.contact_persistence_lock.lock().await;
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            let deleted = contacts.delete(id);
            let mutated = contacts.clone();
            drop(contacts);
            if deleted {
                if let Err(error) = persist_contact_delete_checked(state, id).await {
                    let mut contacts = state.contacts.write().await;
                    if *contacts == mutated {
                        *contacts = previous;
                    }
                    drop(contacts);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // SHAREGROUPS ENDPOINTS
        ("GET", "/api/sharegroups") => {
            let sharegroups = state.sharegroups.read().await;
            let json = sharegroups.json_array(route.query);
            drop(sharegroups);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/sharegroups") => {
            let requested_name = extract_json_string_field(body, "name");
            let name = if route.path.starts_with("/api/v0/") {
                let Some(name) = requested_name else {
                    return Ok(routing::bad_request_response("Name is required."));
                };
                if name.trim().is_empty() {
                    return Ok(routing::bad_request_response("Name is required."));
                }
                name
            } else {
                requested_name.unwrap_or_else(|| "Untitled".to_string())
            };
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            let _share_group_persistence = state.share_group_persistence_lock.lock().await;
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            let Some(mut record) = sharegroups.create(name, description) else {
                return Ok(routing::service_unavailable_response(
                    "share group capacity is full",
                ));
            };
            let json = if route.path.starts_with("/api/v0/") {
                let old_id = record.id.clone();
                record.id = uuid::Uuid::new_v4().to_string();
                if let Some(stored) = sharegroups
                    .records
                    .iter_mut()
                    .find(|group| group.id == old_id)
                {
                    stored.id.clone_from(&record.id);
                }
                serde_json::json!({
                    "id": record.id,
                    "name": record.name,
                    "ownerUserId": "Anonymous",
                    "createdAt": unix_seconds_rfc3339(record.created_at),
                    "updatedAt": unix_seconds_rfc3339(record.updated_at),
                })
                .to_string()
            } else {
                record.json()
            };
            let mutated = sharegroups.clone();
            drop(sharegroups);
            if let Err(error) = persist_share_group(state, &record).await {
                let mut sharegroups = state.sharegroups.write().await;
                if share_group_store_matches(&sharegroups, &mutated) {
                    *sharegroups = previous;
                }
                drop(sharegroups);
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(json))
        }
        ("GET", path)
            if path.starts_with("/api/sharegroups/") && share_group_resource_id(path).is_some() =>
        {
            let id = share_group_resource_id(path).expect("guarded share-group resource path");
            let sharegroups = state.sharegroups.read().await;
            if let Some(record) = sharegroups.get(id) {
                let json = record.json();
                drop(sharegroups);
                Ok(routing::ok_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path)
            if path.starts_with("/api/sharegroups/") && share_group_resource_id(path).is_some() =>
        {
            let id = share_group_resource_id(path).expect("guarded share-group resource path");
            let name =
                extract_json_string_field(body, "name").unwrap_or_else(|| "Untitled".to_string());
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            let _share_group_persistence = state.share_group_persistence_lock.lock().await;
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            if let Some(record) = sharegroups.update(id, name, description) {
                let json = record.json();
                let mutated = sharegroups.clone();
                drop(sharegroups);
                if let Err(error) = persist_share_group(state, &record).await {
                    let mut sharegroups = state.sharegroups.write().await;
                    if share_group_store_matches(&sharegroups, &mutated) {
                        *sharegroups = previous;
                    }
                    drop(sharegroups);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/sharegroups/") && share_group_resource_id(path).is_some() =>
        {
            let id = share_group_resource_id(path).expect("guarded share-group resource path");
            let _share_group_persistence = state.share_group_persistence_lock.lock().await;
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            let deleted = sharegroups.delete(id);
            if deleted {
                let mutated = sharegroups.clone();
                drop(sharegroups);
                if let Err(error) = persist_share_group_delete(state, id).await {
                    let mut sharegroups = state.sharegroups.write().await;
                    if share_group_store_matches(&sharegroups, &mutated) {
                        *sharegroups = previous;
                    }
                    drop(sharegroups);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response("{}".to_string()))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("GET", path)
            if path.starts_with("/api/sharegroups/")
                && path.ends_with("/members")
                && share_group_members_id(path).is_some() =>
        {
            let id = share_group_members_id(path).expect("guarded share-group members path");
            let sharegroups = state.sharegroups.read().await;
            if let Some(record) = sharegroups.get(id) {
                let members = record
                    .members
                    .iter()
                    .map(|m| m.json())
                    .collect::<Vec<_>>()
                    .join(",");
                let json = format!("[{}]", members);
                drop(sharegroups);
                Ok(routing::ok_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path)
            if path.starts_with("/api/sharegroups/")
                && path.ends_with("/members")
                && share_group_members_id(path).is_some() =>
        {
            let id = share_group_members_id(path).expect("guarded share-group members path");
            if route.path.starts_with("/api/v0/")
                && state.sharegroups.read().await.get(id).is_none()
            {
                return Ok(routing::not_found_response());
            }
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::conflict_response("username is required"));
            }
            let _share_group_persistence = state.share_group_persistence_lock.lock().await;
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            match sharegroups.add_member(id, username.clone()) {
                Ok(Some((record, added))) => {
                    let member = record
                        .members
                        .iter()
                        .find(|member| member.username.eq_ignore_ascii_case(&username))
                        .cloned();
                    let json = member
                        .as_ref()
                        .map(ShareGroupMember::json)
                        .unwrap_or_else(|| {
                            format!(
                                "{{\"username\":\"{}\",\"added_at\":{}}}",
                                json_escape(&username),
                                unix_timestamp()
                            )
                        });
                    let mutated = sharegroups.clone();
                    drop(sharegroups);
                    if added {
                        if let Err(error) = persist_share_group(state, &record).await {
                            let mut sharegroups = state.sharegroups.write().await;
                            if share_group_store_matches(&sharegroups, &mutated) {
                                *sharegroups = previous;
                            }
                            drop(sharegroups);
                            return Ok(routing::service_unavailable_response(&error));
                        }
                    }
                    Ok(if added {
                        routing::created_response(json)
                    } else {
                        routing::ok_response(json)
                    })
                }
                Ok(None) => {
                    drop(sharegroups);
                    Ok(routing::not_found_response())
                }
                Err(()) => {
                    drop(sharegroups);
                    Ok(routing::service_unavailable_response(
                        "share group member capacity is full",
                    ))
                }
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/sharegroups/")
                && path.contains("/members/")
                && share_group_member_path(path).is_some() =>
        {
            let (id, username) =
                share_group_member_path(path).expect("guarded share-group member path");
            let _share_group_persistence = state.share_group_persistence_lock.lock().await;
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            if let Some(record) = sharegroups.remove_member(id, &username) {
                let mutated = sharegroups.clone();
                drop(sharegroups);
                if let Err(error) = persist_share_group(state, &record).await {
                    let mut sharegroups = state.sharegroups.write().await;
                    if share_group_store_matches(&sharegroups, &mutated) {
                        *sharegroups = previous;
                    }
                    drop(sharegroups);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response("{}".to_string()))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }

        // USER NOTES ENDPOINTS
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
