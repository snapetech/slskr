use super::*;

#[cfg(feature = "legacy-route-dispatch")]
pub(super) async fn legacy_route_dispatch_group_05(
    context: &super::LegacyRouteDispatchContext<'_, '_>,
) -> Result<HttpResponse, String> {
    let super::LegacyRouteDispatchContext {
        method,
        normalized_path,
        authorization,
        body,
        state,
        route,
        headers,
        extended_mutation,
        request_is_versioned_v0,
    } = *context;
    let _ = (
        authorization,
        headers,
        extended_mutation,
        request_is_versioned_v0,
    );
    match (method, normalized_path.as_str()) {
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
                 let (record, added) = match contacts.create_with_contract(
                     Some(uuid::Uuid::new_v4().to_string()),
                     nickname,
                 ) {
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
             Ok(if added { routing::created_response(json) } else { routing::ok_response(json) })
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
             Ok(if added { routing::created_response(json) } else { routing::ok_response(json) })
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
                    Ok(routing::conflict_response("contact username already exists"))
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
                if let Some(stored) = sharegroups.records.iter_mut().find(|group| group.id == old_id)
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
            let name = extract_json_string_field(body, "name").unwrap_or_else(|| "Untitled".to_string());
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
                let members = record.members.iter()
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
                Ok(if added { routing::created_response(json) } else { routing::ok_response(json) })
              }
              Ok(None) => {
                drop(sharegroups);
                Ok(routing::not_found_response())
              }
              Err(()) => {
                drop(sharegroups);
                Ok(routing::service_unavailable_response("share group member capacity is full"))
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
        ("GET", "/api/users/notes")
            if route.path.starts_with("/api/v0/") || route.path.starts_with("/api/v1/") =>
        {
            let notes = state.user_notes.read().await;
            let value = notes
                .records
                .iter()
                .filter_map(|record| serde_json::from_str::<serde_json::Value>(&record.native_json()).ok())
                .collect::<Vec<_>>();
            Ok(routing::ok_response(serde_json::json!(value).to_string()))
        }
        ("GET", "/api/users/notes") => {
            let notes = state.user_notes.read().await;
            let json = notes.json(None);
            drop(notes);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/users/notes") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let note = extract_json_string_field(body, "note").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::bad_request_response("Username is required."));
            }
            let _user_note_persistence = state.user_note_persistence_lock.lock().await;
            let mut notes = state.user_notes.write().await;
            let previous = notes.clone();
            let versioned = route.path.starts_with("/api/v0/") || route.path.starts_with("/api/v1/");
            let record = if versioned {
                notes.set_versioned(
                    username,
                    note,
                    extract_json_string_field(body, "color").unwrap_or_default(),
                    extract_json_string_field(body, "icon").unwrap_or_default(),
                    extract_json_bool_field(body, "isHighPriority").unwrap_or(false),
                )
            } else {
                notes.create(username, note)
            };
            let Some(record) = record else {
                return Ok(routing::service_unavailable_response("user note capacity is full"));
            };
            let mutated = notes.clone();
            let json = if versioned {
                record.native_json()
            } else {
                record.json()
            };
            drop(notes);
            if let Err(error) = persist_user_note_checked(state, &record).await {
                let mut notes = state.user_notes.write().await;
                if *notes == mutated {
                    *notes = previous;
                }
                drop(notes);
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(if versioned {
                routing::ok_response(json)
            } else {
                routing::created_response(json)
            })
        }
        ("GET", path) if path.starts_with("/api/users/notes/") => {
            let Some(id) = path_segment_after(path, "/api/users/notes/") else {
                return Ok(routing::not_found_response());
            };
            let notes = state.user_notes.read().await;
            let versioned = route.path.starts_with("/api/v0/") || route.path.starts_with("/api/v1/");
            let record = if versioned {
                notes.get_by_username(id)
            } else {
                notes.get(id)
            };
            if let Some(record) = record {
                let json = if versioned {
                    record.native_json()
                } else {
                    record.json()
                };
                drop(notes);
                Ok(routing::ok_response(json))
            } else {
                drop(notes);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/users/notes/") => {
            let Some(id) = path_segment_after(path, "/api/users/notes/") else {
                return Ok(routing::not_found_response());
            };
            let note = extract_json_string_field(body, "note").unwrap_or_default();
            let _user_note_persistence = state.user_note_persistence_lock.lock().await;
            let mut notes = state.user_notes.write().await;
            let previous = notes.clone();
            if let Some(record) = notes.update(id, note) {
                let mutated = notes.clone();
                let json = record.json();
                drop(notes);
                if let Err(error) = persist_user_note_checked(state, &record).await {
                    let mut notes = state.user_notes.write().await;
                    if *notes == mutated {
                        *notes = previous;
                    }
                    drop(notes);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(notes);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/users/notes/") => {
            let Some(id) = path_segment_after(path, "/api/users/notes/") else {
                return Ok(routing::not_found_response());
            };
            let _user_note_persistence = state.user_note_persistence_lock.lock().await;
            let mut notes = state.user_notes.write().await;
            let previous = notes.clone();
            let versioned = route.path.starts_with("/api/v0/") || route.path.starts_with("/api/v1/");
            let removed_id = if versioned {
                notes.delete_by_username(id).map(|record| record.id)
            } else {
                notes.delete(id).then(|| id.to_owned())
            };
            let mutated = notes.clone();
            drop(notes);
            if let Some(removed_id) = removed_id {
                if let Err(error) = persist_user_note_delete_checked(state, &removed_id).await {
                    let mut notes = state.user_notes.write().await;
                    if *notes == mutated {
                        *notes = previous;
                    }
                    drop(notes);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(if versioned {
                    routing::no_content_response()
                } else {
                    routing::ok_response("{}".to_string())
                })
            } else if versioned {
                Ok(routing::no_content_response())
            } else {
                Ok(routing::not_found_response())
            }
        }

        // INTERESTS ENDPOINTS (Liked)
        ("GET", "/api/soulseek/interests") => {
            let interests = state.interests.read().await;
            let json = interests.json_liked();
            drop(interests);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/soulseek/interests") => {
            let versioned = route.path.starts_with("/api/v0/");
            let name = extract_json_string_field(body, "item")
                .or_else(|| extract_json_string_field(body, "name"))
                .map(|name| name.trim().to_owned())
                .unwrap_or_default();
            if name.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            if versioned && state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response(
                    "Soulseek session is disconnected",
                ));
            }
            let _interest_persistence = state.interest_persistence_lock.lock().await;
            let mut interests = state.interests.write().await;
            let previous = interests.clone();
            let Some((record, created)) = interests.add_liked(name) else {
                return Ok(routing::service_unavailable_response("liked interest capacity is full"));
            };
            let mutated = interests.clone();
            let json = record.json();
            drop(interests);
            if created {
                if let Err(error) = persist_interest_checked(state, &record).await {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(_interest_persistence);
                if versioned {
                    if let Err(error) = send_active_interest_command(
                        state,
                        ServerMessage::AddThingILike { item: record.name.clone() },
                    )
                    .await
                    {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::created_response(json))
                }
            } else if versioned {
                drop(_interest_persistence);
                if let Err(error) = send_active_interest_command(
                    state,
                    ServerMessage::AddThingILike {
                        item: record.name.clone(),
                    },
                )
                .await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::no_content_response())
            } else {
                Ok(routing::ok_response(json))
            }
        }
        ("DELETE", path) if path.starts_with("/api/soulseek/interests/") => {
            let versioned = route.path.starts_with("/api/v0/");
            let Some(raw_item) = path_segment_after(path, "/api/soulseek/interests/") else {
                return Ok(routing::not_found_response());
            };
            let item = decoded_path_segment(raw_item).trim().to_owned();
            if item.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            if versioned && state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response("Soulseek session is disconnected"));
            }
            let _interest_persistence = state.interest_persistence_lock.lock().await;
            let mut interests = state.interests.write().await;
            let previous = interests.clone();
            let id = interests
                .liked
                .iter()
                .find(|record| {
                    record.id.eq_ignore_ascii_case(&item)
                        || record.name.eq_ignore_ascii_case(&item)
                })
                .map(|record| record.id.clone());
            let deleted = id
                .as_deref()
                .is_some_and(|id| interests.remove_liked(id));
            let mutated = interests.clone();
            drop(interests);
            if deleted {
                let existing_id = id.as_deref().expect("deleted liked interest id");
                let published_item = id
                    .as_deref()
                    .and_then(|id| {
                        previous
                            .liked
                            .iter()
                            .find(|record| record.id == id)
                            .map(|record| record.name.clone())
                    })
                    .unwrap_or_else(|| item.clone());
                if let Err(error) = persist_interest_delete_checked(state, existing_id).await {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(_interest_persistence);
                if versioned {
                    if let Err(error) = send_active_interest_command(
                        state,
                        ServerMessage::RemoveThingILike {
                            item: published_item,
                        },
                    ).await {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::ok_response("{}".to_string()))
                }
            } else if versioned {
                drop(_interest_persistence);
                if let Err(error) = send_active_interest_command(
                    state,
                    ServerMessage::RemoveThingILike { item },
                )
                .await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::no_content_response())
            } else {
                Ok(routing::not_found_response())
            }
        }

        // INTERESTS ENDPOINTS (Hated)
        ("GET", "/api/soulseek/hated-interests") => {
            let interests = state.interests.read().await;
            let json = interests.json_hated();
            drop(interests);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/soulseek/hated-interests") => {
            let versioned = route.path.starts_with("/api/v0/");
            let name = extract_json_string_field(body, "item")
                .or_else(|| extract_json_string_field(body, "name"))
                .map(|name| name.trim().to_owned())
                .unwrap_or_default();
            if name.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            if versioned && state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response(
                    "Soulseek session is disconnected",
                ));
            }
            let _interest_persistence = state.interest_persistence_lock.lock().await;
            let mut interests = state.interests.write().await;
            let previous = interests.clone();
            let Some((record, created)) = interests.add_hated(name) else {
                return Ok(routing::service_unavailable_response("hated interest capacity is full"));
            };
            let mutated = interests.clone();
            let json = record.json();
            drop(interests);
            if created {
                if let Err(error) = persist_interest_checked(state, &record).await {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(_interest_persistence);
                if versioned {
                    if let Err(error) = send_active_interest_command(
                        state,
                        ServerMessage::AddThingIHate { item: record.name.clone() },
                    )
                    .await
                    {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::created_response(json))
                }
            } else if versioned {
                drop(_interest_persistence);
                if let Err(error) = send_active_interest_command(
                    state,
                    ServerMessage::AddThingIHate {
                        item: record.name.clone(),
                    },
                )
                .await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::no_content_response())
            } else {
                Ok(routing::ok_response(json))
            }
        }
        ("DELETE", path) if path.starts_with("/api/soulseek/hated-interests/") => {
            let versioned = route.path.starts_with("/api/v0/");
            let Some(raw_item) = path_segment_after(path, "/api/soulseek/hated-interests/") else {
                return Ok(routing::not_found_response());
            };
            let item = decoded_path_segment(raw_item).trim().to_owned();
            if item.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            if versioned && state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response("Soulseek session is disconnected"));
            }
            let _interest_persistence = state.interest_persistence_lock.lock().await;
            let mut interests = state.interests.write().await;
            let previous = interests.clone();
            let id = interests
                .hated
                .iter()
                .find(|record| {
                    record.id.eq_ignore_ascii_case(&item)
                        || record.name.eq_ignore_ascii_case(&item)
                })
                .map(|record| record.id.clone());
            let deleted = id
                .as_deref()
                .is_some_and(|id| interests.remove_hated(id));
            let mutated = interests.clone();
            drop(interests);
            if deleted {
                let existing_id = id.as_deref().expect("deleted hated interest id");
                let published_item = id
                    .as_deref()
                    .and_then(|id| {
                        previous
                            .hated
                            .iter()
                            .find(|record| record.id == id)
                            .map(|record| record.name.clone())
                    })
                    .unwrap_or_else(|| item.clone());
                if let Err(error) = persist_interest_delete_checked(state, existing_id).await {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(_interest_persistence);
                if versioned {
                    if let Err(error) = send_active_interest_command(
                        state,
                        ServerMessage::RemoveThingIHate {
                            item: published_item,
                        },
                    ).await {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::ok_response("{}".to_string()))
                }
            } else if versioned {
                drop(_interest_persistence);
                if let Err(error) = send_active_interest_command(
                    state,
                    ServerMessage::RemoveThingIHate { item },
                )
                .await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::no_content_response())
            } else {
                Ok(routing::not_found_response())
            }
        }

        // SHARE GRANTS ENDPOINTS
        ("GET", "/api/share-grants") => {
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let grants = state.share_grants.read().await;
            let records = grants.records.clone();
            drop(grants);
            let mut visible = Vec::with_capacity(records.len());
            for record in &records {
                if !share_grant_collection_forbids(state, &record.collection_id, caller_id.as_deref())
                    .await
                {
                    visible.push(record.json());
                }
            }
            Ok(routing::ok_response(format!("[{}]", visible.join(","))))
        }
        ("POST", "/api/share-grants") => {
            let collection_id = extract_json_string_field(body, "collection_id")
                .or_else(|| extract_json_string_field(body, "collectionId"))
                .unwrap_or_default();
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            if !collection_id.is_empty() {
                let collections = state.collections.read().await;
                let collection_exists = collections.get(&collection_id).is_some();
                drop(collections);
                if !collection_exists {
                    return Ok(routing::not_found_response());
                }
                // Matches the oracle's real Create: a grant may only be
                // created against a collection the caller actually owns.
                if share_grant_collection_forbids(state, &collection_id, caller_id.as_deref()).await {
                    return Ok(routing::not_found_response());
                }
            }
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let Some(username) = normalize_share_grant_username(&username) else {
                return Ok(routing::conflict_response("collection_id and username are required"));
            };
            if collection_id.is_empty() {
                return Ok(routing::conflict_response("collection_id and username are required"));
            }
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let id = compatibility_contract.then(|| uuid::Uuid::new_v4().to_string());
            let _collection_grant_persistence =
                state.collection_grant_persistence_lock.lock().await;
            if share_grant_collection_forbids(state, &collection_id, caller_id.as_deref()).await {
                return Ok(routing::not_found_response());
            }
            let mut grants = state.share_grants.write().await;
            let previous = grants.clone();
            let requested_limit = match crate::share_stream_limits::request_limit(body) {
                Ok(limit) => limit,
                Err(error) => return Ok(routing::bad_request_response(error)),
            };
            let Some((record, created)) = grants.create_with_contract_and_permissions(
                id,
                collection_id,
                username,
                "download,stream",
            ) else {
                return Ok(routing::service_unavailable_response("share grant capacity is full"));
            };
            let record = if created {
                match requested_limit {
                    Some(limit) => grants.set_stream_limit(&record.id, limit).expect("created grant remains owned"),
                    None => record,
                }
            } else { record };
            let json = record.json();
            let mutated = grants.clone();
            drop(grants);
            if created {
                if let Err(error) = persist_share_grant(state, &record).await {
                    let mut grants = state.share_grants.write().await;
                    if route_dispatch::share_grant_store_matches(&grants, &mutated) {
                        *grants = previous;
                    }
                    drop(grants);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::created_response(json))
            } else {
                Ok(routing::ok_response(json))
            }
        }
        ("GET", path)
            if path.starts_with("/api/share-grants/")
                && path.ends_with("/manifest")
                && share_grant_manifest_id(path).is_some() =>
        {
            let grant_id = share_grant_manifest_id(path)
                .expect("guarded share-grant manifest path");
            if query_parameter(route.query, "token").is_some() {
                return Ok(routing::bad_request_response(
                    "share tokens must be sent in X-Share-Token",
                ));
            }
            let api_authorized = !state.config.auth_required
                || is_authorized(
                    &state.config,
                    authorization,
                    headers.cookie.as_deref(),
                );
            let share_token = request_share_token(authorization, headers);
            if let Some(token) = share_token {
                let mut tokens = state.share_access_tokens.write().await;
                let token_record = tokens.validate(&token);
                drop(tokens);
                if token_record.as_ref().map(|record| record.grant_id.as_str()) != Some(grant_id) {
                    return Ok(routing::unauthorized_response());
                }
            } else if !api_authorized {
                return Ok(routing::unauthorized_response());
            }

            let grants = state.share_grants.read().await;
            let Some(grant) = grants.get(grant_id) else {
                drop(grants);
                return Ok(routing::not_found_response());
            };
            drop(grants);
            let collections = state.collections.read().await;
            let Some(collection) = collections.get(&grant.collection_id) else {
                drop(collections);
                return Ok(routing::not_found_response());
            };
            let items = collection
                .items
                .iter()
                .map(|item| {
                    serde_json::from_str::<serde_json::Value>(&item.json())
                        .unwrap_or_else(|_| serde_json::json!({ "id": item.id }))
                })
                .collect::<Vec<_>>();
            let item_count = items.len();
            let collection_value = serde_json::from_str::<serde_json::Value>(&collection.json())
                .unwrap_or_else(|_| serde_json::json!({ "id": collection.id }));
            drop(collections);
            Ok(routing::ok_response(
                serde_json::json!({
                    "share": serde_json::from_str::<serde_json::Value>(&grant.json())
                        .unwrap_or_else(|_| serde_json::json!({ "id": grant.id })),
                    "collection": collection_value,
                    "items": items,
                    "itemCount": item_count,
                    "permissions": grant.permissions,
                })
                .to_string(),
            ))
        }
        ("GET", path)
            if path.starts_with("/api/share-grants/") && share_grant_resource_id(path).is_some() =>
        {
            let id = share_grant_resource_id(path).expect("guarded share-grant resource path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let grants = state.share_grants.read().await;
            let record = grants.get(id);
            drop(grants);
            let Some(record) = record else {
                return Ok(routing::not_found_response());
            };
            if share_grant_collection_forbids(state, &record.collection_id, caller_id.as_deref()).await
            {
                return Ok(routing::not_found_response());
            }
            Ok(routing::ok_response(record.json()))
        }
        ("GET", path)
            if path.starts_with("/api/share-grants/by-collection/")
                && share_grant_collection_id(path).is_some() =>
        {
            let collection_id = share_grant_collection_id(path)
                .expect("guarded share-grant collection path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            // SharesController.GetByCollection first resolves the collection
            // and returns NotFound when the collection no longer exists.
            let collection_exists = state.collections.read().await.get(collection_id).is_some();
            if !collection_exists {
                return Ok(routing::not_found_response());
            }
            // Matches the oracle's real GetByCollection: this is the
            // "outgoing shares" owner-perspective view, so it 404s unless
            // the caller actually owns this collection.
            if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await {
                return Ok(routing::not_found_response());
            }
            let grants = state.share_grants.read().await;
            let records = grants.get_by_collection(collection_id);
            let json = records.iter()
                .map(|r| r.json())
                .collect::<Vec<_>>()
                .join(",");
            let response = format!("[{}]", json);
            drop(grants);
            Ok(routing::ok_response(response))
        }
        ("PUT", path)
            if path.starts_with("/api/share-grants/") && share_grant_resource_id(path).is_some() =>
        {
            let id = share_grant_resource_id(path).expect("guarded share-grant resource path");
            let permissions = extract_json_string_field(body, "permissions").unwrap_or_else(|| "read".to_string());
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_grant_persistence =
                state.collection_grant_persistence_lock.lock().await;
            let mut grants = state.share_grants.write().await;
            let owning_collection_id = grants.get(id).map(|record| record.collection_id.clone());
            if let Some(collection_id) = owning_collection_id.as_deref() {
                if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await {
                    drop(grants);
                    return Ok(routing::not_found_response());
                }
            }
            let previous = grants.clone();
            let requested_limit = match crate::share_stream_limits::request_limit(body) {
                Ok(limit) => limit,
                Err(error) => return Ok(routing::bad_request_response(error)),
            };
            let permissions = if requested_limit.is_some() && extract_json_string_field(body, "permissions").is_none() {
                grants.get(id).map(|record| record.permissions).unwrap_or(permissions)
            } else { permissions };
            if let Some(record) = grants.update(id, permissions) {
                let record = match requested_limit {
                    Some(limit) => grants.set_stream_limit(id, limit).expect("updated grant remains owned"),
                    None => record,
                };
                let json = record.json();
                if let Err(error) = persist_share_grant(state, &record).await {
                    *grants = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(grants);
                Ok(routing::ok_response(json))
            } else {
                drop(grants);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/share-grants/") && share_grant_resource_id(path).is_some() =>
        {
            let id = share_grant_resource_id(path).expect("guarded share-grant resource path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_grant_persistence =
                state.collection_grant_persistence_lock.lock().await;
            let mut grants = state.share_grants.write().await;
            let owning_collection_id = grants.get(id).map(|record| record.collection_id.clone());
            if let Some(collection_id) = owning_collection_id.as_deref() {
                if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await {
                    drop(grants);
                    return Ok(routing::not_found_response());
                }
            }
            let previous = grants.clone();
            let deleted = grants.delete(id);
            if deleted {
                if let Err(error) = persist_share_grant_delete_checked(state, id).await {
                    *grants = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(grants);
                state.share_access_tokens.write().await.revoke_grant(id);
                state
                    .stream_tickets
                    .write()
                    .await
                    .revoke_source(&format!("share:{id}"));
                Ok(routing::ok_response("{}".to_string()))
            } else {
                drop(grants);
                Ok(routing::not_found_response())
            }
        }

        // LIBRARY ITEMS ENDPOINTS
        ("GET", "/api/library/items") => {
            if state.config.controller_profile
                == ControllerProfile::Native
            {
                return Ok(routing::ok_response(
                    native_library_items_search_json(state, route.query).await,
                ));
            }
            let library = state.library.read().await;
            let json = library.json();
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/items/browser") => {
            Ok(library_browser_response(state, route.query).await)
        }
        ("POST", "/api/library/items") => {
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind = extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_string());
            let _library_persistence = state.library_persistence_lock.lock().await;
            let mut library = state.library.write().await;
            let previous = library.clone();
            let Some(record) = library.create(artist, title, kind) else {
                return Ok(routing::service_unavailable_response("library item capacity is full"));
            };
            let mutated = library.clone();
            let json = record.json();
            drop(library);
            if let Err(error) = persist_library_item_checked(state, &record).await {
                rollback_library_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/items/") => {
            let Some(id) = path_segment_after(path, "/api/library/items/") else {
                return Ok(routing::not_found_response());
            };
            let library = state.library.read().await;
            if let Some(record) = library.get(id) {
                let json = record.json();
                drop(library);
                Ok(routing::ok_response(json))
            } else {
                drop(library);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/library/items/") => {
            let Some(id) = path_segment_after(path, "/api/library/items/") else {
                return Ok(routing::not_found_response());
            };
            let _library_persistence = state.library_persistence_lock.lock().await;
            let mut library = state.library.write().await;
            let previous = library.clone();
            let deleted = library.delete(id);
            let mutated = library.clone();
            drop(library);
            if deleted {
                if let Err(error) = persist_library_item_delete_checked(state, id).await {
                    rollback_library_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // DESTINATIONS ENDPOINTS
        ("GET", "/api/destinations") => {
            let destinations = state.destinations.read().await;
            let json = if destinations.records.is_empty() {
                let configured = DestinationStore::from_config(
                    &state.config.downloads_dir,
                    &state.config.core_workflow.destinations,
                );
                if route.path.starts_with("/api/v0/") {
                    configured.versioned_list()
                } else {
                    configured.list()
                }
            } else if route.path.starts_with("/api/v0/") {
                destinations.versioned_list()
            } else {
                destinations.list()
            };
            drop(destinations);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/destinations/default") => {
            let destinations = state.destinations.read().await;
            let json = if destinations.records.is_empty() {
                let configured = DestinationStore::from_config(
                    &state.config.downloads_dir,
                    &state.config.core_workflow.destinations,
                );
                if route.path.starts_with("/api/v0/") {
                    configured.versioned_default()
                } else {
                    configured.default()
                }
            } else if route.path.starts_with("/api/v0/") {
                destinations.versioned_default()
            } else {
                destinations.default()
            };
            drop(destinations);
            Ok(routing::ok_response(json))
        }

        // BROWSE ENDPOINTS
        ("GET", path)
            if path.starts_with("/api/users/")
                && path.ends_with("/browse/status")
                && user_route_username(path, "/browse/status").is_some() =>
        {
            let username = user_route_username(path, "/browse/status")
                .expect("guarded user browse status path");
            if state.runtime.read().await.relay_agent_enabled {
                return Ok(controller_forbidden_response());
            }
            let browse = state.browse.read().await;
            let tracked = browse
                .records
                .iter()
                .find(|record| record.username == username)
                .map(BrowseRecord::controller_status_json);
            drop(browse);
            match tracked {
                Some(body) => Ok(routing::ok_response(body)),
                None => Ok(routing::not_found_response()),
            }
        }
        // ADDITIONAL MISSING USER ENDPOINTS (Phase 5)
        ("GET", "/api/profile/me") => {
            if route.path.starts_with("/api/v0/") {
                let session = state.session.read().await;
                let display_name = session
                    .username
                    .clone()
                    .or_else(|| state.config.username.clone())
                    .unwrap_or_else(|| "Unknown".to_owned())
                    .trim()
                    .to_owned();
                drop(session);
                let descriptor = match local_capability_descriptor(state).await {
                    Ok(descriptor) => descriptor,
                    Err(error) => return Ok(routing::service_unavailable_response(&error)),
                };
                let peer_id = local_profile_peer_id(state);
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "peerId": peer_id,
                        "publicKey": STANDARD.encode(descriptor.public_key),
                        "displayName": display_name,
                        "avatar": null,
                        "capabilities": 0,
                        "endpoints": [],
                        "createdAt": unix_seconds_rfc3339(descriptor.issued_at_unix),
                        "expiresAt": (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339(),
                        "signature": descriptor.signature.map(|signature| STANDARD.encode(signature)).unwrap_or_default(),
                    })
                    .to_string(),
                ));
            }
            let session = state.session.read().await;
            let username = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "local".to_owned());
            let privileges_seconds = session.privileges_seconds.unwrap_or(0);
            let connected = session.state == "connected";
            drop(session);
            let users = state.users.read().await;
            let watched = users
                .records
                .iter()
                .any(|user| user.username.eq_ignore_ascii_case(&username) && user.watched);
            drop(users);
            let descriptor = match local_capability_descriptor(state).await {
                Ok(descriptor) => descriptor,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            let json = serde_json::json!({
                "peerId": descriptor.peer_id,
                "publicKey": STANDARD.encode(descriptor.public_key),
                "displayName": username.clone(),
                "capabilities": descriptor.features,
                "endpoints": descriptor.endpoints,
                "createdAt": descriptor.issued_at_unix,
                "expiresAt": descriptor.expires_at_unix,
                "signature": descriptor.signature.map(|signature| STANDARD.encode(signature)).unwrap_or_default(),
                "username": username,
                "description": "",
                "picture": "",
                "user_type": if privileges_seconds > 0 { "privileged" } else { "normal" },
                "privilegesSeconds": privileges_seconds,
                "connected": connected,
                "watched": watched,
            }).to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", path)
            if route.path.starts_with("/api/v0/") && path.starts_with("/api/profile/") =>
        {
            let Some(raw_peer_id) = path.strip_prefix("/api/profile/") else {
                return Ok(routing::not_found_response());
            };
            if raw_peer_id.is_empty() {
                return Ok(routing::bad_request_response("PeerId is required."));
            }
            if raw_peer_id.contains('/') {
                return Ok(routing::not_found_response());
            }
            let peer_id = decoded_path_segment(raw_peer_id).trim().to_owned();
            if peer_id.is_empty() {
                return Ok(routing::bad_request_response("PeerId is required."));
            }
            let local_peer_id = local_profile_peer_id(state);
            if !peer_id.eq_ignore_ascii_case(&local_peer_id) {
                return Ok(routing::not_found_response());
            }
            let session = state.session.read().await;
            let display_name = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "Unknown".to_owned())
                .trim()
                .to_owned();
            drop(session);
            return Ok(routing::ok_response(
                serde_json::json!({
                    "peerId": local_peer_id,
                    "displayName": display_name,
                    "avatar": null,
                    "capabilities": 0,
                    "endpoints": [],
                })
                .to_string(),
            ));
        }

        ("GET", path) if path.starts_with("/api/profile/") => {
            let Some(username) = path_segment_after(path, "/api/profile/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            let users = state.users.read().await;
            let user = users
                .records
                .iter()
                .find(|user| user.username.eq_ignore_ascii_case(&username))
                .cloned();
            drop(users);
            let json = serde_json::json!({
                "username": username,
                "description": "",
                "picture": "",
                "user_type": "normal",
                "watched": user.as_ref().is_some_and(|user| user.watched),
                "status": user.as_ref().and_then(|user| user.status.clone()).unwrap_or_else(|| "Unknown".to_owned()),
                "averageSpeed": user.as_ref().and_then(|user| user.average_speed).unwrap_or(0),
                "uploadCount": user.as_ref().and_then(|user| user.upload_count).unwrap_or(0),
                "fileCount": user.as_ref().and_then(|user| user.file_count).unwrap_or(0),
                "directoryCount": user.as_ref().and_then(|user| user.directory_count).unwrap_or(0),
            }).to_string();
            Ok(routing::ok_response(json))
        }

        // CONVERSATIONS ENDPOINT
        ("GET", "/api/conversations") => {
            if let Some(response) = controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let messages = state.messages.read().await;
            let body = messages.controller_conversations_json(route.query);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/conversations/activity/unacknowledged") => {
            if let Some(response) = controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let messages = state.messages.read().await;
            let body = messages.has_unacknowledged_messages().to_string();
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", path) if conversation_messages_path(path).is_some() => {
            let Some(username) = conversation_messages_path(path) else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if let Some(response) = controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let unacknowledged_only = match query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "unAcknowledgedOnly")
                .map(|(_, value)| value)
            {
                None => false,
                Some(value) => match parse_bool_value(&value) {
                    Some(value) => value,
                    None => {
                        return Ok(routing::bad_request_response(
                            "unAcknowledgedOnly must be a boolean",
                        ))
                    }
                },
            };
            let messages = state.messages.read().await;
            if state.config.controller_profile == ControllerProfile::Legacy
                && !messages.records.iter().any(|record| record.username == username)
            {
                drop(messages);
                return Ok(routing::not_found_response());
            }
            let body = messages.controller_messages_json(&username, unacknowledged_only);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if let Some(response) = controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let include_messages = match query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "includeMessages")
            {
                None => true,
                Some((_, value)) => match parse_bool_value(&value) {
                    Some(value) => value,
                    None => {
                        return Ok(routing::bad_request_response(
                            "includeMessages must be a boolean",
                        ))
                    }
                },
            };
            let since = match query_millis_parameter(route.query, "since") {
                Ok(value) => value,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let messages = state.messages.read().await;
            if state.config.controller_profile == ControllerProfile::Legacy
                && !messages.records.iter().any(|record| record.username == username)
            {
                drop(messages);
                return Ok(routing::not_found_response());
            }
            let body = messages.controller_conversation_json(&username, include_messages, since);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("POST", "/api/conversations/batch") => {
            if conversation_batch_exceeds_wire_limits(body) {
                return Ok(routing::bad_request_response(
                    "conversation batch exceeds recipient limits",
                ));
            }
            let usernames = match extract_json_string_array_field(body, "usernames")
                .or_else(|| extract_json_string_array_field(body, "recipients"))
            {
                Some(usernames) => usernames,
                None => {
                    return Ok(routing::bad_request_response(
                        "usernames/recipients array is required",
                    ))
                }
            };
            let message_body = match extract_json_string_field(body, "body")
                .or_else(|| extract_json_string_field(body, "message"))
            {
                Some(body) => body,
                None => return Ok(routing::bad_request_response("body/message is required")),
            };
            let command = match private_message_users_command(usernames, message_body.clone()) {
                Ok(ServerMessage::MessageUsers { usernames, .. }) => usernames,
                Ok(_) => return Ok(routing::bad_request_response("invalid message command")),
                Err(error) => return Ok(routing::bad_request_response(&error.to_string())),
            };

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };

            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let records: Vec<_> = command
                .iter()
                .map(|username| messages.add(username.clone(), "outbound", message_body.clone()))
                .collect();
            let mutated = messages.clone();
            drop(messages);

            if let Err(error) = persist_message_records_checked(state, &records).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_message_persistence);
            session_command_permit.send(SessionCommand::MessageUsers {
                usernames: command.clone(),
                body: message_body,
            });

            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                Ok(HttpResponse {
                    status: "201 Created",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(routing::created_response(
                    serde_json::json!({
                        "conversations": records.iter().map(MessageRecord::json).collect::<Vec<_>>(),
                        "usernames": command,
                        "count": records.len(),
                    })
                    .to_string(),
                ))
            }
        }
        ("POST", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username).trim().to_owned();
            let message_body = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "message"))
                .or_else(|| extract_json_string_field(body, "body"))
                .unwrap_or_default();
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if message_body.trim().is_empty() {
                return Ok(routing::bad_request_response("message is required"));
            }
            if route.path.starts_with("/api/v0/")
                && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let record = messages.add(username.clone(), "outbound", message_body.clone());
            let mutated = messages.clone();
            drop(messages);
            if let Err(error) = persist_message_record_checked(state, &record).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_message_persistence);
            session_command_permit.send(SessionCommand::MessageUser {
                username,
                body: message_body,
            });
            Ok(if matches!(
                state.config.controller_profile,
                ControllerProfile::Legacy | ControllerProfile::Native
            )
                && route.path.starts_with("/api/v0/")
            {
                HttpResponse {
                    status: "201 Created",
                    content_type: "",
                    body: String::new(),
                }
            } else {
                routing::ok_response((record.id > 0).to_string())
            })
        }

        // JOBS ENDPOINT
        ("GET", "/api/jobs") => {
            let searches = state.searches.read().await;
            let transfers = state.transfers.read().await;
            let mut jobs = searches
                .records
                .iter()
                .map(|record| {
                    serde_json::json!({
                        "id": record.id,
                        "kind": "search",
                        "type": "search",
                        "status": record.status,
                        "progress": {
                            "releases_total": 0,
                            "releases_done": if record.status == "completed" { 1 } else { 0 },
                            "releases_failed": 0,
                        },
                        "progress_percent": if record.status == "completed" { 100 } else { 50 },
                        "query": record.query,
                        "created_at": record.created_at,
                        "updated_at": record.updated_at,
                    })
                })
                .collect::<Vec<_>>();
            jobs.extend(transfers.entries.iter().map(|entry| {
                let size = entry.size.unwrap_or(0);
                let progress = entry
                    .bytes_transferred
                    .saturating_mul(100)
                    .checked_div(size)
                    .unwrap_or(0)
                    .min(100);
                    serde_json::json!({
                        "id": format!("transfer-{}", entry.id),
                        "kind": "transfer",
                        "type": "transfer",
                        "status": entry.status,
                        "progress": {
                            "releases_total": 0,
                            "releases_done": 0,
                        "releases_failed": if is_failed_transfer_status(&entry.status) { 1 } else { 0 },
                        },
                        "progress_percent": progress,
                        "filename": entry.filename,
                        "created_at": entry.requested_at,
                        "updated_at": entry.updated_at,
                })
            }));
            let total = jobs.len();
            drop(transfers);
            drop(searches);
            Ok(routing::ok_response(serde_json::json!({
                "jobs": jobs,
                "limit": 100,
                "offset": 0,
                "total": total,
                "has_more": total > 100,
            }).to_string()))
        }
        _ => Err(LEGACY_ROUTE_NOT_HANDLED.to_owned()),
    }
    .inspect(complete_legacy_request_span)
}
