use super::{
    bounded_user_username, json_escape, routing, truncate_utf8_bytes, unix_timestamp, AppState,
    HttpResponse, MAX_NOW_PLAYING_ARTIST_BYTES, MAX_NOW_PLAYING_RECORDS,
    MAX_NOW_PLAYING_TITLE_BYTES,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NowPlayingRecord {
    pub(crate) username: String,
    pub(crate) artist: String,
    pub(crate) title: String,
    pub(crate) updated_at: u64,
}

impl NowPlayingRecord {
    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"username\":\"{}\",\"artist\":\"{}\",\"title\":\"{}\",\"updated_at\":{}}}",
            json_escape(&self.username),
            json_escape(&self.artist),
            json_escape(&self.title),
            self.updated_at
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NowPlayingStore {
    pub(crate) records: Vec<NowPlayingRecord>,
    pub(crate) updated_at: u64,
}

impl NowPlayingStore {
    pub(crate) fn new() -> Self {
        Self {
            records: Vec::new(),
            updated_at: unix_timestamp(),
        }
    }

    pub(crate) fn from_persisted(records: Vec<crate::persistence::NowPlayingRecord>) -> Self {
        let mut updated_at = unix_timestamp();
        let mut records = records
            .into_iter()
            .map(|record| {
                let record_updated_at = u64::try_from(record.updated_at).unwrap_or(0);
                updated_at = updated_at.max(record_updated_at);
                NowPlayingRecord {
                    username: bounded_user_username(&record.username),
                    artist: truncate_utf8_bytes(record.artist, MAX_NOW_PLAYING_ARTIST_BYTES),
                    title: truncate_utf8_bytes(record.title, MAX_NOW_PLAYING_TITLE_BYTES),
                    updated_at: record_updated_at,
                }
            })
            .collect::<Vec<_>>();
        records.sort_by_key(|record| std::cmp::Reverse(record.updated_at));
        let mut seen_users = std::collections::HashSet::new();
        records.retain(|record| seen_users.insert(record.username.to_ascii_lowercase()));
        records.truncate(MAX_NOW_PLAYING_RECORDS);
        records.sort_by_key(|record| record.updated_at);
        Self {
            records,
            updated_at,
        }
    }

    pub(crate) fn upsert(
        &mut self,
        username: String,
        artist: String,
        title: String,
    ) -> NowPlayingRecord {
        let now = unix_timestamp();
        let username = if username.trim().is_empty() {
            "local".to_owned()
        } else {
            bounded_user_username(&username)
        };
        let artist = truncate_utf8_bytes(artist, MAX_NOW_PLAYING_ARTIST_BYTES);
        let title = truncate_utf8_bytes(title, MAX_NOW_PLAYING_TITLE_BYTES);
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username.eq_ignore_ascii_case(&username))
        {
            record.artist = artist;
            record.title = title;
            record.updated_at = now;
            self.updated_at = now;
            return record.clone();
        }
        let record = NowPlayingRecord {
            username,
            artist,
            title,
            updated_at: now,
        };
        if self.records.len() == MAX_NOW_PLAYING_RECORDS {
            let oldest = self
                .records
                .iter()
                .enumerate()
                .min_by_key(|(_, record)| record.updated_at)
                .map(|(index, _)| index)
                .unwrap_or(0);
            self.records.remove(oldest);
        }
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    pub(crate) fn clear(&mut self) -> usize {
        let cleared = self.records.len();
        self.records.clear();
        self.updated_at = unix_timestamp();
        cleared
    }

    pub(crate) fn json(&self) -> String {
        let records = self
            .records
            .iter()
            .map(NowPlayingRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"now_playing\":[{}],\"count\":{},\"updated_at\":{}}}",
            records,
            self.records.len(),
            self.updated_at
        )
    }
}

pub(super) async fn persist_now_playing_checked(
    state: &AppState,
    record: &NowPlayingRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = crate::persistence::NowPlayingRecord {
        username: record.username.clone(),
        artist: record.artist.clone(),
        title: record.title.clone(),
        updated_at: i64::try_from(record.updated_at).unwrap_or(i64::MAX),
    };
    db.upsert_now_playing(&persisted)
        .await
        .map_err(|error| format!("now-playing persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_now_playing_clear_checked(state: &AppState) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.clear_now_playing()
        .await
        .map_err(|error| format!("now-playing clear persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn native_nowplaying_set_track(state: &AppState, artist: String, title: String) {
    if artist.is_empty() || title.is_empty() {
        return;
    }
    state
        .now_playing
        .write()
        .await
        .upsert("local".to_owned(), artist, title);
}

pub(super) async fn native_nowplaying_clear_track(state: &AppState) {
    state.now_playing.write().await.clear();
}

pub(super) async fn native_nowplaying_webhook_response(
    body: &str,
    state: &AppState,
) -> HttpResponse {
    if body.trim().is_empty() {
        return routing::bad_request_response("Empty payload");
    }
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(payload) => payload,
        Err(_) => return routing::bad_request_response("Invalid JSON payload"),
    };
    let Some(object) = payload.as_object() else {
        return routing::ok_response(String::new());
    };
    let string_field = |object: &serde_json::Map<String, serde_json::Value>, name: &str| {
        object
            .get(name)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    };

    // Plex sends an event plus a nested Metadata object.
    if let (Some(event), Some(metadata)) = (
        string_field(object, "event"),
        object
            .get("Metadata")
            .and_then(serde_json::Value::as_object),
    ) {
        match event.as_str() {
            "media.play" | "media.resume" | "media.scrobble" => {
                let artist = string_field(metadata, "grandparentTitle")
                    .or_else(|| string_field(metadata, "originalTitle"))
                    .unwrap_or_default();
                let title = string_field(metadata, "title").unwrap_or_default();
                native_nowplaying_set_track(state, artist, title).await;
            }
            "media.pause" | "media.stop" => native_nowplaying_clear_track(state).await,
            _ => {}
        }
        return routing::ok_response(String::new());
    }

    // Jellyfin/Emby sends a flat notification object.
    if let Some(notification_type) = string_field(object, "NotificationType") {
        match notification_type.as_str() {
            "PlaybackStart" | "PlaybackProgress" => {
                let artist = string_field(object, "Artist").unwrap_or_default();
                let title = string_field(object, "Name").unwrap_or_default();
                native_nowplaying_set_track(state, artist, title).await;
            }
            "PlaybackStop" => native_nowplaying_clear_track(state).await,
            _ => {}
        }
        return routing::ok_response(String::new());
    }

    // Generic fallback: { artist, title, album, event }.
    let event = string_field(object, "event").unwrap_or_else(|| "play".to_owned());
    if matches!(event.as_str(), "stop" | "pause") {
        native_nowplaying_clear_track(state).await;
    } else {
        let artist = string_field(object, "artist").unwrap_or_default();
        let title = string_field(object, "title").unwrap_or_default();
        native_nowplaying_set_track(state, artist, title).await;
    }
    routing::ok_response(String::new())
}
