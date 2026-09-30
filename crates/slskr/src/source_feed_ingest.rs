use super::*;

pub(super) const MAX_SOURCE_PROVIDER_RESPONSE_BYTES: usize = 512 * 1024;

pub(super) fn source_provider(source_text: &str, source_kind: &str) -> Option<&'static str> {
    let normalized = source_kind.replace('.', "").to_ascii_lowercase();
    if normalized != "auto" {
        return match normalized.as_str() {
            "youtube" => Some("youtube"),
            "lastfm" => Some("lastfm"),
            "apple" | "itunes" => Some("apple"),
            "listenbrainz" => Some("listenbrainz"),
            _ => None,
        };
    }
    let url = reqwest::Url::parse(source_text.trim()).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    if host == "youtube.com" || host.ends_with(".youtube.com") || host == "youtu.be" {
        Some("youtube")
    } else if host == "last.fm" || host.ends_with(".last.fm") {
        Some("lastfm")
    } else if host == "music.apple.com" || host == "itunes.apple.com" {
        Some("apple")
    } else if host == "bandcamp.com" || host.ends_with(".bandcamp.com") {
        Some("bandcamp")
    } else if host == "listenbrainz.org" || host.ends_with(".listenbrainz.org") {
        Some("listenbrainz")
    } else {
        None
    }
}

pub(super) fn loose_source_row(text: &str, source: &str, row_number: usize) -> SpotifySourceRow {
    let text = text.trim();
    let (artist, title) = text
        .split_once(" - ")
        .map(|(artist, title)| (artist.trim(), title.trim()))
        .unwrap_or(("", text));
    SpotifySourceRow {
        title: title.to_owned(),
        artist: artist.to_owned(),
        source: source.to_owned(),
        source_id: row_number.to_string(),
        raw_text: text.to_owned(),
        ..Default::default()
    }
}

pub(super) fn build_provider_source_result(
    provider: &str,
    source_text: &str,
    rows: Vec<SpotifySourceRow>,
    network_request_count: u64,
) -> serde_json::Value {
    let mut result = build_spotify_source_result(
        &SpotifySourceTarget {
            kind: "url".to_owned(),
            id: source_text.to_owned(),
            requires_user_token: false,
            scope_hint: String::new(),
        },
        rows,
        network_request_count,
    );
    result["provider"] = serde_json::json!(provider);
    result["sourceKind"] = serde_json::json!("url");
    result["sourceId"] = serde_json::json!(source_text);
    if let Some(suggestions) = result["suggestions"].as_array_mut() {
        for suggestion in suggestions {
            let source = suggestion["source"].as_str().unwrap_or_default();
            let normalized = suggestion["searchText"]
                .as_str()
                .unwrap_or_default()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase();
            suggestion["evidenceKey"] =
                serde_json::json!(format!("{provider}:{source}:{normalized}"));
            suggestion["reason"] = serde_json::json!(format!("Imported from {provider} url."));
        }
    }
    result
}

pub(super) async fn provider_get_json(
    timeout_seconds: u64,
    url: String,
    label: &str,
) -> Result<serde_json::Value, String> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("failed to build provider client: {error}"))?
        .get(url)
        .header("User-Agent", "slskr-source-feed-import/1.0")
        .send()
        .await
        .map_err(|error| format!("{label} request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("{label} returned HTTP {}", response.status()));
    }
    read_bounded_source_provider_json(response, label).await
}

pub(super) async fn read_bounded_source_provider_bytes(
    response: reqwest::Response,
    label: &str,
) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_SOURCE_PROVIDER_RESPONSE_BYTES as u64)
    {
        return Err(format!(
            "{label} response exceeds {MAX_SOURCE_PROVIDER_RESPONSE_BYTES} bytes"
        ));
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("{label} response read failed: {error}"))?;
        if body.len().saturating_add(chunk.len()) > MAX_SOURCE_PROVIDER_RESPONSE_BYTES {
            return Err(format!(
                "{label} response exceeds {MAX_SOURCE_PROVIDER_RESPONSE_BYTES} bytes"
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub(super) async fn read_bounded_source_provider_json(
    response: reqwest::Response,
    label: &str,
) -> Result<serde_json::Value, String> {
    let body = read_bounded_source_provider_bytes(response, label).await?;
    serde_json::from_slice(&body).map_err(|error| format!("invalid {label} JSON: {error}"))
}

pub(super) fn decode_markup_entities(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(ampersand) = remaining.find('&') {
        decoded.push_str(&remaining[..ampersand]);
        remaining = &remaining[ampersand..];
        let Some(semicolon) = remaining.find(';') else {
            decoded.push_str(remaining);
            return decoded.trim().to_owned();
        };
        let entity = &remaining[1..semicolon];
        let replacement = match entity {
            "amp" => Some('&'),
            "apos" => Some('\''),
            "gt" => Some('>'),
            "lt" => Some('<'),
            "quot" => Some('"'),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                u32::from_str_radix(&entity[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
            }
            _ if entity.starts_with('#') => {
                entity[1..].parse::<u32>().ok().and_then(char::from_u32)
            }
            _ => None,
        };
        if let Some(replacement) = replacement {
            decoded.push(replacement);
        } else {
            decoded.push_str(&remaining[..=semicolon]);
        }
        remaining = &remaining[semicolon + 1..];
    }
    decoded.push_str(remaining);
    decoded.trim().to_owned()
}

pub(super) fn markup_tag_attributes(tag: &str) -> Vec<(String, String)> {
    let bytes = tag.as_bytes();
    let mut index = 0_usize;
    while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    let mut attributes = Vec::new();
    while index < bytes.len() {
        while index < bytes.len()
            && (bytes[index].is_ascii_whitespace() || matches!(bytes[index], b'/' | b'>'))
        {
            index += 1;
        }
        let name_start = index;
        while index < bytes.len()
            && !bytes[index].is_ascii_whitespace()
            && !matches!(bytes[index], b'=' | b'/' | b'>')
        {
            index += 1;
        }
        if name_start == index {
            break;
        }
        let name = tag[name_start..index].to_owned();
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            attributes.push((name, String::new()));
            continue;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let quote = bytes
            .get(index)
            .copied()
            .filter(|byte| matches!(byte, b'\'' | b'"'));
        if quote.is_some() {
            index += 1;
        }
        let value_start = index;
        while index < bytes.len()
            && quote.map_or_else(
                || !bytes[index].is_ascii_whitespace() && !matches!(bytes[index], b'/' | b'>'),
                |quote| bytes[index] != quote,
            )
        {
            index += 1;
        }
        attributes.push((name, decode_markup_entities(&tag[value_start..index])));
        if quote.is_some() && index < bytes.len() {
            index += 1;
        }
    }
    attributes
}

pub(super) fn metadata_value(html: &str, property: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut offset = 0_usize;
    while let Some(relative) = lower[offset..].find("<meta") {
        let start = offset + relative;
        let Some(end_relative) = lower[start..].find('>') else {
            break;
        };
        let end = start + end_relative + 1;
        let attributes = markup_tag_attributes(&html[start + 1..end - 1]);
        let matches_property = attributes.iter().any(|(name, value)| {
            (name.eq_ignore_ascii_case("property") || name.eq_ignore_ascii_case("name"))
                && value.eq_ignore_ascii_case(property)
        });
        if matches_property {
            if let Some((_, value)) = attributes
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("content"))
            {
                return value.clone();
            }
        }
        offset = end;
    }
    String::new()
}

pub(super) fn metadata_title(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let Some(start) = lower.find("<title") else {
        return String::new();
    };
    let Some(open_end_relative) = lower[start..].find('>') else {
        return String::new();
    };
    let content_start = start + open_end_relative + 1;
    let Some(close_relative) = lower[content_start..].find("</title>") else {
        return String::new();
    };
    decode_markup_entities(
        &html[content_start..content_start + close_relative]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub(super) fn clean_provider_title(title: &str, provider: &str) -> String {
    let mut cleaned = title.trim().to_owned();
    let suffixes: &[&str] = match provider {
        "youtube" => &[" - YouTube"],
        "bandcamp" => &[" | Bandcamp"],
        "lastfm" => &[" | Last.fm", " — Last.fm"],
        "apple" => &[" by Apple Music", " on Apple Music"],
        _ => &[],
    };
    for suffix in suffixes {
        if cleaned
            .get(cleaned.len().saturating_sub(suffix.len())..)
            .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
        {
            cleaned.truncate(cleaned.len() - suffix.len());
            cleaned = cleaned.trim().to_owned();
        }
    }
    cleaned.trim_matches('"').to_owned()
}

pub(super) fn provider_metadata_row(
    provider: &str,
    source_text: &str,
    html: &str,
) -> Option<SpotifySourceRow> {
    let title = [
        metadata_value(html, "music:song"),
        metadata_value(html, "og:title"),
        metadata_value(html, "twitter:title"),
        metadata_title(html),
    ]
    .into_iter()
    .find(|value| !value.trim().is_empty())
    .unwrap_or_default();
    let title = clean_provider_title(&title, provider);
    if title.trim().is_empty() {
        return None;
    }
    let artist = [
        metadata_value(html, "music:musician:description"),
        metadata_value(html, "byl"),
        metadata_value(html, "article:author"),
    ]
    .into_iter()
    .find(|value| !value.trim().is_empty())
    .unwrap_or_default();
    let mut row = loose_source_row(&title, provider, 1);
    if !artist.trim().is_empty() {
        row.artist = artist;
    }
    row.provider_url = source_text.to_owned();
    Some(row)
}

pub(super) fn validate_provider_metadata_url(
    url: &str,
) -> Result<(reqwest::Url, ResolvedIntegrationTarget), String> {
    let parsed =
        reqwest::Url::parse(url).map_err(|error| format!("provider URL is invalid: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("provider URL scheme must be http or https".to_owned());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("provider URL must not contain embedded credentials".to_owned());
    }
    if parsed.fragment().is_some() {
        return Err("provider URL must not contain a fragment".to_owned());
    }
    if source_provider(url, "auto").is_none() {
        return Err("provider URL host is not supported".to_owned());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "provider URL must include a host".to_owned())?
        .to_owned();
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| "provider URL port is unknown".to_owned())?;
    let addrs = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|error| format!("provider URL resolution failed: {error}"))?
        .collect::<Vec<_>>();
    if addrs.is_empty() {
        return Err("provider URL did not resolve".to_owned());
    }
    if addrs
        .iter()
        .any(|address| is_blocked_integration_ip(address.ip()))
    {
        return Err("provider URL resolves to a private address".to_owned());
    }
    Ok((parsed, ResolvedIntegrationTarget { host, addrs }))
}

pub(super) async fn fetch_provider_metadata_page(
    provider: &str,
    source_text: &str,
    timeout_seconds: u64,
) -> Result<Option<SpotifySourceRow>, String> {
    let (url, resolved) = validate_provider_metadata_url(source_text)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .resolve_to_addrs(&resolved.host, &resolved.addrs)
        .build()
        .map_err(|error| format!("failed to build provider client: {error}"))?;
    let response = client
        .get(url)
        .header("User-Agent", "slskr-source-feed-import/1.0")
        .send()
        .await
        .map_err(|error| format!("provider metadata request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "provider metadata returned HTTP {}",
            response.status()
        ));
    }
    let body = read_bounded_source_provider_bytes(response, "provider metadata").await?;
    let html = String::from_utf8_lossy(&body);
    Ok(provider_metadata_row(provider, source_text, &html))
}

const MAX_SOURCE_PREVIEW_ROWS: usize = MAX_CSV_IMPORT_ROWS;

fn detect_local_source_kind(source_text: &str) -> &'static str {
    let trimmed = source_text.trim_start();
    if trimmed.to_ascii_lowercase().starts_with("#extm3u")
        || trimmed.to_ascii_lowercase().contains("#extinf")
    {
        "m3u"
    } else if trimmed.to_ascii_lowercase().starts_with("<rss")
        || trimmed.to_ascii_lowercase().starts_with("<feed")
        || trimmed.to_ascii_lowercase().starts_with("<opml")
    {
        "rss"
    } else {
        let first_line = source_text
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or_default();
        if first_line.contains(',') && first_line.to_ascii_lowercase().contains("track") {
            "csv"
        } else {
            "text"
        }
    }
}

fn local_csv_rows(
    source_text: &str,
    include_album: bool,
) -> Result<Vec<SpotifySourceRow>, &'static str> {
    let rows = parse_wishlist_csv_rows(source_text)?;
    let Some(first) = rows.first() else {
        return Ok(Vec::new());
    };
    let header_names = first
        .iter()
        .map(|value| normalized_wishlist_csv_header(value))
        .collect::<Vec<_>>();
    let has_header = header_names.iter().any(|value| {
        matches!(
            value.as_str(),
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
    let title_index = has_header
        .then(|| {
            wishlist_csv_column(
                first,
                &["trackname", "track", "title", "songname", "song", "name"],
            )
        })
        .flatten()
        .unwrap_or(0);
    let artist_index = has_header
        .then(|| wishlist_csv_column(first, &["artistname", "artistnames", "artists", "artist"]))
        .flatten()
        .unwrap_or(1);
    let album_index = has_header
        .then(|| wishlist_csv_column(first, &["albumname", "album", "release"]))
        .flatten()
        .unwrap_or(2);
    let url_index = has_header
        .then(|| wishlist_csv_column(first, &["url", "spotifyurl", "trackurl", "link"]))
        .flatten();
    Ok(rows
        .iter()
        .enumerate()
        .skip(usize::from(has_header))
        .map(|(index, row)| {
            let cell = |column: usize| {
                row.get(column)
                    .map(|value| value.trim())
                    .unwrap_or_default()
            };
            let title = cell(title_index).to_owned();
            SpotifySourceRow {
                title: title.clone(),
                artist: cell(artist_index).to_owned(),
                album: if include_album {
                    cell(album_index).to_owned()
                } else {
                    String::new()
                },
                source: "csv".to_owned(),
                source_id: (index + 1).to_string(),
                provider_url: url_index
                    .and_then(|column| row.get(column))
                    .map(|value| value.trim().to_owned())
                    .unwrap_or_default(),
                raw_text: row.join(","),
            }
        })
        .collect())
}

fn local_playlist_rows(source_text: &str) -> Result<Vec<SpotifySourceRow>, &'static str> {
    let mut rows = Vec::new();
    for (index, line) in source_text.lines().enumerate() {
        let line = line.trim();
        if !line
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("#EXTINF:"))
        {
            continue;
        }
        if rows.len() >= MAX_SOURCE_PREVIEW_ROWS {
            return Err("Source preview exceeds 10000 rows");
        }
        let Some(title) = line.rsplit_once(',').map(|(_, title)| title) else {
            continue;
        };
        rows.push(loose_source_row(title, "m3u", index + 1));
    }
    Ok(rows)
}

fn local_xml_values(source_text: &str, element: &str) -> Vec<String> {
    let lower = String::from_utf8(
        source_text
            .as_bytes()
            .iter()
            .map(u8::to_ascii_lowercase)
            .collect(),
    )
    .expect("ASCII case conversion preserves UTF-8");
    let mut values = Vec::new();
    let mut offset = 0_usize;
    while let Some(relative) = lower[offset..].find('<') {
        let start = offset + relative;
        let Some(tag_end_relative) = lower[start..].find('>') else {
            break;
        };
        let tag_end = start + tag_end_relative;
        let tag = source_text[start + 1..tag_end].trim();
        if tag.starts_with('/') || tag.starts_with('!') || tag.starts_with('?') {
            offset = tag_end + 1;
            continue;
        }
        let tag_name = tag
            .split_ascii_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches('/');
        if !tag_name
            .rsplit(':')
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case(element))
        {
            offset = tag_end + 1;
            continue;
        }
        let close = format!("</{}>", tag_name.to_ascii_lowercase());
        let content_start = start + tag_end_relative + 1;
        let Some(close_relative) = lower[content_start..].find(&close) else {
            offset = tag_end + 1;
            continue;
        };
        let content_end = content_start + close_relative;
        values.push(decode_markup_entities(
            source_text[content_start..content_end].trim(),
        ));
        offset = content_end + close.len();
    }
    values
}

fn local_xml_rows(source_text: &str) -> Result<Vec<SpotifySourceRow>, &'static str> {
    let mut rows = Vec::new();
    let lower = source_text.to_ascii_lowercase();
    let mut offset = 0_usize;
    while let Some(relative) = lower[offset..].find('<') {
        let start = offset + relative;
        let Some(tag_end_relative) = lower[start..].find('>') else {
            break;
        };
        let tag_end = start + tag_end_relative;
        let tag = source_text[start + 1..tag_end].trim();
        let tag_name = tag
            .split_ascii_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches('/');
        let local_name = tag_name.rsplit(':').next().unwrap_or_default();
        if !tag.starts_with('/')
            && (local_name.eq_ignore_ascii_case("item") || local_name.eq_ignore_ascii_case("entry"))
        {
            let close = format!("</{}>", tag_name.to_ascii_lowercase());
            let content_start = tag_end + 1;
            if let Some(close_relative) = lower[content_start..].find(&close) {
                let content_end = content_start + close_relative;
                let container = &source_text[content_start..content_end];
                let title = local_xml_values(container, "title")
                    .into_iter()
                    .next()
                    .unwrap_or_default();
                if rows.len() >= MAX_SOURCE_PREVIEW_ROWS {
                    return Err("Source preview exceeds 10000 rows");
                }
                rows.push(loose_source_row(&title, "rss", rows.len() + 1));
                offset = content_end + close.len();
                continue;
            }
        }
        offset = tag_end + 1;
    }

    let mut outline_index = 0_usize;
    let mut offset = 0_usize;
    while let Some(relative) = lower[offset..].find('<') {
        let start = offset + relative;
        let Some(tag_end_relative) = lower[start..].find('>') else {
            break;
        };
        let tag_end = start + tag_end_relative;
        let tag = source_text[start + 1..tag_end].trim();
        let tag_name = tag
            .split_ascii_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches('/');
        if !tag.starts_with('/')
            && tag_name
                .rsplit(':')
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case("outline"))
        {
            outline_index += 1;
            let attributes = markup_tag_attributes(tag);
            let text = attributes
                .iter()
                .find(|(name, _)| name == "text")
                .or_else(|| attributes.iter().find(|(name, _)| name == "title"))
                .map(|(_, value)| value.as_str())
                .unwrap_or_default();
            if rows.len() >= MAX_SOURCE_PREVIEW_ROWS {
                return Err("Source preview exceeds 10000 rows");
            }
            rows.push(loose_source_row(text, "opml", outline_index));
        }
        offset = tag_end + 1;
    }
    Ok(rows)
}

pub(super) fn parse_simple_source_preview_items(
    raw: &str,
) -> Result<Vec<serde_json::Value>, &'static str> {
    let mut items = Vec::new();
    for (index, line) in raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .enumerate()
    {
        if index >= MAX_SOURCE_PREVIEW_ROWS {
            return Err("Source preview exceeds 10000 rows");
        }
        let (artist, title) = line
            .split_once(" - ")
            .map(|(artist, title)| (artist.trim(), title.trim()))
            .unwrap_or(("", line));
        items.push(serde_json::json!({
            "id": format!("preview-{}", index + 1),
            "artist": artist,
            "title": title,
            "searchText": line,
            "valid": !line.is_empty(),
        }));
    }
    Ok(items)
}

pub(super) fn preview_local_source_feed(
    source_text: &str,
    requested_kind: &str,
    include_album: bool,
    limit: usize,
) -> Result<serde_json::Value, &'static str> {
    let kind = if requested_kind == "auto" {
        detect_local_source_kind(source_text)
    } else {
        requested_kind
    };
    let rows = match kind {
        "csv" => local_csv_rows(source_text, include_album)?,
        "m3u" | "pls" => local_playlist_rows(source_text)?,
        "rss" | "opml" => local_xml_rows(source_text)?,
        _ => {
            let mut rows = Vec::new();
            for (index, line) in source_text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .enumerate()
            {
                if index >= MAX_SOURCE_PREVIEW_ROWS {
                    return Err("Source preview exceeds 10000 rows");
                }
                rows.push(loose_source_row(line, "text", index + 1));
            }
            rows
        }
    };
    let mut result =
        build_provider_source_result("local", "", rows.into_iter().take(limit).collect(), 0);
    result["sourceKind"] = serde_json::json!(kind);
    result["sourceId"] = serde_json::json!("");
    if let Some(suggestions) = result["suggestions"].as_array_mut() {
        for suggestion in suggestions {
            suggestion["reason"] = serde_json::json!(format!("Imported from local {kind}."));
        }
    }
    Ok(result)
}
