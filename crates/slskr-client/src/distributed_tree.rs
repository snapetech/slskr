use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use slskr_protocol::{
    distributed::{DistributedMessage, DistributedSearch},
    server::{PossibleParent, ServerMessage},
};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{server::ServerSession, stream::DistributedConnection, ClientError};

pub const DEFAULT_MAX_DISTRIBUTED_CHILDREN: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentInfo {
    pub username: String,
    pub ip: std::net::Ipv4Addr,
    pub port: u32,
}

impl ParentInfo {
    #[must_use]
    pub fn from_possible_parent(parent: &PossibleParent) -> Self {
        Self {
            username: parent.username.clone(),
            ip: parent.ip,
            port: parent.port,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildInfo {
    pub username: String,
    pub depth: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistributedEvent {
    Ping,
    Search(DistributedSearch),
    BranchChanged,
    Ignored,
}

#[derive(Debug)]
struct ParentConnection<S> {
    info: ParentInfo,
    connection: DistributedConnection<S>,
}

#[derive(Debug)]
struct ChildConnection<S> {
    info: ChildInfo,
    connection: DistributedConnection<S>,
}

#[derive(Debug)]
pub struct DistributedTree<S> {
    local_username: String,
    branch_level: u32,
    branch_root: String,
    accepting_children: bool,
    parent: Option<ParentConnection<S>>,
    children: HashMap<String, ChildConnection<S>>,
    max_children: usize,
}

impl<S> DistributedTree<S> {
    #[must_use]
    pub fn new(local_username: impl Into<String>) -> Self {
        Self::with_max_children(local_username, DEFAULT_MAX_DISTRIBUTED_CHILDREN)
    }

    #[must_use]
    pub fn with_max_children(local_username: impl Into<String>, max_children: usize) -> Self {
        let local_username = local_username.into();
        Self {
            branch_level: 0,
            branch_root: local_username.clone(),
            local_username,
            accepting_children: false,
            parent: None,
            children: HashMap::new(),
            max_children: max_children.max(1),
        }
    }

    #[must_use]
    pub fn local_username(&self) -> &str {
        &self.local_username
    }

    #[must_use]
    pub const fn branch_level(&self) -> u32 {
        self.branch_level
    }

    #[must_use]
    pub fn branch_root(&self) -> &str {
        &self.branch_root
    }

    #[must_use]
    pub const fn accepting_children(&self) -> bool {
        self.accepting_children
    }

    #[must_use]
    pub fn parent(&self) -> Option<&ParentInfo> {
        self.parent.as_ref().map(|parent| &parent.info)
    }

    #[must_use]
    pub fn children_len(&self) -> usize {
        self.children.len()
    }

    #[must_use]
    pub fn child_info(&self, username: &str) -> Option<&ChildInfo> {
        self.children.get(username).map(|child| &child.info)
    }

    #[must_use]
    pub fn child_depth(&self) -> u32 {
        self.children
            .values()
            .map(|child| child.info.depth.saturating_add(1))
            .max()
            .unwrap_or(0)
    }

    #[must_use]
    pub fn choose_parent(&self, candidates: &[PossibleParent]) -> Option<ParentInfo> {
        candidates
            .iter()
            .filter(|candidate| candidate.username != self.local_username)
            .filter(|candidate| candidate.port != 0)
            .map(ParentInfo::from_possible_parent)
            .min_by(|left, right| {
                left.username
                    .cmp(&right.username)
                    .then_with(|| left.ip.octets().cmp(&right.ip.octets()))
                    .then_with(|| left.port.cmp(&right.port))
            })
    }

    pub fn connect_parent(&mut self, info: ParentInfo, connection: DistributedConnection<S>) {
        self.branch_level = 1;
        self.branch_root = info.username.clone();
        self.parent = Some(ParentConnection { info, connection });
    }

    pub fn disconnect_parent(&mut self) {
        self.parent = None;
        self.branch_level = 0;
        self.branch_root = self.local_username.clone();
    }

    pub fn reset(&mut self) {
        self.disconnect_parent();
        self.children.clear();
        self.accepting_children = false;
    }

    pub fn set_accepting_children(&mut self, accept: bool) -> ServerMessage {
        self.accepting_children = accept;
        ServerMessage::AcceptChildren { accept }
    }

    #[must_use]
    pub fn have_no_parent_message(&self) -> ServerMessage {
        ServerMessage::HaveNoParent {
            no_parent: self.parent.is_none(),
        }
    }

    #[must_use]
    pub fn branch_server_messages(&self) -> [ServerMessage; 2] {
        [
            ServerMessage::BranchLevel {
                level: self.branch_level,
            },
            ServerMessage::BranchRoot {
                username: self.branch_root.clone(),
            },
        ]
    }

    #[must_use]
    pub fn parent_branch_messages(&self) -> [DistributedMessage; 3] {
        [
            DistributedMessage::BranchLevel {
                level: self.branch_level,
            },
            DistributedMessage::BranchRoot {
                username: self.branch_root.clone(),
            },
            DistributedMessage::ChildDepth {
                depth: self.child_depth(),
            },
        ]
    }

    pub fn add_child(
        &mut self,
        username: impl Into<String>,
        connection: DistributedConnection<S>,
    ) -> Result<bool, ClientError> {
        let username = username.into();
        if self.children.len() >= self.max_children && !self.children.contains_key(&username) {
            return Err(ClientError::DistributedChildCapacityFull {
                max: self.max_children,
            });
        }
        let replaced = self.children.insert(
            username.clone(),
            ChildConnection {
                info: ChildInfo { username, depth: 0 },
                connection,
            },
        );
        Ok(replaced.is_some())
    }

    pub fn remove_child(&mut self, username: &str) -> Option<ChildInfo> {
        self.children.remove(username).map(|child| child.info)
    }

    pub fn handle_parent_message(&mut self, message: DistributedMessage) -> DistributedEvent {
        match message {
            DistributedMessage::Ping => DistributedEvent::Ping,
            DistributedMessage::Search(search) => DistributedEvent::Search(search),
            DistributedMessage::BranchLevel { level } => {
                self.branch_level = level.saturating_add(1);
                DistributedEvent::BranchChanged
            }
            DistributedMessage::BranchRoot { username } => {
                self.branch_root = username;
                DistributedEvent::BranchChanged
            }
            DistributedMessage::ChildDepth { .. }
            | DistributedMessage::EmbeddedMessage { .. }
            | DistributedMessage::EmbeddedServerMessage(_)
            | DistributedMessage::Unknown { .. } => DistributedEvent::Ignored,
        }
    }

    pub fn handle_child_message(
        &mut self,
        username: &str,
        message: DistributedMessage,
    ) -> DistributedEvent {
        match message {
            DistributedMessage::Ping => DistributedEvent::Ping,
            DistributedMessage::Search(search) => DistributedEvent::Search(search),
            DistributedMessage::ChildDepth { depth } => {
                if let Some(child) = self.children.get_mut(username) {
                    child.info.depth = depth;
                    DistributedEvent::BranchChanged
                } else {
                    DistributedEvent::Ignored
                }
            }
            DistributedMessage::BranchLevel { .. }
            | DistributedMessage::BranchRoot { .. }
            | DistributedMessage::EmbeddedMessage { .. }
            | DistributedMessage::EmbeddedServerMessage(_)
            | DistributedMessage::Unknown { .. } => DistributedEvent::Ignored,
        }
    }
}

impl<S> DistributedTree<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn send_branch_info_to_parent(&mut self) -> Result<bool, ClientError> {
        let messages = self.parent_branch_messages();
        let Some(parent) = self.parent.as_mut() else {
            return Ok(false);
        };

        for message in messages {
            parent.connection.send(&message).await?;
        }
        Ok(true)
    }

    pub async fn forward_search_to_children(
        &mut self,
        search: &DistributedSearch,
        except_username: Option<&str>,
    ) -> Result<usize, ClientError> {
        let mut sent = 0;
        for (username, child) in &mut self.children {
            if except_username == Some(username.as_str()) {
                continue;
            }
            child
                .connection
                .send(&DistributedMessage::Search(search.clone()))
                .await?;
            sent += 1;
        }
        Ok(sent)
    }

    pub async fn send_branch_info_to_server<T>(
        &self,
        server: &mut ServerSession<T>,
    ) -> Result<(), ClientError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        for message in self.branch_server_messages() {
            server.send_server_message(message).await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchInfoReporter {
    interval: Duration,
    next_due: Instant,
}

impl BranchInfoReporter {
    #[must_use]
    pub fn new(interval: Duration, now: Instant) -> Self {
        Self {
            interval,
            next_due: now + interval,
        }
    }

    #[must_use]
    pub const fn next_due(&self) -> Instant {
        self.next_due
    }

    #[must_use]
    pub fn due_messages<S>(
        &mut self,
        now: Instant,
        tree: &DistributedTree<S>,
    ) -> Option<[ServerMessage; 2]> {
        if now < self.next_due {
            return None;
        }

        while self.next_due <= now {
            self.next_due += self.interval;
        }
        Some(tree.branch_server_messages())
    }
}
