use super::*;

const MAX_SUPPORTED_UNIX_MILLIS: u64 = 253_402_300_799_999;

pub(super) fn url_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

pub(super) fn query_parameter(query: Option<&str>, name: &str) -> Option<String> {
    query?.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        percent_decode_component(key)
            .eq_ignore_ascii_case(name)
            .then(|| percent_decode_component(value))
    })
}

pub(super) fn query_millis_parameter(
    query: Option<&str>,
    name: &str,
) -> Result<Option<u64>, String> {
    let Some(raw) = query_parameter(query, name) else {
        return Ok(None);
    };
    let parsed = raw
        .parse::<i128>()
        .map_err(|_| format!("{name} is outside the supported Unix timestamp range"))?;
    if parsed < 0 {
        return Err(format!(
            "{name} must be a non-negative Unix timestamp in milliseconds"
        ));
    }
    let parsed = u64::try_from(parsed)
        .ok()
        .filter(|value| *value <= MAX_SUPPORTED_UNIX_MILLIS)
        .ok_or_else(|| format!("{name} is outside the supported Unix timestamp range"))?;
    Ok(Some(parsed))
}

pub(super) fn query_bounded_usize(
    query: Option<&str>,
    name: &str,
    minimum: usize,
    maximum: usize,
) -> Result<Option<usize>, ()> {
    let Some(raw) = query_parameter(query, name) else {
        return Ok(None);
    };
    let value = raw.parse::<i128>().map_err(|_| ())?;
    let value = usize::try_from(value).map_err(|_| ())?;
    if !(minimum..=maximum).contains(&value) {
        return Err(());
    }
    Ok(Some(value))
}

pub(super) fn extract_json_u32_field(body: &str, field: &str) -> Option<u32> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get(field)?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
}

pub(super) fn extract_json_bool_field(body: &str, field: &str) -> Option<bool> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get(field)?
        .as_bool()
}

pub(super) fn extract_json_u64_field(body: &str, field: &str) -> Option<u64> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get(field)?
        .as_u64()
}

pub(super) fn extract_json_optional_u64_field(body: &str, field: &str) -> Option<Option<u64>> {
    let value = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let value = value.get(field)?;
    if value.is_null() {
        Some(None)
    } else {
        value.as_u64().map(Some)
    }
}

pub(super) fn extract_json_i32_field(body: &str, field: &str) -> Option<i32> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get(field)?
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
}

pub(super) fn json_body_string(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
}

pub(super) fn path_segment_after<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    path.strip_prefix(prefix)
        .filter(|segment| !segment.is_empty() && !segment.contains('/'))
}

pub(super) fn path_segment_between<'a>(
    path: &'a str,
    prefix: &str,
    suffix: &str,
) -> Option<&'a str> {
    let segment = path.strip_prefix(prefix)?.strip_suffix(suffix)?;
    (!segment.is_empty() && !segment.contains('/')).then_some(segment)
}

pub(super) fn json_array_exceeds_limit(value: &serde_json::Value, limit: usize) -> bool {
    value.as_array().is_some_and(|items| items.len() > limit)
}

pub(super) fn json_array_field_exceeds_limit(body: &str, field: &str, limit: usize) -> bool {
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    payload
        .get(field)
        .is_some_and(|value| json_array_exceeds_limit(value, limit))
}

pub(super) fn conversation_batch_exceeds_wire_limits(body: &str) -> bool {
    ["usernames", "recipients"]
        .into_iter()
        .any(|field| json_array_field_exceeds_limit(body, field, MAX_PRIVATE_MESSAGE_RECIPIENTS))
}

pub(super) fn wishlist_bulk_filter_exceeds_wire_limits(body: &str) -> bool {
    ["ids", "itemIds"]
        .into_iter()
        .any(|field| json_array_field_exceeds_limit(body, field, MAX_WISHLIST_ITEMS))
}

pub(super) fn browse_response_exceeds_wire_limits(payload: &serde_json::Value) -> bool {
    let mut file_count = payload
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    if file_count > MAX_BROWSE_WIRE_FILES_PER_RESPONSE {
        return true;
    }

    let Some(directories) = payload
        .get("directories")
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };
    if directories.len() > MAX_BROWSE_WIRE_FOLDERS_PER_RESPONSE {
        return true;
    }

    for directory in directories {
        let Some(files) = directory.get("files").and_then(serde_json::Value::as_array) else {
            continue;
        };
        if files.len() > MAX_BROWSE_WIRE_FILES_PER_RESPONSE.saturating_sub(file_count) {
            return true;
        }
        file_count += files.len();
    }
    false
}

pub(super) fn search_response_exceeds_wire_limits(payload: &serde_json::Value) -> bool {
    let file_count = payload
        .get("files")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let locked_file_count = payload
        .get("lockedFiles")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    file_count > MAX_SEARCH_RESULTS_PER_SEARCH
        || locked_file_count > MAX_SEARCH_RESULTS_PER_SEARCH.saturating_sub(file_count)
}

pub(super) fn collection_reorder_exceeds_wire_limits(body: &str) -> bool {
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    ["item_ids", "itemIds", "items"]
        .into_iter()
        .find_map(|field| payload.get(field))
        .is_some_and(|value| json_array_exceeds_limit(value, MAX_COLLECTION_ITEMS))
}
