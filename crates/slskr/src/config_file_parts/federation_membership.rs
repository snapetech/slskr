use super::*;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(in crate::config) struct SocialFederationFileConfig {
    #[serde(alias = "Enabled")]
    pub(in crate::config) enabled: Option<bool>,
    #[serde(alias = "Mode")]
    pub(in crate::config) mode: Option<String>,
    #[serde(alias = "Domain")]
    pub(in crate::config) domain: Option<String>,
    #[serde(alias = "BaseUrl", alias = "base_url")]
    pub(in crate::config) base_url: Option<String>,
    #[serde(alias = "ApprovedPeers", alias = "approved_peers")]
    pub(in crate::config) approved_peers: Vec<String>,
    #[serde(alias = "OutboxMaxActivities", alias = "outbox_max_activities")]
    pub(in crate::config) outbox_max_activities: Option<u32>,
    #[serde(alias = "PageSize", alias = "page_size")]
    pub(in crate::config) page_size: Option<u32>,
    #[serde(alias = "VerifySignatures", alias = "verify_signatures")]
    pub(in crate::config) verify_signatures: Option<bool>,
    #[serde(alias = "HttpTimeoutSeconds", alias = "http_timeout_seconds")]
    pub(in crate::config) http_timeout_seconds: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(in crate::config) struct FederationPublishingFileConfig {
    #[serde(alias = "Enabled")]
    pub(in crate::config) enabled: Option<bool>,
    #[serde(alias = "PublishableDomains", alias = "publishable_domains")]
    pub(in crate::config) publishable_domains: Vec<String>,
    #[serde(alias = "DefaultVisibility", alias = "default_visibility")]
    pub(in crate::config) default_visibility: Option<String>,
    #[serde(alias = "ApprovedCircles", alias = "approved_circles")]
    pub(in crate::config) approved_circles: Vec<String>,
    #[serde(
        alias = "RequireModerationApproval",
        alias = "require_moderation_approval"
    )]
    pub(in crate::config) require_moderation_approval: Option<bool>,
    #[serde(alias = "IncludeExternalLinks", alias = "include_external_links")]
    pub(in crate::config) include_external_links: Option<bool>,
    #[serde(alias = "MaxMetadataSizeKb", alias = "max_metadata_size_kb")]
    pub(in crate::config) max_metadata_size_kb: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RealmSettings {
    pub id: String,
    pub governance_roots: Vec<String>,
    pub bootstrap_nodes: Vec<String>,
    pub gossip_enabled: bool,
    pub replication_enabled: bool,
    pub max_gossip_hops: u32,
    pub gossip_interval_seconds: u64,
    pub federation_allowed: bool,
}

impl RealmSettings {
    pub(in crate::config) fn from_layers<E: ConfigEnv>(
        file: RealmFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let id = env
            .var("SLSKR_REALM_ID")
            .or_else(|| env.var("SLSKD_REALM_ID"))
            .or(file.id)
            .unwrap_or_else(|| DEFAULT_REALM_ID.to_owned())
            .trim()
            .to_owned();
        if id.len() < 3
            || id.len() > 64
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            || id.starts_with('.')
            || id.ends_with('.')
            || id.contains("..")
        {
            return Err(
                "realm.id must be 3-64 characters of letters, numbers, hyphens, underscores, or periods and may not contain consecutive periods".to_owned(),
            );
        }

        let governance_roots = env
            .var("SLSKR_REALM_GOVERNANCE_ROOTS")
            .or_else(|| env.var("SLSKD_REALM_GOVERNANCE_ROOTS"))
            .map(|roots| {
                roots
                    .split(',')
                    .map(str::trim)
                    .filter(|root| !root.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or(file.governance_roots);
        if governance_roots.is_empty() || governance_roots.iter().any(|root| root.trim().is_empty())
        {
            return Err(
                "realm.governanceRoots must contain at least one non-empty root".to_owned(),
            );
        }
        let bootstrap_nodes = file
            .bootstrap_nodes
            .into_iter()
            .map(|node| node.trim().to_owned())
            .collect::<Vec<_>>();
        if bootstrap_nodes.iter().any(String::is_empty) {
            return Err("realm.bootstrapNodes cannot contain empty entries".to_owned());
        }
        if !(1..=10).contains(&file.policies.max_gossip_hops) {
            return Err("realm.policies.maxGossipHops must be between 1 and 10".to_owned());
        }
        if !(30..=3600).contains(&file.policies.gossip_interval_seconds) {
            return Err(
                "realm.policies.gossipIntervalSeconds must be between 30 and 3600".to_owned(),
            );
        }
        Ok(Self {
            id,
            governance_roots,
            bootstrap_nodes,
            gossip_enabled: file.policies.gossip_enabled,
            replication_enabled: file.policies.replication_enabled,
            max_gossip_hops: file.policies.max_gossip_hops,
            gossip_interval_seconds: file.policies.gossip_interval_seconds,
            federation_allowed: file.policies.federation_allowed,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(in crate::config) struct RealmFileConfig {
    pub(in crate::config) id: Option<String>,
    #[serde(alias = "governance_roots")]
    pub(in crate::config) governance_roots: Vec<String>,
    #[serde(alias = "bootstrap_nodes")]
    pub(in crate::config) bootstrap_nodes: Vec<String>,
    pub(in crate::config) policies: RealmPoliciesFileConfig,
}

impl Default for RealmFileConfig {
    fn default() -> Self {
        Self {
            id: Some(DEFAULT_REALM_ID.to_owned()),
            governance_roots: vec![DEFAULT_GOVERNANCE_ROOT.to_owned()],
            bootstrap_nodes: Vec::new(),
            policies: RealmPoliciesFileConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(in crate::config) struct RealmPoliciesFileConfig {
    pub(in crate::config) gossip_enabled: bool,
    pub(in crate::config) replication_enabled: bool,
    pub(in crate::config) max_gossip_hops: u32,
    pub(in crate::config) gossip_interval_seconds: u64,
    pub(in crate::config) federation_allowed: bool,
}

impl Default for RealmPoliciesFileConfig {
    fn default() -> Self {
        Self {
            gossip_enabled: true,
            replication_enabled: true,
            max_gossip_hops: 3,
            gossip_interval_seconds: 300,
            federation_allowed: true,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(in crate::config) struct MultiRealmFileConfig {
    pub(in crate::config) realms: Vec<RealmFileConfig>,
    pub(in crate::config) bridge: MultiRealmBridgeFileConfig,
    pub(in crate::config) is_bridging_enabled: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(in crate::config) struct MultiRealmBridgeFileConfig {
    pub(in crate::config) enabled: bool,
    pub(in crate::config) allowed_flows: Vec<String>,
    pub(in crate::config) disallowed_flows: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PodCoreFileConfig {
    #[serde(alias = "Join")]
    pub(in crate::config) join: PodJoinFileConfig,
    #[serde(alias = "Security")]
    pub(in crate::config) security: PodSecurityFileConfig,
    #[serde(
        rename = "GoldStarClub",
        alias = "goldStarClub",
        alias = "gold_star_club"
    )]
    pub(in crate::config) gold_star_club: GoldStarClubFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PodJoinFileConfig {
    #[serde(alias = "SignatureMode")]
    pub(in crate::config) signature_mode: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PodSecurityFileConfig {
    #[serde(alias = "SignatureMode")]
    pub(in crate::config) signature_mode: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GoldStarClubFileConfig {
    #[serde(alias = "AutoJoin", alias = "autoJoin", alias = "auto_join")]
    pub(in crate::config) autojoin: Option<bool>,
}
