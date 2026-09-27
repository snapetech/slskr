use super::*;

pub(super) async fn sync_lidarr_wanted_to_wishlist(
    state: &AppState,
    lidarr: &config::LidarrIntegrationSettings,
) -> Result<serde_json::Value, String> {
    let mut wanted_count = 0_u64;
    let mut created_count = 0_u64;
    let mut duplicate_count = 0_u64;
    let mut skipped_count = 0_u64;
    if !lidarr.enabled {
        return Ok(serde_json::json!({
            "enabled": false,
            "wantedCount": wanted_count,
            "createdCount": created_count,
            "duplicateCount": duplicate_count,
            "skippedCount": skipped_count,
        }));
    }

    let mut page = 1_u64;
    const PAGE_SIZE: u64 = 250;
    let mut quality_profiles: Option<Vec<serde_json::Value>> = None;
    let mut monitored_releases = BTreeMap::<i64, Option<serde_json::Value>>::new();

    loop {
        let response = fetch_lidarr_wanted_missing(lidarr, page, PAGE_SIZE).await?;
        wanted_count = response
            .get("totalRecords")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default();
        let raw_records = response
            .get("records")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        if raw_records.is_empty() {
            break;
        }

        let mut records = Vec::with_capacity(raw_records.len());
        for mut album in raw_records {
            let album_id = album
                .get("id")
                .and_then(serde_json::Value::as_i64)
                .filter(|value| *value > 0);
            let quality_profile_id = album
                .get("profileId")
                .or_else(|| album.pointer("/artist/qualityProfileId"))
                .and_then(serde_json::Value::as_i64)
                .filter(|value| *value > 0);
            if quality_profile_id.is_some() && quality_profiles.is_none() {
                quality_profiles = Some(
                    fetch_lidarr_json_get(lidarr, "/api/v1/qualityprofile", "quality profiles")
                        .await?
                        .as_array()
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            let mut selected_filter = lidarr.wishlist_filter.clone();
            if let Some(profile_id) = quality_profile_id {
                if let Some(profile) = quality_profiles.as_ref().and_then(|profiles| {
                    profiles.iter().find(|profile| {
                        profile.get("id").and_then(serde_json::Value::as_i64) == Some(profile_id)
                    })
                }) {
                    match lidarr_quality_profile_filter(profile) {
                        Some(filter) if !filter.is_empty() => selected_filter = filter,
                        None => selected_filter.clear(),
                        _ => {}
                    }
                }
            }
            let monitored_release = if state.config.current_upstream_behavior {
                if let Some(album_id) = album_id {
                    let needs_release_fetch = matches!(
                        monitored_releases.entry(album_id),
                        std::collections::btree_map::Entry::Vacant(_)
                    );
                    if needs_release_fetch {
                        let release = fetch_lidarr_json_get(
                            lidarr,
                            &format!("/api/v1/album/{album_id}"),
                            "album release detail",
                        )
                        .await
                        .ok()
                        .and_then(|detail| {
                            detail
                                .get("releases")
                                .and_then(serde_json::Value::as_array)
                                .and_then(|releases| {
                                    releases
                                        .iter()
                                        .find(|release| {
                                            release
                                                .get("monitored")
                                                .and_then(serde_json::Value::as_bool)
                                                .unwrap_or(false)
                                        })
                                        .or_else(|| releases.first())
                                        .cloned()
                                })
                        });
                        monitored_releases.insert(album_id, release);
                    }
                    monitored_releases.get(&album_id).cloned().flatten()
                } else {
                    None
                }
            } else {
                None
            };
            let track_file_count = album
                .pointer("/statistics/trackFileCount")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default();
            let total_track_count = album
                .pointer("/statistics/trackCount")
                .and_then(serde_json::Value::as_i64)
                .or_else(|| {
                    album
                        .pointer("/statistics/totalTrackCount")
                        .and_then(serde_json::Value::as_i64)
                })
                .unwrap_or_default();
            let partially_missing = state.config.current_upstream_behavior
                && album_id.is_some()
                && track_file_count > 0
                && total_track_count > track_file_count;
            let expected_track_count = monitored_release
                .as_ref()
                .and_then(|release| release.get("trackCount"))
                .and_then(serde_json::Value::as_i64)
                .filter(|value| *value > 0)
                .or_else(|| (total_track_count > 0).then_some(total_track_count));
            let release_duration_seconds = monitored_release
                .as_ref()
                .and_then(|release| release.get("duration"))
                .and_then(serde_json::Value::as_i64)
                .filter(|value| *value > 0)
                .map(|value| (value / 1_000).max(1));
            let release_disambiguation = monitored_release
                .as_ref()
                .and_then(|release| {
                    release
                        .get("disambiguation")
                        .or_else(|| release.get("title"))
                })
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned);
            album["__slskrSelectedFilter"] = serde_json::json!(selected_filter);
            album["__slskrExpectedTrackCount"] = expected_track_count
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
            album["__slskrReleaseDurationSeconds"] = release_duration_seconds
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
            album["__slskrReleaseDisambiguation"] = release_disambiguation
                .clone()
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
            if let Some(album_id) = album_id.filter(|_| partially_missing) {
                let tracks = fetch_lidarr_json_get(
                    lidarr,
                    &format!("/api/v1/track?albumId={album_id}"),
                    "album tracks",
                )
                .await?
                .as_array()
                .cloned()
                .unwrap_or_default();
                let album_title = album
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .trim();
                let artist = album
                    .pointer("/artist/artistName")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .trim();
                let missing_tracks = tracks
                    .into_iter()
                    .filter(|track| {
                        !track
                            .get("hasFile")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false)
                            && track
                                .get("trackFileId")
                                .and_then(serde_json::Value::as_i64)
                                .unwrap_or_default()
                                <= 0
                    })
                    .filter_map(|track| {
                        let track_id = track
                            .get("id")
                            .and_then(serde_json::Value::as_i64)
                            .filter(|value| *value > 0)?;
                        let track_title = track
                            .get("title")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .trim();
                        if track_title.is_empty() {
                            return None;
                        }
                        let track_number = track
                            .get("trackNumber")
                            .map(|value| match value {
                                serde_json::Value::String(value) => value.clone(),
                                value => value.to_string(),
                            })
                            .unwrap_or_default();
                        let title = [album_title, track_number.trim(), track_title]
                            .into_iter()
                            .filter(|value| !value.is_empty())
                            .collect::<Vec<_>>()
                            .join(" ");
                        let search_text = [artist, title.as_str()]
                            .into_iter()
                            .filter(|value| !value.is_empty())
                            .collect::<Vec<_>>()
                            .join(" ");
                        let duration_seconds = track
                            .get("duration")
                            .and_then(serde_json::Value::as_i64)
                            .filter(|value| *value > 0)
                            .map(|value| (value / 1_000).max(1));
                        Some(serde_json::json!({
                            "id": track_id,
                            "artist": artist,
                            "title": title,
                            "searchText": search_text,
                            "durationSeconds": duration_seconds,
                            "releaseDisambiguation": release_disambiguation.clone(),
                        }))
                    })
                    .collect::<Vec<_>>();
                album["__slskrPartialAlbum"] = serde_json::json!(true);
                album["__slskrMissingTracks"] = serde_json::Value::Array(missing_tracks);
            }
            records.push(album);
        }

        let mut created = Vec::new();
        let mut updated = Vec::new();
        let mut reached_cap = false;
        let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
        let (previous_wishlist, mutated_wishlist) = {
            let mut wishlist = state.wishlist.write().await;
            let previous_wishlist = wishlist.clone();
            let mut existing = wishlist
                .records
                .iter()
                .flat_map(|record| record.items.iter())
                .map(|item| {
                    format!(
                        "{}\u{1f}{}",
                        item.search_text().trim().to_ascii_lowercase(),
                        item.filter.trim().to_ascii_lowercase()
                    )
                })
                .collect::<HashSet<_>>();
            for album in records {
                if created_count.saturating_add(created.len() as u64) >= lidarr.max_items_per_sync {
                    reached_cap = true;
                    break;
                }
                let title = album
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                let artist = album
                    .pointer("/artist/artistName")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                let selected_filter = album
                    .get("__slskrSelectedFilter")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(lidarr.wishlist_filter.as_str())
                    .to_owned();
                let lidarr_album_id = album
                    .get("id")
                    .and_then(serde_json::Value::as_i64)
                    .filter(|value| *value > 0);
                let lidarr_track_count = album
                    .pointer("/statistics/trackCount")
                    .and_then(serde_json::Value::as_i64)
                    .or_else(|| {
                        album
                            .pointer("/statistics/totalTrackCount")
                            .and_then(serde_json::Value::as_i64)
                    })
                    .filter(|value| *value > 0);
                let expected_track_count = album
                    .get("__slskrExpectedTrackCount")
                    .and_then(serde_json::Value::as_i64)
                    .filter(|value| *value > 0);
                let release_duration_seconds = album
                    .get("__slskrReleaseDurationSeconds")
                    .and_then(serde_json::Value::as_i64)
                    .filter(|value| *value > 0);
                let release_disambiguation = album
                    .get("__slskrReleaseDisambiguation")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned);
                let lidarr_track_count = expected_track_count.or(lidarr_track_count);
                let lidarr_duration_seconds = release_duration_seconds.or_else(|| {
                    album
                        .get("duration")
                        .and_then(serde_json::Value::as_i64)
                        .filter(|value| *value > 0)
                        .map(|value| (value / 1_000).max(1))
                });
                let lidarr_release_disambiguation = release_disambiguation.clone().or_else(|| {
                    album
                        .get("disambiguation")
                        .or_else(|| album.get("releaseDisambiguation"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned)
                });
                let search_text = match (artist.is_empty(), title.is_empty()) {
                    (true, true) => String::new(),
                    (true, false) => title.clone(),
                    (false, true) => artist.clone(),
                    (false, false) => format!("{artist} {title}"),
                };
                if search_text.is_empty() {
                    skipped_count = skipped_count.saturating_add(1);
                    continue;
                }

                if album
                    .get("__slskrPartialAlbum")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
                {
                    if let Some(album_id) = lidarr_album_id {
                        let missing_track_ids = album
                            .get("__slskrMissingTracks")
                            .and_then(serde_json::Value::as_array)
                            .into_iter()
                            .flatten()
                            .filter_map(|track| track.get("id").and_then(serde_json::Value::as_i64))
                            .collect::<HashSet<_>>();
                        updated.extend(
                            wishlist.disable_lidarr_items_for_album(album_id, &missing_track_ids),
                        );
                    }
                    let missing_tracks = album
                        .get("__slskrMissingTracks")
                        .and_then(serde_json::Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    if missing_tracks.is_empty() {
                        skipped_count = skipped_count.saturating_add(1);
                        continue;
                    }
                    for track in missing_tracks {
                        if created_count.saturating_add(created.len() as u64)
                            >= lidarr.max_items_per_sync
                        {
                            reached_cap = true;
                            break;
                        }
                        let track_id = track
                            .get("id")
                            .and_then(serde_json::Value::as_i64)
                            .filter(|value| *value > 0);
                        let artist = track
                            .get("artist")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned();
                        let title = track
                            .get("title")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned();
                        let search_text = track
                            .get("searchText")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .trim()
                            .to_owned();
                        if search_text.is_empty() {
                            skipped_count = skipped_count.saturating_add(1);
                            continue;
                        }
                        let key = format!(
                            "{}\u{1f}{}",
                            search_text.to_ascii_lowercase(),
                            selected_filter.trim().to_ascii_lowercase()
                        );
                        if !existing.insert(key) {
                            duplicate_count = duplicate_count.saturating_add(1);
                            continue;
                        }
                        let item = wishlist.add_item_with_settings(
                            artist,
                            title,
                            "Audio".to_owned(),
                            selected_filter.clone(),
                            true,
                            lidarr.auto_download,
                            usize::try_from(lidarr.wishlist_max_results)
                                .unwrap_or(MAX_WISHLIST_RESULTS),
                            Some(1),
                        );
                        match item {
                            Ok(item) => {
                                let item = wishlist
                                    .update_lidarr_metadata(
                                        &item.id,
                                        lidarr_album_id,
                                        track_id,
                                        None,
                                        track
                                            .get("durationSeconds")
                                            .and_then(serde_json::Value::as_i64),
                                        track
                                            .get("releaseDisambiguation")
                                            .and_then(serde_json::Value::as_str)
                                            .map(str::to_owned)
                                            .or_else(|| lidarr_release_disambiguation.clone()),
                                    )
                                    .unwrap_or(item);
                                created.push(item);
                            }
                            Err(()) => {
                                skipped_count = skipped_count.saturating_add(1);
                                reached_cap = true;
                                break;
                            }
                        }
                    }
                    if reached_cap {
                        break;
                    }
                    continue;
                }
                let key = format!(
                    "{}\u{1f}{}",
                    search_text.to_ascii_lowercase(),
                    selected_filter.trim().to_ascii_lowercase()
                );
                if !existing.insert(key) {
                    duplicate_count = duplicate_count.saturating_add(1);
                    continue;
                }
                match wishlist.add_item_with_settings(
                    artist,
                    title,
                    "Audio".to_owned(),
                    selected_filter.clone(),
                    true,
                    lidarr.auto_download,
                    usize::try_from(lidarr.wishlist_max_results).unwrap_or(MAX_WISHLIST_RESULTS),
                    None,
                ) {
                    Ok(item) => {
                        let item = wishlist
                            .update_lidarr_metadata(
                                &item.id,
                                lidarr_album_id,
                                None,
                                lidarr_track_count,
                                lidarr_duration_seconds,
                                lidarr_release_disambiguation,
                            )
                            .unwrap_or(item);
                        created.push(item);
                    }
                    Err(()) => {
                        skipped_count = skipped_count.saturating_add(1);
                        reached_cap = true;
                        break;
                    }
                }
            }
            (previous_wishlist, wishlist.clone())
        };
        let items_to_persist = created
            .iter()
            .chain(updated.iter())
            .cloned()
            .collect::<Vec<_>>();
        if !items_to_persist.is_empty() {
            if let Err(error) = persist_wishlist_items_checked(state, &items_to_persist).await {
                rollback_wishlist_if_unchanged(state, previous_wishlist, &mutated_wishlist).await;
                return Err(error);
            }
        }
        created_count = created_count.saturating_add(created.len() as u64);
        drop(_wishlist_search_persistence);

        if reached_cap || page.saturating_mul(PAGE_SIZE) >= wanted_count {
            break;
        }
        page = page.saturating_add(1);
    }

    let result = serde_json::json!({
        "enabled": true,
        "wantedCount": wanted_count,
        "createdCount": created_count,
        "duplicateCount": duplicate_count,
        "skippedCount": skipped_count,
    });
    {
        let mut sync = state.lidarr_sync_state.write().await;
        sync.last_sync_at = Some(chrono::Utc::now().to_rfc3339());
        sync.last_error = None;
        sync.last_result = Some(result.clone());
    }
    Ok(result)
}

pub(super) fn spawn_lidarr_sync_scheduler(state: Arc<AppState>) {
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        loop {
            let delay = run_lidarr_sync_scheduler_cycle(&state).await;
            time::sleep(delay).await;
        }
    });
}

pub(super) async fn run_lidarr_sync_scheduler_cycle(state: &AppState) -> Duration {
    let lidarr = state.integration_settings.read().await.lidarr.clone();
    let delay = Duration::from_secs(lidarr.sync_interval_seconds.max(300));
    if lidarr.enabled && lidarr.sync_wanted_to_wishlist {
        state.lidarr_sync_state.write().await.is_syncing = true;
        if let Err(error) = sync_lidarr_wanted_to_wishlist(state, &lidarr).await {
            state.lidarr_sync_state.write().await.last_error = Some(error);
        }
        state.lidarr_sync_state.write().await.is_syncing = false;
    }
    state.lidarr_sync_state.write().await.next_sync_at = Some(
        (chrono::Utc::now()
            + chrono::Duration::seconds(i64::try_from(delay.as_secs()).unwrap_or(i64::MAX)))
        .to_rfc3339(),
    );
    delay
}
