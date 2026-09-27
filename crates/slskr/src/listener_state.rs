use super::{json_option, unix_timestamp, AppConfig};
use tokio::sync::oneshot;

#[derive(Clone, Debug)]
pub(super) struct ListenerSnapshot {
    pub(super) regular_bind: Option<String>,
    pub(super) regular_local_addr: Option<String>,
    pub(super) obfuscated_bind: Option<String>,
    pub(super) obfuscated_local_addr: Option<String>,
    pub(super) regular_accepts: u64,
    pub(super) obfuscated_accepts: u64,
    pub(super) peer_messages: u64,
    pub(super) obfuscated_peer_messages: u64,
    pub(super) file_transfers: u64,
    pub(super) distributed: u64,
    pub(super) peer_inits: u64,
    pub(super) pierce_firewalls: u64,
    pub(super) unknown_inits: u64,
    pub(super) user_info_requests: u64,
    pub(super) user_info_responses: u64,
    pub(super) share_list_requests: u64,
    pub(super) share_list_responses: u64,
    pub(super) file_search_requests: u64,
    pub(super) file_search_responses: u64,
    pub(super) transfer_rejections: u64,
    pub(super) unsupported_peer_messages: u64,
    pub(super) errors: u64,
    pub(super) last_event: Option<String>,
    pub(super) last_error: Option<String>,
    pub(super) updated_at: u64,
}

#[derive(Debug)]
pub(super) enum ListenerCommand {
    Reconfigure {
        bind: Option<String>,
        response: oneshot::Sender<Result<bool, String>>,
    },
}

impl ListenerSnapshot {
    pub(super) fn new(config: &AppConfig) -> Self {
        Self {
            regular_bind: config.listener_bind.clone(),
            regular_local_addr: None,
            obfuscated_bind: config
                .obfuscation_enabled
                .then(|| config.obfuscated_listener_bind.clone())
                .flatten(),
            obfuscated_local_addr: None,
            regular_accepts: 0,
            obfuscated_accepts: 0,
            peer_messages: 0,
            obfuscated_peer_messages: 0,
            file_transfers: 0,
            distributed: 0,
            peer_inits: 0,
            pierce_firewalls: 0,
            unknown_inits: 0,
            user_info_requests: 0,
            user_info_responses: 0,
            share_list_requests: 0,
            share_list_responses: 0,
            file_search_requests: 0,
            file_search_responses: 0,
            transfer_rejections: 0,
            unsupported_peer_messages: 0,
            errors: 0,
            last_event: None,
            last_error: None,
            updated_at: unix_timestamp(),
        }
    }

    pub(super) fn json(&self) -> String {
        format!(
            "{{\"regular_bind\":{},\"regular_local_addr\":{},\"obfuscated_bind\":{},\"obfuscated_local_addr\":{},\"regular_accepts\":{},\"obfuscated_accepts\":{},\"peer_messages\":{},\"obfuscated_peer_messages\":{},\"file_transfers\":{},\"distributed\":{},\"peer_inits\":{},\"pierce_firewalls\":{},\"unknown_inits\":{},\"user_info_requests\":{},\"user_info_responses\":{},\"share_list_requests\":{},\"share_list_responses\":{},\"file_search_requests\":{},\"file_search_responses\":{},\"transfer_rejections\":{},\"unsupported_peer_messages\":{},\"errors\":{},\"last_event\":{},\"last_error\":{},\"updated_at\":{}}}",
            json_option(self.regular_bind.as_deref()),
            json_option(self.regular_local_addr.as_deref()),
            json_option(self.obfuscated_bind.as_deref()),
            json_option(self.obfuscated_local_addr.as_deref()),
            self.regular_accepts,
            self.obfuscated_accepts,
            self.peer_messages,
            self.obfuscated_peer_messages,
            self.file_transfers,
            self.distributed,
            self.peer_inits,
            self.pierce_firewalls,
            self.unknown_inits,
            self.user_info_requests,
            self.user_info_responses,
            self.share_list_requests,
            self.share_list_responses,
            self.file_search_requests,
            self.file_search_responses,
            self.transfer_rejections,
            self.unsupported_peer_messages,
            self.errors,
            json_option(self.last_event.as_deref()),
            json_option(public_listener_error(self.last_error.as_deref())),
            self.updated_at
        )
    }
}

pub(super) fn public_listener_error(error: Option<&str>) -> Option<&'static str> {
    error.map(|_| "listener unavailable")
}
