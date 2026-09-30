use super::*;

pub(super) fn wishlist_storage_error_response(
    is_versioned_v0: bool,
    message: &str,
) -> HttpResponse {
    if is_versioned_v0 {
        routing::internal_server_error_response(message)
    } else {
        routing::service_unavailable_response(message)
    }
}

fn persisted_wishlist_item(item: &WishlistItem) -> crate::persistence::WishlistItemRecord {
    crate::persistence::WishlistItemRecord {
        id: item.id.clone(),
        artist: item.artist.clone(),
        title: item.title.clone(),
        kind: item.kind.clone(),
        filter: item.filter.clone(),
        enabled: item.enabled,
        auto_download: item.auto_download,
        max_results: i64::try_from(item.max_results).unwrap_or(i64::MAX),
        max_downloads: item
            .max_downloads
            .map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        last_viewed_at: item
            .last_viewed_at
            .map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        last_searched_at: item
            .last_searched_at
            .map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        last_match_count: i64::try_from(item.last_match_count).unwrap_or(i64::MAX),
        last_visible_hit_count: i64::try_from(item.last_visible_hit_count).unwrap_or(i64::MAX),
        last_hidden_locked_hit_count: i64::try_from(item.last_hidden_locked_hit_count)
            .unwrap_or(i64::MAX),
        last_filtered_out_hit_count: i64::try_from(item.last_filtered_out_hit_count)
            .unwrap_or(i64::MAX),
        last_ignored_result_hit_count: i64::try_from(item.last_ignored_result_hit_count)
            .unwrap_or(i64::MAX),
        last_response_count: i64::try_from(item.last_response_count).unwrap_or(i64::MAX),
        total_search_count: i64::try_from(item.total_search_count).unwrap_or(i64::MAX),
        total_download_count: i64::try_from(item.total_download_count).unwrap_or(i64::MAX),
        last_search_id: item.last_search_id.clone(),
        lidarr_album_id: item.lidarr_album_id,
        lidarr_track_id: item.lidarr_track_id,
        lidarr_track_count: item.lidarr_track_count,
        lidarr_duration_seconds: item.lidarr_duration_seconds,
        lidarr_release_disambiguation: item.lidarr_release_disambiguation.clone(),
        added_at: i64::try_from(item.added_at).unwrap_or(i64::MAX),
    }
}

pub(super) async fn persist_wishlist_item_checked(
    state: &AppState,
    item: &WishlistItem,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.upsert_wishlist_item(&persisted_wishlist_item(item))
        .await
        .map_err(|error| format!("wishlist persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_wishlist_items_checked(
    state: &AppState,
    items: &[WishlistItem],
) -> Result<bool, String> {
    if items.is_empty() {
        let Some(db) = state.db.as_ref() else {
            return Ok(false);
        };
        db.list_wishlist_items(1, 0)
            .await
            .map_err(|error| format!("wishlist persistence failed: {error}"))?;
        return Ok(true);
    }
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = items
        .iter()
        .map(persisted_wishlist_item)
        .collect::<Vec<_>>();
    db.upsert_wishlist_items(&persisted)
        .await
        .map_err(|error| format!("wishlist persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_wishlist_item_delete_checked(
    state: &AppState,
    id: &str,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.delete_wishlist_item(id)
        .await
        .map_err(|error| format!("wishlist deletion persistence failed: {error}"))?;
    Ok(true)
}

fn persisted_wishlist_ignored_result(
    rule: &WishlistIgnoredResult,
) -> crate::persistence::WishlistIgnoredResultRecord {
    crate::persistence::WishlistIgnoredResultRecord {
        id: rule.id.clone(),
        wishlist_item_id: rule.wishlist_item_id.clone(),
        username: rule.username.clone(),
        directory: rule.directory.clone(),
        created_at: i64::try_from(rule.created_at).unwrap_or(i64::MAX),
    }
}

pub(super) async fn persist_wishlist_ignored_result_and_searches_checked(
    state: &AppState,
    rule: &WishlistIgnoredResult,
    searches: &[SearchRecord],
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let searches = searches
        .iter()
        .map(|search| {
            (
                persisted_search_record(search),
                search.id.clone(),
                persisted_search_result_records(search),
            )
        })
        .collect::<Vec<_>>();
    db.upsert_wishlist_ignored_result_and_searches(
        &persisted_wishlist_ignored_result(rule),
        &searches,
    )
    .await
    .map_err(|error| format!("wishlist ignored-result transaction failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_wishlist_ignored_result_delete_checked(
    state: &AppState,
    item_id: &str,
    rule_id: &str,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let deleted = db
        .delete_wishlist_ignored_result(item_id, rule_id)
        .await
        .map_err(|error| format!("wishlist ignored-result deletion failed: {error}"))?;
    Ok(deleted > 0)
}

pub(super) async fn rollback_wishlist_if_unchanged(
    state: &AppState,
    previous: WishlistStore,
    mutated: &WishlistStore,
) {
    let mut wishlist = state.wishlist.write().await;
    if *wishlist == *mutated {
        *wishlist = previous;
    }
}

pub(super) async fn load_wishlist_store(
    db: Option<&crate::persistence::DatabaseManager>,
) -> Result<WishlistStore, String> {
    let Some(db) = db else {
        return Ok(WishlistStore::new());
    };
    let records = db
        .list_wishlist_items(EVENT_HISTORY_LIMIT as i32, 0)
        .await
        .map_err(|error| format!("failed to load persisted wishlist items: {error}"))?;
    let ignored_results = db
        .list_all_wishlist_ignored_results()
        .await
        .map_err(|error| format!("failed to load persisted wishlist ignored results: {error}"))?;
    Ok(WishlistStore::from_persisted_with_ignored(
        records,
        ignored_results,
    ))
}
