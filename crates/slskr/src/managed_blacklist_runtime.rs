use super::{compile_controller_regexes, ControllerProfile, ControllerRegex};
use std::{collections::BTreeMap, net::IpAddr};

const MANAGED_BLACKLIST_DECISION_TTL_SECONDS: u64 = 10 * 60;
const MAX_MANAGED_BLACKLIST_DECISIONS: usize = 1_000;

#[derive(Debug)]
pub(crate) struct ManagedBlacklistRuntime {
    settings: crate::config::ManagedBlacklistSettings,
    patterns: Vec<ControllerRegex>,
    target: ControllerProfile,
    decisions: BTreeMap<String, (bool, u64)>,
}

impl ManagedBlacklistRuntime {
    pub(crate) fn username_is_blacklisted(&self, username: &str) -> bool {
        let member_match = match self.target {
            ControllerProfile::Legacy => self
                .settings
                .members
                .iter()
                .any(|member| member == username),
            ControllerProfile::Native => self
                .settings
                .members
                .iter()
                .any(|member| member.eq_ignore_ascii_case(username)),
        };
        member_match
            || self
                .patterns
                .iter()
                .any(|pattern| pattern.is_match(username))
    }

    pub(crate) fn new(
        settings: crate::config::ManagedBlacklistSettings,
        target: ControllerProfile,
        case_sensitive_regex: bool,
    ) -> Self {
        let patterns = compile_controller_regexes(&settings.patterns, case_sensitive_regex, target)
            .expect("validated blacklist patterns must compile");
        Self {
            settings,
            patterns,
            target,
            decisions: BTreeMap::new(),
        }
    }

    pub(crate) fn replace(
        &mut self,
        settings: crate::config::ManagedBlacklistSettings,
        target: ControllerProfile,
        case_sensitive_regex: bool,
    ) {
        self.patterns =
            compile_controller_regexes(&settings.patterns, case_sensitive_regex, target)
                .expect("validated blacklist patterns must compile");
        self.settings = settings;
        self.target = target;
        self.decisions.clear();
    }

    pub(crate) fn is_blacklisted(
        &mut self,
        username: Option<&str>,
        address: Option<IpAddr>,
        now: u64,
    ) -> bool {
        self.decisions.retain(|_, (_, decided_at)| {
            now.saturating_sub(*decided_at) < MANAGED_BLACKLIST_DECISION_TTL_SECONDS
        });
        let key = username
            .map(str::trim)
            .filter(|username| !username.is_empty())
            .map(str::to_owned);
        if let Some(cached) = key
            .as_ref()
            .and_then(|username| self.decisions.get(username))
            .map(|(blacklisted, _)| *blacklisted)
        {
            return cached;
        }
        let blacklisted = username.is_some_and(|username| self.username_is_blacklisted(username))
            || address.is_some_and(|address| self.settings.contains(address));
        if let Some(key) = key {
            if self.decisions.len() >= MAX_MANAGED_BLACKLIST_DECISIONS {
                if let Some(oldest) = self
                    .decisions
                    .iter()
                    .min_by_key(|(_, (_, decided_at))| *decided_at)
                    .map(|(username, _)| username.clone())
                {
                    self.decisions.remove(&oldest);
                }
            }
            self.decisions.insert(key, (blacklisted, now));
        }
        blacklisted
    }
}
