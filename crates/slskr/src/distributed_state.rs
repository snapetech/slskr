use std::collections::BTreeMap;

use tokio::sync::mpsc;

use super::{ControllerProfile, DistributedMessage};

#[derive(Debug)]
pub(super) struct DistributedRuntime {
    pub(super) local_username: String,
    pub(super) branch_level: u32,
    pub(super) branch_root: String,
    pub(super) parent: Option<String>,
    pub(super) parent_sender: Option<mpsc::Sender<DistributedMessage>>,
    pub(super) children: BTreeMap<String, mpsc::Sender<DistributedMessage>>,
    pub(super) child_depths: BTreeMap<String, u32>,
    pub(super) persistence_revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DistributedPersistenceSnapshot {
    pub(super) revision: u64,
    pub(super) branch_level: u32,
    pub(super) branch_root: String,
    pub(super) parent_username: Option<String>,
    pub(super) children: Vec<(String, u32)>,
}

#[derive(Clone, Debug)]
pub(super) struct DistributedPersistenceStatus {
    pub(super) revision: u64,
    pub(super) result: Result<(), String>,
}

impl DistributedRuntime {
    pub(super) fn new(username: Option<&str>) -> Self {
        let local_username = username.unwrap_or_default().trim().to_owned();
        Self {
            branch_level: 0,
            branch_root: local_username.clone(),
            local_username,
            parent: None,
            parent_sender: None,
            children: BTreeMap::new(),
            child_depths: BTreeMap::new(),
            persistence_revision: 0,
        }
    }

    pub(super) fn reset_branch(&mut self) {
        self.parent = None;
        self.parent_sender = None;
        self.branch_level = 0;
        self.branch_root.clone_from(&self.local_username);
    }

    pub(super) fn child_depth(&self) -> u32 {
        self.child_depths
            .values()
            .copied()
            .map(|depth| depth.saturating_add(1))
            .max()
            .unwrap_or(0)
    }

    pub(super) fn json(
        &self,
        settings: crate::config::SoulseekDistributedSettings,
    ) -> serde_json::Value {
        let mut children = self.children.keys().cloned().collect::<Vec<_>>();
        children.sort_unstable();
        serde_json::json!({
            "branchLevel": self.branch_level,
            "branchRoot": self.branch_root,
            "canAcceptChildren": !settings.disabled
                && !settings.disable_children
                && self.children.len() < settings.child_limit,
            "childLimit": settings.child_limit,
            "children": children,
            "hasParent": self.parent.is_some(),
            "isBranchRoot": self.parent.is_none(),
            "parent": self.parent,
        })
    }

    pub(super) fn application_json(
        &self,
        settings: crate::config::SoulseekDistributedSettings,
        connected: bool,
        target: ControllerProfile,
    ) -> serde_json::Value {
        if connected {
            let mut value = self.json(settings);
            if target == ControllerProfile::Native && self.parent.is_none() {
                value["parent"] = serde_json::json!("");
            } else if self.parent.is_none() {
                value
                    .as_object_mut()
                    .expect("distributed application state must be an object")
                    .remove("parent");
            }
            return value;
        }
        let mut value = serde_json::json!({
            "branchLevel": 0,
            "canAcceptChildren": false,
            "childLimit": 0,
            "children": [],
            "hasParent": false,
            "isBranchRoot": false,
        });
        if target == ControllerProfile::Native {
            value["branchRoot"] = serde_json::json!("");
            value["parent"] = serde_json::json!("");
        }
        value
    }

    pub(super) fn persistence_snapshot(&self) -> DistributedPersistenceSnapshot {
        DistributedPersistenceSnapshot {
            revision: self.persistence_revision,
            branch_level: self.branch_level,
            branch_root: self.branch_root.clone(),
            parent_username: self.parent.clone(),
            children: self
                .child_depths
                .iter()
                .map(|(username, depth)| (username.clone(), *depth))
                .collect(),
        }
    }

    pub(super) async fn load_persisted_state(
        db: &crate::persistence::DatabaseManager,
    ) -> Result<(Option<(u32, String, Option<String>)>, Vec<(String, u32)>), String> {
        db.load_distributed_state()
            .await
            .map_err(|error| format!("failed to load distributed tree state: {error}"))
    }

    pub(super) fn restore_persisted_state(
        &mut self,
        tree_state: Option<(u32, String, Option<String>)>,
        children: Vec<(String, u32)>,
    ) {
        if let Some((branch_level, branch_root, parent_username)) = tree_state {
            self.branch_level = branch_level;
            self.branch_root = branch_root;
            self.parent = parent_username;
        }
        self.child_depths = children.into_iter().collect();
    }
}

impl DistributedPersistenceSnapshot {
    pub(super) async fn save(
        &self,
        db: &crate::persistence::DatabaseManager,
    ) -> Result<(), String> {
        db.save_distributed_state(
            self.branch_level,
            &self.branch_root,
            self.parent_username.as_deref(),
            &self.children,
        )
        .await
        .map_err(|error| format!("failed to save distributed tree state: {error}"))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DistributedConnectionRole {
    Parent,
    Child,
}
