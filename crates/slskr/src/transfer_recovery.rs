use super::{virtual_basename, SearchResultEntry, SearchStore, TransferEntry};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug)]
pub(super) struct AutoRetryCandidate {
    pub(super) source: TransferEntry,
    pub(super) username: String,
    pub(super) filename: String,
    pub(super) size: Option<u64>,
    pub(super) source_kind: &'static str,
}

#[derive(Debug, Default)]
pub(super) struct AutoRetryTracker {
    pub(super) retried_ids: HashSet<u64>,
    pub(super) retry_counts: BTreeMap<String, usize>,
    pub(super) peer_retry_after: BTreeMap<String, u64>,
    pub(super) alternate_search_requested_at: BTreeMap<u64, u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum UnderperformanceReason {
    QueuedTooLong,
    ThroughputTooLow,
    Stalled,
}

impl UnderperformanceReason {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::QueuedTooLong => "queued_too_long",
            Self::ThroughputTooLow => "throughput_too_low",
            Self::Stalled => "stalled",
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum RescueAction {
    Search,
    Replace(SearchResultEntry),
}

#[derive(Clone, Debug)]
pub(super) struct RescueCandidate {
    pub(super) source: TransferEntry,
    pub(super) reason: UnderperformanceReason,
    pub(super) action: RescueAction,
}

#[derive(Debug, Default)]
pub(super) struct RescueTracker {
    pub(super) last_progress: BTreeMap<u64, (u64, u64)>,
    pub(super) search_requested_at: BTreeMap<u64, u64>,
    pub(super) retry_after: BTreeMap<u64, u64>,
}

fn prune_rescue_tracker(tracker: &mut RescueTracker, entries: &[TransferEntry]) {
    let active_ids = entries
        .iter()
        .filter(|entry| {
            entry.direction == 0 && matches!(entry.status.as_str(), "queued" | "in_progress")
        })
        .map(|entry| entry.id)
        .collect::<HashSet<_>>();
    tracker
        .last_progress
        .retain(|id, _| active_ids.contains(id));
    tracker
        .search_requested_at
        .retain(|id, _| active_ids.contains(id));
    tracker.retry_after.retain(|id, _| active_ids.contains(id));
}

fn underperformance_reason(
    entry: &TransferEntry,
    tracker: &mut RescueTracker,
    settings: &crate::config::TransferRescueSettings,
    now: u64,
) -> Option<UnderperformanceReason> {
    if entry.status == "queued"
        && now.saturating_sub(entry.requested_at) >= settings.max_queue_time.as_secs()
    {
        return Some(UnderperformanceReason::QueuedTooLong);
    }
    if entry.status != "in_progress" {
        return None;
    }
    let duration = entry
        .started_at
        .map(|started_at| now.saturating_sub(started_at))
        .unwrap_or(0);
    if duration >= settings.min_duration.as_secs()
        && entry.average_speed_at(now) < settings.min_throughput_bytes_per_second as f64
    {
        return Some(UnderperformanceReason::ThroughputTooLow);
    }
    let (last_bytes, last_progress_at) = tracker
        .last_progress
        .entry(entry.id)
        .or_insert((entry.bytes_transferred, now));
    if entry.bytes_transferred > *last_bytes {
        *last_bytes = entry.bytes_transferred;
        *last_progress_at = now;
        return None;
    }
    (now.saturating_sub(*last_progress_at) >= settings.stalled_timeout.as_secs())
        .then_some(UnderperformanceReason::Stalled)
}

pub(super) fn create_rescue_plan(
    entries: &[TransferEntry],
    searches: &SearchStore,
    tracker: &mut RescueTracker,
    settings: &crate::config::TransferRescueSettings,
    now: u64,
) -> Vec<RescueCandidate> {
    prune_rescue_tracker(tracker, entries);
    let latest = latest_download_attempts(entries);
    let mut candidates = entries
        .iter()
        .filter(|entry| {
            if entry.direction != 0
                || !matches!(entry.status.as_str(), "queued" | "in_progress")
                || !is_auto_retry_audio_file(&entry.filename)
                || entry.peer_username.is_none()
                || tracker
                    .retry_after
                    .get(&entry.id)
                    .is_some_and(|retry_after| *retry_after > now)
            {
                return false;
            }
            let identity = entry.request_id.clone().unwrap_or_else(|| {
                auto_retry_key(
                    entry.peer_username.as_deref().unwrap_or_default(),
                    &entry.filename,
                )
            });
            latest.get(&identity) == Some(&entry.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by_key(|entry| (entry.requested_at, entry.id));

    let mut plan = Vec::new();
    for source in candidates {
        let Some(reason) = underperformance_reason(&source, tracker, settings, now) else {
            continue;
        };
        let alternative = searches.best_transfer_alternative(
            &source,
            f64::from(settings.alternate_source_size_tolerance_percent),
            |_| false,
        );
        let action = if let Some(alternative) = alternative {
            RescueAction::Replace(alternative)
        } else if tracker
            .search_requested_at
            .get(&source.id)
            .is_none_or(|requested_at| {
                requested_at.saturating_add(settings.retry_cooldown.as_secs()) <= now
            })
        {
            RescueAction::Search
        } else {
            continue;
        };
        plan.push(RescueCandidate {
            source,
            reason,
            action,
        });
        if plan.len() >= settings.max_files_per_cycle {
            break;
        }
    }
    plan
}

pub(super) fn auto_retry_key(username: &str, filename: &str) -> String {
    format!("{}\u{1f}{}", username.to_ascii_lowercase(), filename)
}

pub(super) fn prune_auto_retry_tracker(
    tracker: &mut AutoRetryTracker,
    entries: &[TransferEntry],
    now: u64,
) {
    let live_ids = entries.iter().map(|entry| entry.id).collect::<HashSet<_>>();
    let live_retry_keys = entries
        .iter()
        .filter_map(|entry| {
            entry
                .peer_username
                .as_deref()
                .map(|username| auto_retry_key(username, &entry.filename))
        })
        .collect::<HashSet<_>>();
    tracker.retried_ids.retain(|id| live_ids.contains(id));
    tracker
        .alternate_search_requested_at
        .retain(|id, _| live_ids.contains(id));
    tracker
        .retry_counts
        .retain(|key, _| live_retry_keys.contains(key));
    tracker
        .peer_retry_after
        .retain(|_, retry_after| *retry_after > now);
}

pub(super) fn is_auto_retry_audio_file(filename: &str) -> bool {
    let extension = virtual_basename(filename)
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());
    matches!(
        extension.as_deref(),
        Some("flac" | "mp3" | "m4a" | "aac" | "ogg" | "opus" | "wav" | "alac")
    )
}

pub(super) fn alternate_size_is_eligible(
    original: Option<u64>,
    candidate: u64,
    tolerance: f64,
) -> bool {
    let Some(original) = original.filter(|size| *size > 0) else {
        return true;
    };
    let difference_percent = (original.abs_diff(candidate) as f64 / original as f64) * 100.0;
    difference_percent <= tolerance
}

pub(super) fn latest_download_attempts(entries: &[TransferEntry]) -> BTreeMap<String, u64> {
    let mut latest = BTreeMap::<String, u64>::new();
    for entry in entries.iter().filter(|entry| entry.direction == 0) {
        let identity = entry.request_id.clone().unwrap_or_else(|| {
            auto_retry_key(
                entry.peer_username.as_deref().unwrap_or_default(),
                &entry.filename,
            )
        });
        latest
            .entry(identity)
            .and_modify(|id| *id = (*id).max(entry.id))
            .or_insert(entry.id);
    }
    latest
}

pub(super) fn create_auto_retry_plan(
    entries: &[TransferEntry],
    searches: &SearchStore,
    tracker: &AutoRetryTracker,
    settings: &crate::config::TransferAutoRetrySettings,
    now: u64,
    available_slots: usize,
) -> Vec<AutoRetryCandidate> {
    if available_slots == 0 {
        return Vec::new();
    }
    let cutoff = now.saturating_sub(settings.retry_delay.as_secs());
    let latest = latest_download_attempts(entries);
    let mut failed = entries
        .iter()
        .filter(|entry| {
            if entry.direction != 0
                || entry.status != "failed"
                || entry.updated_at > cutoff
                || tracker.retried_ids.contains(&entry.id)
                || !is_auto_retry_audio_file(&entry.filename)
                || entry.peer_username.is_none()
            {
                return false;
            }
            let identity = entry.request_id.clone().unwrap_or_else(|| {
                auto_retry_key(
                    entry.peer_username.as_deref().unwrap_or_default(),
                    &entry.filename,
                )
            });
            latest.get(&identity) == Some(&entry.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    failed.sort_by_key(|entry| (entry.updated_at, entry.id));

    let global_limit = settings.max_files_per_cycle.min(available_slots);
    let mut per_peer = BTreeMap::<String, usize>::new();
    let mut alternate_searches = 0usize;
    let mut plan = Vec::new();
    for source in failed {
        let alternative = settings
            .alternate_sources_enabled
            .then(|| {
                searches.best_transfer_alternative(
                    &source,
                    settings.alternate_source_size_tolerance_percent,
                    |username| {
                        tracker
                            .peer_retry_after
                            .get(&username.to_ascii_lowercase())
                            .is_some_and(|retry_after| *retry_after > now)
                    },
                )
            })
            .flatten();
        let (username, filename, size, source_kind) = if let Some(alternative) = alternative {
            let Some(username) = alternative.peer_username else {
                continue;
            };
            (
                username,
                alternative.filename,
                Some(alternative.size),
                "cached-search",
            )
        } else if settings.alternate_sources_enabled
            && !tracker
                .alternate_search_requested_at
                .contains_key(&source.id)
            && alternate_searches < settings.max_alternate_source_searches_per_cycle
        {
            alternate_searches += 1;
            (
                source
                    .peer_username
                    .clone()
                    .expect("filtered peer username"),
                source.filename.clone(),
                source.size,
                "network-search",
            )
        } else if tracker
            .alternate_search_requested_at
            .get(&source.id)
            .is_some_and(|requested_at| {
                requested_at.saturating_add(settings.check_interval.as_secs()) > now
            })
        {
            continue;
        } else {
            (
                source
                    .peer_username
                    .clone()
                    .expect("filtered peer username"),
                source.filename.clone(),
                source.size,
                "original",
            )
        };
        let peer_key = username.to_ascii_lowercase();
        if tracker
            .peer_retry_after
            .get(&peer_key)
            .is_some_and(|retry_after| *retry_after > now)
        {
            continue;
        }
        if per_peer.get(&peer_key).copied().unwrap_or(0) >= settings.max_files_per_peer_per_cycle {
            continue;
        }
        let retry_key = auto_retry_key(&username, &filename);
        let attempts = tracker.retry_counts.get(&retry_key).copied().unwrap_or(0);
        if settings.max_attempts != 0 && attempts >= settings.max_attempts {
            continue;
        }
        *per_peer.entry(peer_key).or_default() += 1;
        plan.push(AutoRetryCandidate {
            source,
            username,
            filename,
            size,
            source_kind,
        });
        if plan.len() >= global_limit {
            break;
        }
    }
    plan
}
