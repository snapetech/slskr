use super::ServerMessage;

use super::{
    json_bool_option, json_option, json_u32_option, json_u64_option, redact_username,
    unix_timestamp, AppConfig,
};

#[derive(Clone, Debug)]
pub(super) struct SessionSnapshot {
    pub(super) state: &'static str,
    pub(super) username: Option<String>,
    pub(super) supporter: Option<bool>,
    pub(super) privileges_seconds: Option<u32>,
    pub(super) last_error: Option<String>,
    pub(super) last_server_message: Option<String>,
    pub(super) server_messages_seen: u64,
    pub(super) reconnects: u64,
    pub(super) connected_at: Option<u64>,
    pub(super) updated_at: u64,
}

impl SessionSnapshot {
    pub(super) fn disconnected(config: &AppConfig) -> Self {
        Self {
            state: "disconnected",
            username: config.username.as_deref().map(redact_username),
            supporter: None,
            privileges_seconds: None,
            last_error: None,
            last_server_message: None,
            server_messages_seen: 0,
            reconnects: 0,
            connected_at: None,
            updated_at: unix_timestamp(),
        }
    }

    pub(super) fn json(&self) -> String {
        format!(
            "{{\"state\":\"{}\",\"username\":{},\"supporter\":{},\"privileges_seconds\":{},\"last_error\":{},\"last_server_message\":{},\"server_messages_seen\":{},\"reconnects\":{},\"connected_at\":{},\"updated_at\":{}}}",
            self.state,
            json_option(self.username.as_deref()),
            json_bool_option(self.supporter),
            json_u32_option(self.privileges_seconds),
            json_option(public_session_error(self.last_error.as_deref())),
            json_option(self.last_server_message.as_deref()),
            self.server_messages_seen,
            self.reconnects,
            json_u64_option(self.connected_at),
            self.updated_at
        )
    }

    pub(super) fn summary_json(&self) -> String {
        format!(
            "{{\"state\":\"{}\",\"connected\":{},\"privileges_seconds\":{},\"server_messages_seen\":{},\"reconnects\":{},\"connected_at\":{},\"updated_at\":{}}}",
            self.state,
            self.state == "connected",
            json_u32_option(self.privileges_seconds),
            self.server_messages_seen,
            self.reconnects,
            json_u64_option(self.connected_at),
            self.updated_at
        )
    }
}

pub(super) fn public_session_error(error: Option<&str>) -> Option<&'static str> {
    error.map(|error| {
        if error.starts_with("login failed") {
            "login failed"
        } else if error.starts_with("connect failed") {
            "connection failed"
        } else {
            "session operation failed"
        }
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SessionCommand {
    Connect,
    Disconnect,
    VpnDisconnect,
    Ping,
    CheckPrivileges,
    SetWaitPort {
        port: u32,
        obfuscated_port: Option<u32>,
    },
    DistributedBranch {
        has_parent: bool,
        accept_children: bool,
        level: u32,
        root: String,
    },
    Search {
        token: u32,
        query: String,
        target: SearchDispatchTarget,
    },
    WatchUser(String),
    UnwatchUser(String),
    BrowseUser(String),
    BrowseFolder {
        username: String,
        folder: String,
    },
    IndirectBrowse {
        username: String,
        token: u32,
    },
    RequestUserStats(String),
    RequestUserInterests(String),
    SendServerMessage(ServerMessage),
    TransferPeer {
        id: u64,
        username: String,
    },
    RequestPeerEndpoint(String),
    ProbePeerCapability(String),
    IndirectTransfer {
        id: u64,
        username: String,
        token: u32,
    },
    MessageUser {
        username: String,
        body: String,
    },
    MessageUsers {
        usernames: Vec<String>,
        body: String,
    },
    MessageAcked {
        id: u32,
    },
    RefreshRooms,
    JoinRoom(String),
    LeaveRoom(String),
    SayRoom {
        room: String,
        body: String,
    },
    SetRoomTicker {
        room: String,
        ticker: String,
    },
    AddRoomMember {
        room: String,
        username: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SearchDispatchTarget {
    Global,
    User(String),
    Room(String),
    Wishlist,
}
