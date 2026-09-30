use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Default)]
struct MapEnv {
    values: BTreeMap<String, String>,
}

impl MapEnv {
    fn with(mut self, name: &str, value: &str) -> Self {
        self.values.insert(name.to_owned(), value.to_owned());
        self
    }
}

impl super::ConfigEnv for MapEnv {
    fn var(&self, name: &str) -> Option<String> {
        self.values.get(name).cloned()
    }
}

#[path = "config_tests/federation_membership.rs"]
mod federation_membership;

#[path = "config_tests/file_layers.rs"]
mod file_layers;

#[path = "config_tests/media_integrations.rs"]
mod media_integrations;

#[path = "config_tests/peer_transport.rs"]
mod peer_transport;

#[path = "config_tests/runtime_policy.rs"]
mod runtime_policy;

#[path = "config_tests/transfer_policy.rs"]
mod transfer_policy;

#[path = "config_tests/web_security.rs"]
mod web_security;
