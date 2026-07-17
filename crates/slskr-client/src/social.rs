use std::collections::{HashMap, HashSet};

use slskr_protocol::server::{PrivateMessage, ServerMessage, UserStatus, WatchedUser};

use crate::ClientError;

pub const MAX_PRIVATE_MESSAGE_RECIPIENTS: usize = 100;
pub const MAX_STORED_ROOM_MESSAGES: usize = 1_000;
pub const MAX_STORED_PRIVATE_MESSAGES: usize = 1_000;
pub const MAX_STORED_SOCIAL_FIELD_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_USER_WATCH_RECORDS: usize = 1_024;
pub const DEFAULT_MAX_JOINED_ROOMS: usize = 1_024;

fn retain_newest<T>(items: &mut Vec<T>, max: usize) {
    if items.len() > max {
        items.drain(..items.len() - max);
    }
}

pub fn private_message_users_command<I, S>(
    usernames: I,
    message: impl Into<String>,
) -> Result<ServerMessage, ClientError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let message = message.into();
    if message.len() > MAX_STORED_SOCIAL_FIELD_BYTES {
        return Err(ClientError::PrivateMessageFieldTooLong {
            field: "body",
            length: message.len(),
            max: MAX_STORED_SOCIAL_FIELD_BYTES,
        });
    }
    let mut seen = HashSet::new();
    let mut recipients = Vec::new();

    for username in usernames {
        let username = username.into().trim().to_owned();
        if username.is_empty() {
            return Err(ClientError::BlankMessageRecipient);
        }
        if username.len() > MAX_STORED_SOCIAL_FIELD_BYTES {
            return Err(ClientError::PrivateMessageFieldTooLong {
                field: "recipient",
                length: username.len(),
                max: MAX_STORED_SOCIAL_FIELD_BYTES,
            });
        }
        if seen.insert(username.to_ascii_lowercase()) {
            recipients.push(username);
            if recipients.len() > MAX_PRIVATE_MESSAGE_RECIPIENTS {
                return Err(ClientError::TooManyMessageRecipients {
                    count: recipients.len(),
                    max: MAX_PRIVATE_MESSAGE_RECIPIENTS,
                });
            }
        }
    }

    if recipients.is_empty() {
        return Err(ClientError::EmptyMessageRecipients);
    }
    Ok(ServerMessage::MessageUsers {
        usernames: recipients,
        message,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserWatchState {
    watched: HashMap<String, WatchedUser>,
    statuses: HashMap<String, UserStatus>,
    max_records: usize,
}

impl Default for UserWatchState {
    fn default() -> Self {
        Self::with_max_records(DEFAULT_MAX_USER_WATCH_RECORDS)
    }
}

impl UserWatchState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_max_records(max_records: usize) -> Self {
        Self {
            watched: HashMap::new(),
            statuses: HashMap::new(),
            max_records: max_records.max(1),
        }
    }

    fn can_insert(&self, username: &str) -> bool {
        let key = username_key(username);
        self.watched.contains_key(&key)
            || self.statuses.contains_key(&key)
            || self
                .watched
                .keys()
                .chain(self.statuses.keys())
                .collect::<HashSet<_>>()
                .len()
                < self.max_records
    }

    #[must_use]
    pub fn watch_message(username: impl Into<String>) -> ServerMessage {
        ServerMessage::WatchUserRequest {
            username: username.into().trim().to_owned(),
        }
    }

    #[must_use]
    pub fn unwatch_message(username: impl Into<String>) -> ServerMessage {
        ServerMessage::UnwatchUser {
            username: username.into().trim().to_owned(),
        }
    }

    pub fn apply_server_message(&mut self, message: &ServerMessage) -> bool {
        match message {
            ServerMessage::WatchUserResponse(user) => {
                if user.username.len() > MAX_STORED_SOCIAL_FIELD_BYTES
                    || user
                        .country_code
                        .as_ref()
                        .is_some_and(|country| country.len() > MAX_STORED_SOCIAL_FIELD_BYTES)
                {
                    return false;
                }
                let mut user = user.clone();
                user.username = user.username.trim().to_owned();
                if user.username.is_empty() || !self.can_insert(&user.username) {
                    return false;
                }
                self.watched.insert(username_key(&user.username), user);
                true
            }
            ServerMessage::GetUserStatusResponse(status) => {
                if status.username.len() > MAX_STORED_SOCIAL_FIELD_BYTES {
                    return false;
                }
                let mut status = status.clone();
                status.username = status.username.trim().to_owned();
                if status.username.is_empty() || !self.can_insert(&status.username) {
                    return false;
                }
                self.statuses.insert(username_key(&status.username), status);
                true
            }
            ServerMessage::UnwatchUser { username } => {
                let key = username_key(username);
                self.watched.remove(&key);
                self.statuses.remove(&key);
                true
            }
            _ => false,
        }
    }

    #[must_use]
    pub fn watched(&self, username: &str) -> Option<&WatchedUser> {
        self.watched.get(&username_key(username))
    }

    #[must_use]
    pub fn status(&self, username: &str) -> Option<&UserStatus> {
        self.statuses.get(&username_key(username))
    }
}

fn username_key(username: &str) -> String {
    username.trim().to_ascii_lowercase()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomState {
    joined: HashSet<String>,
    messages: Vec<RoomMessage>,
    max_joined_rooms: usize,
}

impl Default for RoomState {
    fn default() -> Self {
        Self::with_max_joined_rooms(DEFAULT_MAX_JOINED_ROOMS)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomMessage {
    pub room: String,
    pub username: String,
    pub message: String,
}

impl RoomState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_max_joined_rooms(max_joined_rooms: usize) -> Self {
        Self {
            joined: HashSet::new(),
            messages: Vec::new(),
            max_joined_rooms: max_joined_rooms.max(1),
        }
    }

    #[must_use]
    pub fn join_global_message() -> ServerMessage {
        ServerMessage::JoinGlobalRoom
    }

    #[must_use]
    pub fn leave_global_message() -> ServerMessage {
        ServerMessage::LeaveGlobalRoom
    }

    #[must_use]
    pub fn leave_room_message(room: impl Into<String>) -> ServerMessage {
        ServerMessage::LeaveRoom {
            room: room.into().trim().to_owned(),
        }
    }

    pub fn apply_server_message(&mut self, message: &ServerMessage) -> bool {
        match message {
            ServerMessage::GlobalRoomMessage {
                room,
                username,
                message,
            } => {
                if [room, username, message]
                    .iter()
                    .any(|value| value.len() > MAX_STORED_SOCIAL_FIELD_BYTES)
                {
                    return false;
                }
                let room = room.trim();
                let username = username.trim();
                if room.is_empty() || username.is_empty() {
                    return false;
                }
                let key = room_key(room);
                if self.joined.len() >= self.max_joined_rooms && !self.joined.contains(&key) {
                    return false;
                }
                self.joined.insert(key);
                self.messages.push(RoomMessage {
                    room: room.to_owned(),
                    username: username.to_owned(),
                    message: message.clone(),
                });
                retain_newest(&mut self.messages, MAX_STORED_ROOM_MESSAGES);
                true
            }
            ServerMessage::LeaveRoom { room } => {
                self.joined.remove(&room_key(room));
                true
            }
            _ => false,
        }
    }

    #[must_use]
    pub fn is_joined(&self, room: &str) -> bool {
        self.joined.contains(&room_key(room))
    }

    #[must_use]
    pub fn messages(&self) -> &[RoomMessage] {
        &self.messages
    }
}

fn room_key(room: &str) -> String {
    room.trim().to_ascii_lowercase()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrivateMessageInbox {
    messages: Vec<PrivateMessage>,
}

impl PrivateMessageInbox {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_server_message(&mut self, message: &ServerMessage) -> Option<ServerMessage> {
        match message {
            ServerMessage::MessageUserResponse(message) => {
                if message.username.len() > MAX_STORED_SOCIAL_FIELD_BYTES
                    || message.message.len() > MAX_STORED_SOCIAL_FIELD_BYTES
                {
                    return Some(ServerMessage::MessageAcked { id: message.id });
                }
                if !self.messages.iter().any(|stored| stored.id == message.id) {
                    self.messages.push(message.clone());
                    retain_newest(&mut self.messages, MAX_STORED_PRIVATE_MESSAGES);
                }
                Some(ServerMessage::MessageAcked { id: message.id })
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn messages(&self) -> &[PrivateMessage] {
        &self.messages
    }
}
