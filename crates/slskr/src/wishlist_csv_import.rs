use super::*;

pub(super) fn parse_wishlist_csv_rows(csv: &str) -> Result<Vec<Vec<String>>, &'static str> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut characters = csv.chars().peekable();
    let mut quoted = false;
    while let Some(character) = characters.next() {
        match character {
            '"' if quoted && characters.peek() == Some(&'"') => {
                field.push('"');
                characters.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => row.push(std::mem::take(&mut field)),
            '\r' | '\n' if !quoted => {
                if character == '\r' && characters.peek() == Some(&'\n') {
                    characters.next();
                }
                row.push(std::mem::take(&mut field));
                if row.iter().any(|value| !value.trim().is_empty()) {
                    if rows.len() >= MAX_CSV_IMPORT_ROWS {
                        return Err("CSV import exceeds 10000 rows");
                    }
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            character => field.push(character),
        }
    }
    row.push(field);
    if row.iter().any(|value| !value.trim().is_empty()) {
        if rows.len() >= MAX_CSV_IMPORT_ROWS {
            return Err("CSV import exceeds 10000 rows");
        }
        rows.push(row);
    }
    Ok(rows)
}

pub(super) fn parse_simple_wishlist_import_rows(
    raw: &str,
) -> Result<Vec<(String, String, String)>, &'static str> {
    let mut parsed_items = Vec::new();
    for (index, line) in raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .enumerate()
    {
        if index >= MAX_CSV_IMPORT_ROWS {
            return Err("CSV import exceeds 10000 rows");
        }
        let parts = line.split(',').map(str::trim).collect::<Vec<_>>();
        if index == 0
            && parts
                .first()
                .is_some_and(|value| value.eq_ignore_ascii_case("artist"))
        {
            continue;
        }
        let (artist, title) = if parts.len() >= 2 {
            (parts[0].to_owned(), parts[1].to_owned())
        } else if let Some((artist, title)) = line.split_once(" - ") {
            (artist.trim().to_owned(), title.trim().to_owned())
        } else {
            (String::new(), line.to_owned())
        };
        parsed_items.push((artist, title, "Audio".to_owned()));
    }
    Ok(parsed_items)
}

pub(super) fn normalized_wishlist_csv_header(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub(super) fn wishlist_csv_column(header: &[String], names: &[&str]) -> Option<usize> {
    header.iter().position(|value| {
        let normalized = normalized_wishlist_csv_header(value);
        names
            .iter()
            .any(|name| normalized.eq_ignore_ascii_case(name))
    })
}

pub(super) async fn versioned_wishlist_csv_import_response(
    body: &str,
    state: &AppState,
) -> HttpResponse {
    let request = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(serde_json::Value::Object(request)) => request,
        Ok(_) => return routing::bad_request_response("request must be an object"),
        Err(_) => return routing::bad_request_response("invalid JSON body"),
    };
    let csv = request
        .get("csvText")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if csv.trim().is_empty() {
        return routing::bad_request_response("CsvText is required");
    }
    let max_results = request
        .get("maxResults")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(100);
    if max_results <= 0 {
        return routing::bad_request_response("MaxResults must be greater than 0");
    }
    let max_results = usize::try_from(max_results)
        .unwrap_or(usize::MAX)
        .min(MAX_WISHLIST_RESULTS);
    let filter = request
        .get("filter")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    let enabled = request
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let auto_download = request
        .get("autoDownload")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let include_album = request
        .get("includeAlbum")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let rows = match parse_wishlist_csv_rows(csv) {
        Ok(rows) => rows,
        Err(error) => return routing::bad_request_response(error),
    };
    if rows.is_empty() {
        return routing::bad_request_response("CSV did not contain any track rows");
    }
    let header = &rows[0];
    let has_header = header.iter().any(|value| {
        matches!(
            normalized_wishlist_csv_header(value).as_str(),
            "trackname"
                | "track"
                | "title"
                | "songname"
                | "song"
                | "artistname"
                | "artist"
                | "artists"
                | "albumname"
                | "album"
        )
    });
    let (title_index, artist_index, album_index, start_index) = if has_header {
        (
            wishlist_csv_column(
                header,
                &["trackname", "track", "title", "songname", "song", "name"],
            ),
            wishlist_csv_column(header, &["artistname", "artistnames", "artists", "artist"]),
            wishlist_csv_column(header, &["albumname", "album", "release"]),
            1,
        )
    } else {
        (Some(0), Some(1), Some(2), 0)
    };
    let parsed = rows
        .iter()
        .enumerate()
        .skip(start_index)
        .map(|(index, row)| {
            let cell = |column: Option<usize>| {
                column
                    .and_then(|column| row.get(column))
                    .map(|value| value.trim())
                    .unwrap_or_default()
            };
            let mut parts = vec![cell(artist_index), cell(title_index)];
            if include_album {
                parts.push(cell(album_index));
            }
            let parts = parts
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>();
            (
                index + 1,
                if parts.len() >= 2 {
                    parts.join(" ")
                } else {
                    String::new()
                },
                row.join(","),
            )
        })
        .collect::<Vec<_>>();
    if parsed.is_empty() {
        return routing::bad_request_response("CSV did not contain any track rows");
    }

    let total_rows = parsed.len();
    let mut skipped_rows = Vec::new();
    let mut duplicate_count = 0usize;
    let mut created_items = Vec::new();
    let mut persisted_items = Vec::new();
    let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
    let mut wishlist = state.wishlist.write().await;
    let previous = wishlist.clone();
    let mut keys = wishlist
        .records
        .iter()
        .flat_map(|record| &record.items)
        .map(|item| format!("{}\u{1f}{}", item.search_text(), item.filter).to_ascii_lowercase())
        .collect::<HashSet<_>>();
    for (row_number, search_text, raw_text) in parsed {
        if search_text.is_empty() {
            skipped_rows.push(serde_json::json!({
                "rowNumber": row_number,
                "reason": "Missing artist or track title",
                "rawText": raw_text,
            }));
            continue;
        }
        let key = format!("{search_text}\u{1f}{filter}").to_ascii_lowercase();
        if !keys.insert(key) {
            duplicate_count += 1;
            continue;
        }
        let item = match wishlist.add_item_with_contract(
            Some(uuid::Uuid::new_v4().to_string()),
            search_text,
            String::new(),
            "Audio".to_owned(),
            filter.clone(),
            enabled,
            auto_download,
            max_results,
            None,
        ) {
            Ok(item) => item,
            Err(()) => {
                *wishlist = previous;
                return routing::service_unavailable_response("wishlist item capacity is full");
            }
        };
        created_items.push(serde_json::json!({
            "id": item.id,
            "searchText": item.search_text(),
            "filter": item.filter,
            "enabled": item.enabled,
            "autoDownload": item.auto_download,
            "maxResults": item.max_results,
            "createdAt": unix_seconds_rfc3339(item.added_at),
            "lastMatchCount": 0,
            "lastVisibleHitCount": 0,
            "lastHiddenLockedHitCount": 0,
            "lastFilteredOutHitCount": 0,
            "lastIgnoredResultHitCount": 0,
            "lastResponseCount": 0,
            "totalSearchCount": 0,
            "totalDownloadCount": 0,
        }));
        persisted_items.push(item);
    }
    let mutated = wishlist.clone();
    drop(wishlist);
    if let Err(error) = persist_wishlist_items_checked(state, &persisted_items).await {
        rollback_wishlist_if_unchanged(state, previous, &mutated).await;
        return routing::internal_server_error_response(&error);
    }
    routing::ok_response(
        serde_json::json!({
            "totalRows": total_rows,
            "createdCount": created_items.len(),
            "duplicateCount": duplicate_count,
            "skippedCount": skipped_rows.len(),
            "createdItems": created_items,
            "skippedRows": skipped_rows,
        })
        .to_string(),
    )
}
