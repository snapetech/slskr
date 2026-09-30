//! Grant-scoped admission held until a ticketed response finishes or is canceled.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

pub(super) const MAX_EXPLICIT_STREAM_LIMIT: u32 = 64;

#[derive(Debug, Default)]
struct Counts {
    closed: bool,
    active: BTreeMap<String, u32>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ShareStreamLimits {
    counts: Arc<Mutex<Counts>>,
}

#[derive(Debug)]
pub(super) struct StreamLease {
    counts: Arc<Mutex<Counts>>,
    grant_id: String,
}

impl ShareStreamLimits {
    pub(super) fn try_acquire(&self, grant_id: &str, limit: Option<u32>) -> Option<StreamLease> {
        if grant_id.is_empty()
            || grant_id.len() > 256
            || limit.is_some_and(|limit| !(1..=MAX_EXPLICIT_STREAM_LIMIT).contains(&limit))
        {
            return None;
        }
        let mut counts = self
            .counts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if counts.closed
            || (!counts.active.contains_key(grant_id)
                && counts.active.len() >= crate::MAX_SHARE_GRANTS)
        {
            return None;
        }
        let active = counts.active.entry(grant_id.to_owned()).or_default();
        if limit.is_some_and(|limit| *active >= limit) {
            return None;
        }
        *active = active.checked_add(1)?;
        Some(StreamLease {
            counts: Arc::clone(&self.counts),
            grant_id: grant_id.to_owned(),
        })
    }

    pub(super) fn close(&self) {
        self.counts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .closed = true;
    }
}

impl Drop for StreamLease {
    fn drop(&mut self) {
        let mut counts = self
            .counts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(active) = counts.active.get_mut(&self.grant_id) {
            *active = active.saturating_sub(1);
            if *active == 0 {
                counts.active.remove(&self.grant_id);
            }
        }
    }
}

/// Outer option means field presence; null clears an explicit limit.
pub(super) fn request_limit(body: &str) -> Result<Option<Option<u32>>, &'static str> {
    let Ok(request) = serde_json::from_str::<serde_json::Value>(body) else {
        return Ok(None);
    };
    let Some(value) = request.get("maxConcurrentStreams") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(Some(None));
    }
    let limit = value
        .as_u64()
        .and_then(|limit| u32::try_from(limit).ok())
        .filter(|limit| (1..=MAX_EXPLICIT_STREAM_LIMIT).contains(limit))
        .ok_or("maxConcurrentStreams must be null or an integer from 1 through 64")?;
    Ok(Some(Some(limit)))
}

#[derive(Debug)]
pub(super) enum AdmissionError {
    Unauthorized,
    Busy,
}

pub(super) async fn acquire_ticket_stream(
    state: &crate::AppState,
    stream_id: &str,
    query: Option<&str>,
) -> Result<Option<StreamLease>, AdmissionError> {
    let Some(token) = crate::query_parameter(query, "ticket") else {
        return Ok(None);
    };
    let ticket = state
        .stream_tickets
        .write()
        .await
        .get(&token)
        .filter(|ticket| ticket.family == "share" && ticket.content_id == stream_id)
        .ok_or(AdmissionError::Unauthorized)?;
    let grant_id = ticket
        .source
        .strip_prefix("share:")
        .ok_or(AdmissionError::Unauthorized)?;
    let grants = state.share_grants.read().await;
    let grant = grants
        .get(grant_id)
        .filter(|grant| crate::share_grant_allows_stream(&grant.permissions))
        .ok_or(AdmissionError::Unauthorized)?;
    state
        .share_stream_limits
        .try_acquire(grant_id, grant.max_concurrent_streams)
        .map(Some)
        .ok_or(AdmissionError::Busy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leases_isolate_grants_release_capacity_and_respect_lowered_limits() {
        let limits = ShareStreamLimits::default();
        let first = limits.try_acquire("first", Some(2)).unwrap();
        let second = limits.try_acquire("first", Some(2)).unwrap();
        assert!(limits.try_acquire("first", Some(2)).is_none());
        assert!(limits.try_acquire("first", Some(1)).is_none());
        let other = limits.try_acquire("other", Some(1)).unwrap();
        drop(first);
        assert!(limits.try_acquire("first", Some(1)).is_none());
        drop(second);
        assert!(limits.try_acquire("first", Some(1)).is_some());
        drop(other);
        assert!(limits.counts.lock().unwrap().active.is_empty());
        limits.close();
        assert!(limits.try_acquire("first", Some(1)).is_none());
    }

    #[test]
    fn explicit_limits_reject_invalid_values_and_accept_null_reset() {
        assert_eq!(request_limit("{}"), Ok(None));
        assert_eq!(
            request_limit(r#"{"maxConcurrentStreams":null}"#),
            Ok(Some(None))
        );
        assert_eq!(
            request_limit(r#"{"maxConcurrentStreams":1}"#),
            Ok(Some(Some(1)))
        );
        assert_eq!(
            request_limit(r#"{"maxConcurrentStreams":64}"#),
            Ok(Some(Some(64)))
        );
        for value in [
            "0",
            "65",
            "-1",
            "1.5",
            "true",
            "\"1\"",
            "[]",
            "{}",
            "18446744073709551615",
        ] {
            assert!(request_limit(&format!("{{\"maxConcurrentStreams\":{value}}}")).is_err());
        }
    }

    #[tokio::test]
    async fn cancelled_stream_task_releases_its_lease() {
        let limits = ShareStreamLimits::default();
        let lease = limits.try_acquire("owned", Some(1)).unwrap();
        let tasks = crate::managed_tasks::ManagedTaskRegistry::default();
        tasks.spawn(async move {
            let _lease = lease;
            std::future::pending::<()>().await;
        });
        assert!(limits.try_acquire("owned", Some(1)).is_none());
        tasks.shutdown().await;
        assert!(limits.try_acquire("owned", Some(1)).is_some());
    }

    #[test]
    fn active_grant_map_has_a_finite_capacity() {
        let limits = ShareStreamLimits::default();
        let leases = (0..crate::MAX_SHARE_GRANTS)
            .map(|id| limits.try_acquire(&format!("grant-{id}"), Some(1)).unwrap())
            .collect::<Vec<_>>();
        assert!(limits.try_acquire("overflow", Some(1)).is_none());
        drop(leases);
        assert!(limits.counts.lock().unwrap().active.is_empty());
        assert!(limits.try_acquire("overflow", Some(1)).is_some());
    }

    #[test]
    fn unconfigured_streams_count_when_an_explicit_limit_is_added() {
        let limits = ShareStreamLimits::default();
        let old_stream = limits.try_acquire("grant", None).unwrap();
        assert!(limits.try_acquire("grant", Some(1)).is_none());
        drop(old_stream);
        assert!(limits.try_acquire("grant", Some(1)).is_some());
    }
}
