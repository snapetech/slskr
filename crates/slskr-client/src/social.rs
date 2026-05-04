use std::collections::{HashMap, HashSet};

use slskr_protocol::server::{PrivateMessage, ServerMessage, UserStatus, WatchedUser};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserWatchState {
    watched: HashMap<String, WatchedUser>,
    statuses: HashMap<String, UserStatus>,
}

impl UserWatchState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn watch_message(username: impl Into<String>) -> ServerMessage {
        ServerMessage::WatchUserRequest {
            username: username.into(),
        }
    }

    #[must_use]
    pub fn unwatch_message(username: impl Into<String>) -> ServerMessage {
        ServerMessage::UnwatchUser {
            username: username.into(),
        }
    }

    pub fn apply_server_message(&mut self, message: &ServerMessage) -> bool {
        match message {
            ServerMessage::WatchUserResponse(user) => {
                self.watched.insert(user.username.clone(), user.clone());
                true
            }
            ServerMessage::GetUserStatusResponse(status) => {
                self.statuses
                    .insert(status.username.clone(), status.clone());
                true
            }
            ServerMessage::UnwatchUser { username } => {
                self.watched.remove(username);
                self.statuses.remove(username);
                true
            }
            _ => false,
        }
    }

    #[must_use]
    pub fn watched(&self, username: &str) -> Option<&WatchedUser> {
        self.watched.get(username)
    }

    #[must_use]
    pub fn status(&self, username: &str) -> Option<&UserStatus> {
        self.statuses.get(username)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoomState {
    joined: HashSet<String>,
    messages: Vec<RoomMessage>,
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
    pub fn join_global_message() -> ServerMessage {
        ServerMessage::JoinGlobalRoom
    }

    #[must_use]
    pub fn leave_global_message() -> ServerMessage {
        ServerMessage::LeaveGlobalRoom
    }

    #[must_use]
    pub fn leave_room_message(room: impl Into<String>) -> ServerMessage {
        ServerMessage::LeaveRoom { room: room.into() }
    }

    pub fn apply_server_message(&mut self, message: &ServerMessage) -> bool {
        match message {
            ServerMessage::GlobalRoomMessage {
                room,
                username,
                message,
            } => {
                self.joined.insert(room.clone());
                self.messages.push(RoomMessage {
                    room: room.clone(),
                    username: username.clone(),
                    message: message.clone(),
                });
                true
            }
            ServerMessage::LeaveRoom { room } => {
                self.joined.remove(room);
                true
            }
            _ => false,
        }
    }

    #[must_use]
    pub fn is_joined(&self, room: &str) -> bool {
        self.joined.contains(room)
    }

    #[must_use]
    pub fn messages(&self) -> &[RoomMessage] {
        &self.messages
    }
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
                self.messages.push(message.clone());
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
