use super::*;

pub(super) async fn controller_events_read_failure_response(
    state: &AppState,
    path: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Legacy || !path.starts_with("/api/v0/")
    {
        return None;
    }
    let db = state.db.as_ref()?;
    if db.list_events(1, 0).await.is_err() {
        return Some(routing::internal_server_error_response(
            "event storage unavailable",
        ));
    }
    None
}

pub(super) async fn controller_native_events_read_failure_response(
    state: &AppState,
    path: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Native || path != "/api/v0/events" {
        return None;
    }
    let db = state.db.as_ref()?;
    if db.list_events(1, 0).await.is_err() {
        return Some(routing::internal_server_error_response(
            "Failed to list events",
        ));
    }
    None
}

pub(super) async fn controller_search_responses_read_failure_response(
    state: &AppState,
    path: &str,
    search_id: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Legacy
        || !path.starts_with("/api/v0/searches/")
        || !path.ends_with("/responses")
    {
        return None;
    }
    let db = state.db.as_ref()?;
    if db.list_search_results(Some(search_id), 1, 0).await.is_err() {
        return Some(routing::internal_server_error_response(
            "search storage unavailable",
        ));
    }
    None
}

fn controller_native_search_guid_from_path(path: &str) -> Option<&str> {
    let value = path.strip_prefix("/api/v0/searches/")?;
    let value = value.strip_suffix("/responses").unwrap_or(value);
    let value = value
        .split_once("/items/")
        .map_or(value, |(search_id, _)| search_id);
    if matches!(value, "cleanup" | "prune" | "records") {
        return None;
    }
    (value.split('/').count() == 1 && !value.is_empty()).then_some(value)
}

pub(super) fn controller_search_id_is_valid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok()
        || (value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

pub(super) fn controller_search_identifier_is_valid(value: &str) -> bool {
    controller_search_id_is_valid(value) || value.parse::<u32>().is_ok()
}

pub(super) fn controller_native_search_query_validation(
    method: &str,
    path: &str,
    query: Option<&str>,
) -> Option<HttpResponse> {
    let params = query_params(query.unwrap_or_default());
    let invalid_i32 = |name: &str| {
        params.iter().any(|(key, value)| {
            key.eq_ignore_ascii_case(name)
                && (value.parse::<i32>().is_err()
                    || value.parse::<i32>().is_ok_and(|value| value < 0))
        })
    };
    let invalid_optional_bool = |name: &str| {
        params
            .iter()
            .any(|(key, value)| key.eq_ignore_ascii_case(name) && parse_bool_value(value).is_none())
    };

    if (method == "GET"
        && path == "/api/v0/searches"
        && (invalid_i32("limit") || invalid_i32("offset")))
        || (method == "GET"
            && controller_native_search_guid_from_path(path).is_some()
            && !path.ends_with("/responses")
            && invalid_optional_bool("includeResponses"))
        || (method == "POST"
            && path == "/api/v0/searches/cleanup"
            && (invalid_i32("maxAgeDays") || invalid_i32("maxCount")))
    {
        return Some(routing::bad_request_response("The request is invalid"));
    }
    None
}

pub(super) async fn controller_native_search_storage_failure_response(
    state: &AppState,
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Native
        || !path.starts_with("/api/v0/searches")
    {
        return None;
    }

    let id_route = (method == "GET" && path != "/api/v0/searches")
        || matches!(method, "DELETE" | "PUT")
        || (method == "POST" && path.contains("/items/"));
    if id_route {
        if let Some(search_id) = controller_native_search_guid_from_path(path) {
            // The frozen controller binds real IDs as GUIDs.  slskR also
            // retains its historical numeric-token aliases for the v0
            // compatibility surface, so reject malformed text while
            // allowing those numeric aliases to reach the normal handler.
            if !controller_search_identifier_is_valid(search_id) {
                return Some(routing::bad_request_response("The request is invalid"));
            }
        }
    }

    let db = state.db.as_ref()?;
    let result = if method == "GET" && path == "/api/v0/searches" {
        db.list_searches(1, 0).await.map(|_| ())
    } else if method == "GET" && path.ends_with("/responses") {
        if let Some(search_id) = controller_native_search_guid_from_path(path) {
            db.list_search_results(Some(search_id), 1, 0)
                .await
                .map(|_| ())
        } else {
            Ok(())
        }
    } else if method == "GET" {
        if let Some(search_id) = controller_native_search_guid_from_path(path) {
            db.get_search(search_id).await.map(|_| ())
        } else {
            Ok(())
        }
    } else if method == "DELETE" && path == "/api/v0/searches" {
        db.list_searches(1, 0).await.map(|_| ())
    } else if matches!(method, "DELETE" | "PUT") {
        if let Some(search_id) = controller_native_search_guid_from_path(path) {
            db.get_search(search_id).await.map(|_| ())
        } else {
            Ok(())
        }
    } else if method == "POST" && path == "/api/v0/searches/cleanup" {
        db.list_searches(1, 0).await.map(|_| ())
    } else if method == "POST" && path.contains("/items/") {
        if let Some(search_id) = controller_native_search_guid_from_path(path) {
            db.get_search(search_id).await.map(|_| ())
        } else {
            Ok(())
        }
    } else {
        return None;
    };

    result
        .err()
        .map(|_| routing::internal_server_error_response("search storage unavailable"))
}

pub(super) async fn controller_native_hashdb_read_failure_response(
    state: &AppState,
    path: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Native
        || !path.starts_with("/api/v0/hashdb/")
        // HashDbController.GenerateKey is a pure key derivation operation;
        // it does not touch SQLite and remains available while the database
        // is unavailable.
        || path == "/api/v0/hashdb/key"
    {
        return None;
    }
    let db = state.db.as_ref()?;
    if db.list_hash_db_entries().await.is_err() || db.get_hash_db_state("latest_seq").await.is_err()
    {
        return Some(routing::internal_server_error_response(
            "hash database storage unavailable",
        ));
    }
    None
}

pub(super) async fn controller_native_hashdb_write_failure_response(
    state: &AppState,
    path: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Native
        || !path.starts_with("/api/v0/hashdb/")
    {
        return None;
    }
    let db = state.db.as_ref()?;
    if db.list_hash_db_entries().await.is_err() || db.get_hash_db_state("latest_seq").await.is_err()
    {
        return Some(routing::internal_server_error_response(
            "hash database storage unavailable",
        ));
    }
    None
}

pub(super) async fn controller_native_backfill_candidates_read_failure_response(
    state: &AppState,
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Native
        || method != "GET"
        || path != "/api/v0/backfill/candidates"
    {
        return None;
    }
    let db = state.db.as_ref()?;
    if db.list_hash_db_entries().await.is_err() {
        return Some(routing::internal_server_error_response(
            "backfill storage unavailable",
        ));
    }
    None
}

pub(super) async fn controller_native_backfill_file_write_failure_response(
    state: &AppState,
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Native
        || method != "POST"
        || path != "/api/v0/backfill/file"
    {
        return None;
    }
    let db = state.db.as_ref()?;
    if db.list_hash_db_entries().await.is_err() {
        return Some(routing::internal_server_error_response(
            "backfill storage unavailable",
        ));
    }
    None
}
