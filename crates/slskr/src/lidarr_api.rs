use super::*;

pub(super) async fn fetch_lidarr_system_status(
    lidarr: &config::LidarrIntegrationSettings,
) -> Result<serde_json::Value, String> {
    let base_url = lidarr
        .url
        .as_deref()
        .ok_or("Lidarr URL is not configured")?;
    let resolved = validate_lidarr_base_url(base_url)?;
    let api_key = lidarr
        .api_key
        .as_deref()
        .ok_or("Lidarr API key is not configured")?;
    let url = format!("{}/api/v1/system/status", base_url.trim_end_matches('/'));
    let mut client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy();
    for addr in &resolved.addrs {
        client_builder = client_builder.resolve(&resolved.host, *addr);
    }
    let client = client_builder
        .build()
        .map_err(|error| format!("failed to build Lidarr client: {error}"))?;
    let response = client
        .get(url)
        .header("X-Api-Key", api_key)
        .send()
        .await
        .map_err(|error| format!("Lidarr status request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Lidarr returned HTTP {}", response.status()));
    }
    read_bounded_integration_json(response, "Lidarr status").await
}

pub(super) async fn fetch_lidarr_wanted_missing(
    lidarr: &config::LidarrIntegrationSettings,
    page: u64,
    page_size: u64,
) -> Result<serde_json::Value, String> {
    let base_url = lidarr
        .url
        .as_deref()
        .ok_or("Lidarr URL is not configured")?;
    let resolved = validate_lidarr_base_url(base_url)?;
    let api_key = lidarr
        .api_key
        .as_deref()
        .ok_or("Lidarr API key is not configured")?;
    let url = format!(
        "{}/api/v1/wanted/missing?page={}&pageSize={}&includeArtist=true&monitored=true",
        base_url.trim_end_matches('/'),
        page.max(1),
        page_size.clamp(1, 250),
    );
    let mut client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy();
    for addr in &resolved.addrs {
        client_builder = client_builder.resolve(&resolved.host, *addr);
    }
    let client = client_builder
        .build()
        .map_err(|error| format!("failed to build Lidarr client: {error}"))?;
    let response = client
        .get(url)
        .header("X-Api-Key", api_key)
        .send()
        .await
        .map_err(|error| format!("Lidarr wanted request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Lidarr returned HTTP {}", response.status()));
    }
    read_bounded_integration_json(response, "Lidarr wanted").await
}

/// Convert Lidarr's allowed quality tree into the same literal filter syntax
/// consumed by wishlist candidate selection. `None` means the profile is
/// intentionally unrestricted; `Some("")` means it contained no recognized
/// quality and therefore must not replace the operator's configured filter.
pub(super) fn lidarr_quality_profile_filter(profile: &serde_json::Value) -> Option<String> {
    fn quality_name(item: &serde_json::Value) -> String {
        item.pointer("/quality/name")
            .or_else(|| item.get("name"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
    }
    fn visit(
        item: &serde_json::Value,
        filters: &mut Vec<String>,
        seen: &mut HashSet<String>,
    ) -> bool {
        let name = quality_name(item);
        let allowed = item
            .get("allowed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if allowed && (name.contains("any") || name.contains("all")) {
            return true;
        }
        if allowed {
            let mut formats = Vec::new();
            let mut add = |format: &str| {
                if !formats.iter().any(|value: &String| value == format) {
                    formats.push(format.to_owned());
                }
            };
            if name.contains("lossless") {
                for format in ["flac", "alac", "m4a", "wav", "ape", "aiff", "aif"] {
                    add(format);
                }
            }
            if name.contains("lossy") {
                for format in ["mp3", "aac", "m4a", "ogg", "oga", "opus"] {
                    add(format);
                }
            }
            for (needle, format) in [
                ("mp3", "mp3"),
                ("flac", "flac"),
                ("alac", "alac"),
                ("aac", "aac"),
                ("vorbis", "ogg"),
                ("ogg", "ogg"),
                ("opus", "opus"),
                ("ape", "ape"),
                ("wav", "wav"),
                ("aiff", "aiff"),
                ("aif", "aif"),
                ("m4a", "m4a"),
            ] {
                if name.contains(needle) {
                    add(format);
                }
            }
            let bitrate = name
                .split(|character: char| !character.is_ascii_digit())
                .filter_map(|part| part.parse::<u32>().ok())
                .find(|value| (32..=1_000).contains(value));
            for format in formats {
                let filter = bitrate
                    .map(|value| format!("{format} minbr:{value}"))
                    .unwrap_or(format);
                if seen.insert(filter.clone()) {
                    filters.push(filter);
                }
            }
        }
        item.get("items")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| items.iter().any(|child| visit(child, filters, seen)))
    }

    let mut filters = Vec::new();
    let mut seen = HashSet::new();
    let unrestricted = profile
        .get("items")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| visit(item, &mut filters, &mut seen))
        });
    if unrestricted {
        None
    } else {
        Some(filters.join(" OR "))
    }
}

pub(super) async fn fetch_lidarr_json_get(
    lidarr: &config::LidarrIntegrationSettings,
    path: &str,
    label: &str,
) -> Result<serde_json::Value, String> {
    let base_url = lidarr
        .url
        .as_deref()
        .ok_or("Lidarr URL is not configured")?;
    let resolved = validate_lidarr_base_url(base_url)?;
    let api_key = lidarr
        .api_key
        .as_deref()
        .ok_or("Lidarr API key is not configured")?;
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy();
    for addr in &resolved.addrs {
        builder = builder.resolve(&resolved.host, *addr);
    }
    let response = builder
        .build()
        .map_err(|error| format!("failed to build Lidarr client: {error}"))?
        .get(url)
        .header("X-Api-Key", api_key)
        .send()
        .await
        .map_err(|error| format!("Lidarr {label} request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Lidarr returned HTTP {}", response.status()));
    }
    read_bounded_integration_json(response, &format!("Lidarr {label}")).await
}

pub(super) async fn fetch_lidarr_manual_import_candidates(
    lidarr: &config::LidarrIntegrationSettings,
    directory: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let base_url = lidarr
        .url
        .as_deref()
        .ok_or("Lidarr URL is not configured")?;
    let resolved = validate_lidarr_base_url(base_url)?;
    let api_key = lidarr
        .api_key
        .as_deref()
        .ok_or("Lidarr API key is not configured")?;
    let url = format!(
        "{}/api/v1/manualimport?folder={}&filterExistingFiles=false&replaceExistingFiles={}",
        base_url.trim_end_matches('/'),
        url_encode(directory),
        lidarr.import_replace_existing_files,
    );
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy();
    for addr in &resolved.addrs {
        builder = builder.resolve(&resolved.host, *addr);
    }
    let response = builder
        .build()
        .map_err(|error| format!("failed to build Lidarr client: {error}"))?
        .get(url)
        .header("X-Api-Key", api_key)
        .send()
        .await
        .map_err(|error| format!("Lidarr manual import request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Lidarr returned HTTP {}", response.status()));
    }
    read_bounded_integration_json(response, "Lidarr manual import")
        .await?
        .as_array()
        .cloned()
        .ok_or_else(|| "Lidarr manual import response was not an array".to_owned())
}

pub(super) async fn start_lidarr_manual_import(
    lidarr: &config::LidarrIntegrationSettings,
    files: Vec<serde_json::Value>,
    import_mode: &str,
) -> Result<i64, String> {
    let base_url = lidarr
        .url
        .as_deref()
        .ok_or("Lidarr URL is not configured")?;
    let resolved = validate_lidarr_base_url(base_url)?;
    let api_key = lidarr
        .api_key
        .as_deref()
        .ok_or("Lidarr API key is not configured")?;
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy();
    for addr in &resolved.addrs {
        builder = builder.resolve(&resolved.host, *addr);
    }
    let response = builder
        .build()
        .map_err(|error| format!("failed to build Lidarr client: {error}"))?
        .post(format!("{}/api/v1/command", base_url.trim_end_matches('/')))
        .header("X-Api-Key", api_key)
        .json(&serde_json::json!({
            "name": "ManualImport",
            "sendUpdatesToClient": false,
            "files": files,
            "importMode": import_mode,
            "replaceExistingFiles": lidarr.import_replace_existing_files,
        }))
        .send()
        .await
        .map_err(|error| format!("Lidarr command request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Lidarr returned HTTP {}", response.status()));
    }
    let value = read_bounded_integration_json(response, "Lidarr command").await?;
    value
        .as_i64()
        .or_else(|| value.get("id").and_then(serde_json::Value::as_i64))
        .ok_or_else(|| "Lidarr command response did not include an id".to_owned())
}
