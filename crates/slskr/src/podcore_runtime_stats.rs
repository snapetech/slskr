use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicI64, AtomicU64, Ordering},
        Mutex,
    },
};

pub(super) const MAX_PODCORE_STAT_KEYS: usize = 4_096;
pub(super) const MAX_PODCORE_STAT_KEY_BYTES: usize = 256;

/// Runtime counters for PodCore operations that are implemented locally.
///
/// The oracle keeps these counters in its backfill/discovery services. Keep
/// them separate from the persisted controller-feature records: those records
/// describe current publications and cursors, while these values describe
/// work performed by this process since startup.
#[derive(Debug, Default)]
pub(super) struct PodCoreRuntimeStats {
    pub(super) dht_active_publications: AtomicI64,
    pub(super) dht_expired_publications: AtomicU64,
    pub(super) dht_publications_by_domain: Mutex<BTreeMap<String, u64>>,
    pub(super) dht_publications_by_visibility: Mutex<BTreeMap<String, u64>>,
    pub(super) dht_last_publish_operation: Mutex<Option<String>>,
    pub(super) backfill_requests: AtomicU64,
    pub(super) backfill_requests_received: AtomicU64,
    pub(super) backfill_completed: AtomicU64,
    pub(super) backfill_failed: AtomicU64,
    pub(super) messages_backfilled: AtomicU64,
    pub(super) backfill_bytes_transferred: AtomicU64,
    pub(super) total_backfill_duration_ms: AtomicU64,
    pub(super) backfill_requests_by_pod: Mutex<BTreeMap<String, u64>>,
    pub(super) last_backfill_operation: Mutex<Option<String>>,
    pub(super) discovery_searches: AtomicU64,
    pub(super) total_discovery_search_time_ms: AtomicU64,
    pub(super) discovery_searches_by_type: Mutex<BTreeMap<String, u64>>,
    pub(super) last_discovery_operation: Mutex<Option<String>>,
    pub(super) routing_messages: AtomicU64,
    pub(super) routing_attempts: AtomicU64,
    pub(super) routing_successes: AtomicU64,
    pub(super) routing_failures: AtomicU64,
    pub(super) total_routing_time_ms: AtomicU64,
    pub(super) routing_messages_by_pod: Mutex<BTreeMap<String, u64>>,
    pub(super) last_routing_operation: Mutex<Option<String>>,
    /// Last locally observed opinion count per pod. The frozen PodCore
    /// opinion service reports the number of opinions newly observed during
    /// a refresh rather than returning a constant zero.
    pub(super) opinion_refresh_counts: Mutex<BTreeMap<String, usize>>,
}

fn podcore_stat_key(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_PODCORE_STAT_KEY_BYTES
        || value.chars().any(char::is_control)
    {
        return None;
    }
    Some(value)
}

fn increment_podcore_stat(map: &mut BTreeMap<String, u64>, key: &str) {
    let Some(key) = podcore_stat_key(key) else {
        return;
    };
    if !map.contains_key(key) && map.len() >= MAX_PODCORE_STAT_KEYS {
        return;
    }
    let count = map.entry(key.to_owned()).or_default();
    *count = count.saturating_add(1);
}

impl PodCoreRuntimeStats {
    pub(super) fn record_dht_unpublish(&self) {
        self.dht_active_publications.fetch_sub(1, Ordering::Relaxed);
        self.dht_expired_publications
            .fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn record_dht_publish(&self, pod: Option<&serde_json::Value>) {
        self.dht_active_publications.fetch_add(1, Ordering::Relaxed);
        if let Some(pod) = pod {
            if let Some(domain) = pod
                .get("focusContentId")
                .and_then(serde_json::Value::as_str)
                .and_then(|content_id| content_id.split(':').nth(1))
            {
                if let Ok(mut by_domain) = self.dht_publications_by_domain.lock() {
                    increment_podcore_stat(&mut by_domain, domain);
                }
            }
            let visibility = super::podcore_controller::pod_visibility_name(
                pod.get("visibility").unwrap_or(&serde_json::Value::from(1)),
            );
            if let Ok(mut by_visibility) = self.dht_publications_by_visibility.lock() {
                *by_visibility.entry(visibility).or_default() += 1;
            }
        }
        if let Ok(mut last_operation) = self.dht_last_publish_operation.lock() {
            *last_operation = Some(chrono::Utc::now().to_rfc3339());
        }
    }

    pub(super) fn record_dht_refresh(&self) {
        if let Ok(mut last_operation) = self.dht_last_publish_operation.lock() {
            *last_operation = Some(chrono::Utc::now().to_rfc3339());
        }
    }

    pub(super) fn record_backfill(
        &self,
        pod_id: &str,
        messages: usize,
        bytes: usize,
        elapsed_ms: u64,
        success: bool,
    ) {
        self.backfill_requests.fetch_add(1, Ordering::Relaxed);
        if success {
            self.backfill_completed.fetch_add(1, Ordering::Relaxed);
            self.messages_backfilled
                .fetch_add(messages as u64, Ordering::Relaxed);
            self.backfill_bytes_transferred
                .fetch_add(bytes as u64, Ordering::Relaxed);
            self.total_backfill_duration_ms
                .fetch_add(elapsed_ms, Ordering::Relaxed);
            if let Ok(mut by_pod) = self.backfill_requests_by_pod.lock() {
                increment_podcore_stat(&mut by_pod, pod_id);
            }
        } else {
            self.backfill_failed.fetch_add(1, Ordering::Relaxed);
        }
        if let Ok(mut last_operation) = self.last_backfill_operation.lock() {
            *last_operation = Some(chrono::Utc::now().to_rfc3339());
        }
    }

    pub(super) fn record_routing(
        &self,
        pod_id: &str,
        target_count: usize,
        successful_count: usize,
        failed_count: usize,
        elapsed_ms: u64,
    ) {
        self.routing_messages.fetch_add(1, Ordering::Relaxed);
        self.routing_attempts
            .fetch_add(target_count as u64, Ordering::Relaxed);
        self.routing_successes
            .fetch_add(successful_count as u64, Ordering::Relaxed);
        self.routing_failures
            .fetch_add(failed_count as u64, Ordering::Relaxed);
        self.total_routing_time_ms
            .fetch_add(elapsed_ms, Ordering::Relaxed);
        if let Ok(mut by_pod) = self.routing_messages_by_pod.lock() {
            increment_podcore_stat(&mut by_pod, pod_id);
        }
        if let Ok(mut last_operation) = self.last_routing_operation.lock() {
            *last_operation = Some(chrono::Utc::now().to_rfc3339());
        }
    }

    pub(super) fn record_discovery_search(&self, search_type: &str, elapsed_ms: u64) {
        self.discovery_searches.fetch_add(1, Ordering::Relaxed);
        self.total_discovery_search_time_ms
            .fetch_add(elapsed_ms, Ordering::Relaxed);
        if let Ok(mut by_type) = self.discovery_searches_by_type.lock() {
            increment_podcore_stat(&mut by_type, search_type);
        }
        if let Ok(mut last_operation) = self.last_discovery_operation.lock() {
            *last_operation = Some(chrono::Utc::now().to_rfc3339());
        }
    }

    pub(super) fn record_opinion_refresh(&self, pod_id: &str, opinion_count: usize) -> usize {
        let Some(pod_id) = podcore_stat_key(pod_id) else {
            return 0;
        };
        let Ok(mut counts) = self.opinion_refresh_counts.lock() else {
            return 0;
        };
        if !counts.contains_key(pod_id) && counts.len() >= MAX_PODCORE_STAT_KEYS {
            return 0;
        }
        let previous_count = counts.insert(pod_id.to_owned(), opinion_count).unwrap_or(0);
        opinion_count.saturating_sub(previous_count)
    }
}

/// Matches the oracle's `MessageSigner`'s real `Interlocked` counters --
/// real signing/verification activity, not the hardcoded zeros
/// `/api/podcore/signing/stats` used to report regardless of use.
#[derive(Debug, Default)]
pub(super) struct PodSignatureStats {
    pub(super) signatures_created: AtomicU64,
    pub(super) signatures_verified: AtomicU64,
    pub(super) successful_verifications: AtomicU64,
    pub(super) failed_verifications: AtomicU64,
    pub(super) total_signing_time_ms: AtomicU64,
    pub(super) total_verification_time_ms: AtomicU64,
    pub(super) last_operation_at: Mutex<Option<String>>,
}

impl PodSignatureStats {
    pub(super) fn record_sign(&self, elapsed_ms: u64) {
        self.signatures_created.fetch_add(1, Ordering::Relaxed);
        self.total_signing_time_ms
            .fetch_add(elapsed_ms, Ordering::Relaxed);
        self.touch();
    }

    pub(super) fn record_verify(&self, elapsed_ms: u64, success: bool) {
        self.signatures_verified.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful_verifications
                .fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_verifications.fetch_add(1, Ordering::Relaxed);
        }
        self.total_verification_time_ms
            .fetch_add(elapsed_ms, Ordering::Relaxed);
        self.touch();
    }

    fn touch(&self) {
        if let Ok(mut guard) = self.last_operation_at.lock() {
            *guard = Some(chrono::Utc::now().to_rfc3339());
        }
    }
}

/// Matches the oracle's `PodMembershipVerifier`'s real counters -- real
/// membership/signature verification activity, not the hardcoded zeros
/// `/api/podcore/verification/stats` used to report regardless of use.
#[derive(Debug, Default)]
pub(super) struct PodVerificationStats {
    pub(super) total_verifications: AtomicU64,
    pub(super) successful_verifications: AtomicU64,
    pub(super) failed_membership_checks: AtomicU64,
    pub(super) failed_signature_checks: AtomicU64,
    pub(super) banned_member_rejections: AtomicU64,
    pub(super) total_verification_time_ms: AtomicU64,
    pub(super) last_verification_at: Mutex<Option<String>>,
}

impl PodVerificationStats {
    pub(super) fn record(
        &self,
        elapsed_ms: u64,
        is_from_valid_member: bool,
        is_not_banned: bool,
        has_valid_signature: bool,
        is_valid: bool,
    ) {
        self.total_verifications.fetch_add(1, Ordering::Relaxed);
        self.total_verification_time_ms
            .fetch_add(elapsed_ms, Ordering::Relaxed);
        if is_valid {
            self.successful_verifications
                .fetch_add(1, Ordering::Relaxed);
        } else if !is_not_banned {
            self.banned_member_rejections
                .fetch_add(1, Ordering::Relaxed);
        } else if !is_from_valid_member {
            self.failed_membership_checks
                .fetch_add(1, Ordering::Relaxed);
        }
        if !has_valid_signature {
            self.failed_signature_checks.fetch_add(1, Ordering::Relaxed);
        }
        if let Ok(mut guard) = self.last_verification_at.lock() {
            *guard = Some(chrono::Utc::now().to_rfc3339());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PodCoreRuntimeStats, PodSignatureStats, PodVerificationStats, MAX_PODCORE_STAT_KEYS,
        MAX_PODCORE_STAT_KEY_BYTES,
    };
    use std::sync::atomic::Ordering;

    #[test]
    fn diagnostic_dimension_maps_bound_untrusted_keys() {
        let stats = PodCoreRuntimeStats::default();
        for index in 0..MAX_PODCORE_STAT_KEYS {
            stats.record_discovery_search(&format!("type-{index}"), 0);
        }
        stats.record_discovery_search("new-type", 0);
        stats.record_discovery_search(&"x".repeat(MAX_PODCORE_STAT_KEY_BYTES + 1), 0);

        assert_eq!(
            stats
                .discovery_searches_by_type
                .lock()
                .expect("discovery stat lock")
                .len(),
            MAX_PODCORE_STAT_KEYS
        );
        assert_eq!(
            stats.record_opinion_refresh(&"x".repeat(MAX_PODCORE_STAT_KEY_BYTES + 1), 1,),
            0
        );
    }

    #[test]
    fn signing_and_membership_counters_track_real_operations() {
        let signing = PodSignatureStats::default();
        signing.record_sign(5);
        signing.record_verify(7, true);
        signing.record_verify(11, false);

        assert_eq!(signing.signatures_created.load(Ordering::Relaxed), 1);
        assert_eq!(signing.total_signing_time_ms.load(Ordering::Relaxed), 5);
        assert_eq!(signing.signatures_verified.load(Ordering::Relaxed), 2);
        assert_eq!(signing.successful_verifications.load(Ordering::Relaxed), 1);
        assert_eq!(signing.failed_verifications.load(Ordering::Relaxed), 1);
        assert_eq!(
            signing.total_verification_time_ms.load(Ordering::Relaxed),
            18
        );
        assert!(signing
            .last_operation_at
            .lock()
            .expect("signing operation lock")
            .is_some());

        let verification = PodVerificationStats::default();
        verification.record(13, true, true, true, true);
        verification.record(17, false, true, false, false);
        verification.record(19, true, false, true, false);

        assert_eq!(verification.total_verifications.load(Ordering::Relaxed), 3);
        assert_eq!(
            verification
                .successful_verifications
                .load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            verification
                .failed_membership_checks
                .load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            verification.failed_signature_checks.load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            verification
                .banned_member_rejections
                .load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            verification
                .total_verification_time_ms
                .load(Ordering::Relaxed),
            49
        );
        assert!(verification
            .last_verification_at
            .lock()
            .expect("verification operation lock")
            .is_some());
    }
}
