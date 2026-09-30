async fn route_dispatch_group_4_wishlist(
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
        ("GET", "/api/wishlist") => {
            let mut wishlist = state.wishlist.write().await;
            let json = if route.path.starts_with("/api/v0/") {
                wishlist.get_or_create();
                format!(
                    "[{}]",
                    wishlist
                        .records
                        .iter()
                        .flat_map(|record| record.items.iter())
                        .map(WishlistItem::native_json)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                wishlist.json_array()
            };
            drop(wishlist);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/wishlist") => {
            let search_text = extract_json_string_field(body, "searchText").unwrap_or_default();
            let artist =
                extract_json_string_field(body, "artist").unwrap_or_else(|| search_text.clone());
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind =
                extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_string());
            if artist.trim().is_empty() && title.trim().is_empty() {
                return Ok(routing::bad_request_response("SearchText is required"));
            }
            let filter = extract_json_string_field(body, "filter").unwrap_or_default();
            let enabled = extract_json_bool_field(body, "enabled").unwrap_or(true);
            let auto_download = extract_json_bool_field(body, "autoDownload").unwrap_or(false);
            let max_results = extract_json_u64_field(body, "maxResults").unwrap_or(100);
            if max_results == 0 || max_results > MAX_WISHLIST_RESULTS as u64 {
                return Ok(routing::bad_request_response(
                    "MaxResults must be between 1 and 10000",
                ));
            }
            let max_downloads = extract_json_optional_u64_field(body, "maxDownloads");
            if max_downloads
                .flatten()
                .is_some_and(|value| value == 0 || value > MAX_WISHLIST_DOWNLOADS)
            {
                return Ok(routing::bad_request_response(
                    "MaxDownloads must be null or between 1 and 1000000",
                ));
            }

            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let id = compatibility_contract.then(|| uuid::Uuid::new_v4().to_string());
            match wishlist.add_item_with_contract(
                id,
                artist,
                title,
                kind,
                filter,
                enabled,
                auto_download,
                usize::try_from(max_results).unwrap_or(MAX_WISHLIST_RESULTS),
                max_downloads.flatten(),
            ) {
                Ok(item) => {
                    let mutated = wishlist.clone();
                    let json = if compatibility_contract {
                        item.native_json()
                    } else {
                        item.json()
                    };
                    drop(wishlist);
                    if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                        rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                        return Ok(wishlist_storage_error_response(
                            route.path.starts_with("/api/v0/"),
                            &error,
                        ));
                    }
                    Ok(routing::created_response(json))
                }
                Err(()) => {
                    drop(wishlist);
                    Ok(routing::service_unavailable_response(
                        "wishlist item capacity is full",
                    ))
                }
            }
        }
        ("GET", path) if wishlist_item_action_id(path, "/searches").is_some() => {
            let requested_item_id =
                wishlist_item_action_id(path, "/searches").expect("guarded wishlist history path");
            let native = route.path.starts_with("/api/v0/");
            let wishlist = state.wishlist.read().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, native) else {
                return Ok(routing::not_found_response());
            };
            drop(wishlist);
            let searches = state.searches.read().await;
            let json = searches.wishlist_history_json(&item_id, route.query);
            drop(searches);
            Ok(routing::ok_response(json))
        }
        ("GET", path)
            if path.starts_with("/api/wishlist/") && !path.contains("/ignored-results") =>
        {
            let Some(requested_item_id) = path_segment_after(path, "/api/wishlist/") else {
                return Ok(routing::not_found_response());
            };
            let native = route.path.starts_with("/api/v0/");
            let wishlist = state.wishlist.read().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, native) else {
                return Ok(routing::not_found_response());
            };
            let Some(item) = wishlist.get_item(&item_id) else {
                return Ok(routing::not_found_response());
            };
            Ok(routing::ok_response(
                if route.path.starts_with("/api/v0/") {
                    item.native_json()
                } else {
                    item.json()
                },
            ))
        }
        ("POST", "/api/wishlist/mark-all-viewed") => {
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let items = wishlist.mark_all_viewed();
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) = persist_wishlist_items_checked(state, &items).await {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::no_content_response())
        }
        ("POST", path) if wishlist_item_action_id(path, "/mark-viewed").is_some() => {
            let requested_item_id = wishlist_item_action_id(path, "/mark-viewed")
                .expect("guarded wishlist viewed path");
            let native = route.path.starts_with("/api/v0/");
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, native) else {
                return Ok(routing::not_found_response());
            };
            let previous = wishlist.clone();
            let Some(item) = wishlist.mark_viewed(&item_id) else {
                return Ok(routing::not_found_response());
            };
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::no_content_response())
        }
        ("GET", path) if wishlist_ignored_results_item_id(path).is_some() => {
            let requested_item_id =
                wishlist_ignored_results_item_id(path).expect("guarded ignored path");
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let wishlist = state.wishlist.read().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, compatibility_contract)
            else {
                return Ok(routing::not_found_response());
            };
            let Some(rules) = wishlist.list_ignored_results(&item_id) else {
                return Ok(routing::not_found_response());
            };
            let json = serde_json::Value::Array(
                rules
                    .iter()
                    .map(|rule| {
                        if compatibility_contract {
                            rule.native_json()
                        } else {
                            rule.json()
                        }
                    })
                    .collect(),
            )
            .to_string();
            Ok(routing::ok_response(json))
        }
        ("POST", path) if wishlist_ignored_results_item_id(path).is_some() => {
            let requested_item_id =
                wishlist_ignored_results_item_id(path).expect("guarded ignored path");
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let directory = extract_json_string_field(body, "directory").unwrap_or_default();
            if username.trim().is_empty() || normalize_wishlist_directory(&directory).is_empty() {
                return Ok(routing::bad_request_response(
                    if route.path.starts_with("/api/v0/") {
                        "Username and Directory are required"
                    } else {
                        "username and directory are required"
                    },
                ));
            }

            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let _search_persistence = state.search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, compatibility_contract)
            else {
                return Ok(routing::not_found_response());
            };
            let previous = wishlist.clone();
            let (rule, created) = match wishlist.ignore_result(
                &item_id,
                &username,
                &directory,
                compatibility_contract,
            ) {
                Ok(result) => result,
                Err("not_found") => return Ok(routing::not_found_response()),
                Err("capacity") => {
                    return Ok(routing::service_unavailable_response(
                        "wishlist ignored-result capacity is full",
                    ));
                }
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        if route.path.starts_with("/api/v0/") {
                            "Username and Directory are required"
                        } else {
                            "username and directory are required"
                        },
                    ));
                }
            };
            let mutated = wishlist.clone();
            drop(wishlist);
            if created {
                let (previous_searches, mutated_searches, changed_searches) = {
                    let mut searches = state.searches.write().await;
                    let previous = searches.clone();
                    let changed = searches.suppress_ignored_result(&rule);
                    let mutated = searches.clone();
                    (previous, mutated, changed)
                };
                if let Err(error) = persist_wishlist_ignored_result_and_searches_checked(
                    state,
                    &rule,
                    &changed_searches,
                )
                .await
                {
                    rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                    let mut searches = state.searches.write().await;
                    if *searches == mutated_searches {
                        *searches = previous_searches;
                    }
                    drop(searches);
                    return Ok(wishlist_storage_error_response(
                        route.path.starts_with("/api/v0/"),
                        &error,
                    ));
                }
            }
            let json = if compatibility_contract {
                rule.native_json()
            } else {
                rule.json()
            }
            .to_string();
            if created || compatibility_contract {
                Ok(routing::created_response(json))
            } else {
                Ok(routing::ok_response(json))
            }
        }
        ("DELETE", path) if wishlist_ignored_result_ids(path).is_some() => {
            let (requested_item_id, rule_id) =
                wishlist_ignored_result_ids(path).expect("guarded ignored rule path");
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, compatibility_contract)
            else {
                return Ok(routing::not_found_response());
            };
            let previous = wishlist.clone();
            if !wishlist.delete_ignored_result(&item_id, rule_id) {
                return Ok(routing::not_found_response());
            }
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) =
                persist_wishlist_ignored_result_delete_checked(state, &item_id, rule_id).await
            {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::no_content_response())
        }
        ("DELETE", path) if path.starts_with("/api/wishlist/") => {
            let Some(requested_item_id) = path_segment_after(path, "/api/wishlist/") else {
                return Ok(routing::not_found_response());
            };
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, compatibility_contract)
            else {
                return Ok(if compatibility_contract {
                    routing::no_content_response()
                } else {
                    routing::not_found_response()
                });
            };
            let previous = wishlist.clone();
            if let Some(record) = wishlist.remove_item(&item_id) {
                let mutated = wishlist.clone();
                let json = serde_json::json!({
                    "deleted": true,
                    "item_id": requested_item_id,
                    "remaining": record.items.len(),
                    "updated_at": record.updated_at,
                })
                .to_string();
                drop(wishlist);
                if let Err(error) = persist_wishlist_item_delete_checked(state, &item_id).await {
                    rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                    return Ok(wishlist_storage_error_response(
                        route.path.starts_with("/api/v0/"),
                        &error,
                    ));
                }
                Ok(if route.path.starts_with("/api/v0/") {
                    routing::no_content_response()
                } else {
                    routing::ok_response(json)
                })
            } else {
                drop(wishlist);
                Ok(if route.path.starts_with("/api/v0/") {
                    routing::no_content_response()
                } else {
                    routing::not_found_response()
                })
            }
        }

        // CONTACTS ENDPOINTS
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
