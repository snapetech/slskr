use super::{relay, restore_changed_value_if_unchanged, unix_timestamp};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RelayState {
    pub(crate) enabled: bool,
    pub(crate) updated_at: u64,
    pub(crate) protocol: relay::RuntimeState,
}

impl RelayState {
    pub(crate) fn new() -> Self {
        Self {
            enabled: false,
            updated_at: unix_timestamp(),
            protocol: relay::RuntimeState::new(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn from_persisted(record: &crate::persistence::RuntimeCompatRecord) -> Self {
        Self {
            enabled: record.relay_enabled,
            updated_at: u64::try_from(record.updated_at).unwrap_or_else(|_| unix_timestamp()),
            protocol: relay::RuntimeState::new(),
        }
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool) -> serde_json::Value {
        self.enabled = enabled;
        self.updated_at = unix_timestamp();
        self.json_value("configured")
    }

    pub(crate) fn json_value(&self, status: &str) -> serde_json::Value {
        serde_json::json!({
            "relay_enabled": self.enabled,
            "enabled": self.enabled,
            "status": status,
            "updated_at": self.updated_at,
        })
    }

    pub(crate) fn rollback_changes_if_unchanged(&mut self, previous: &Self, mutated: &Self) {
        restore_changed_value_if_unchanged(&mut self.enabled, &previous.enabled, &mutated.enabled);
        restore_changed_value_if_unchanged(
            &mut self.updated_at,
            &previous.updated_at,
            &mutated.updated_at,
        );
        restore_changed_value_if_unchanged(
            &mut self.protocol,
            &previous.protocol,
            &mutated.protocol,
        );
    }
}
