use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum PodSignatureMode {
    Off,
    Warn,
    Enforce,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SocialFederationSettings {
    pub enabled: bool,
    pub mode: String,
    pub domain: Option<String>,
    pub base_url: Option<String>,
    pub approved_peers: Vec<String>,
    pub outbox_max_activities: u32,
    pub page_size: u32,
    pub verify_signatures: bool,
    pub http_timeout_seconds: u32,
}

impl SocialFederationSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        file: SocialFederationFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let page_size = env_parse_any_layer(
            env,
            &["FEDERATION_PAGE_SIZE", "SLSKR_FEDERATION_PAGE_SIZE"],
            file.page_size,
            20_u32,
        )?;
        if !(10..=100).contains(&page_size) {
            return Err("federation.page_size must be between 10 and 100".to_owned());
        }
        let outbox_max_activities = env_parse_any_layer(
            env,
            &[
                "FEDERATION_OUTBOX_MAX_ACTIVITIES",
                "SLSKR_FEDERATION_OUTBOX_MAX_ACTIVITIES",
            ],
            file.outbox_max_activities,
            100_u32,
        )?;
        if !(10..=1_000).contains(&outbox_max_activities) {
            return Err("federation.outbox_max_activities must be between 10 and 1000".to_owned());
        }
        let http_timeout_seconds = env_parse_any_layer(
            env,
            &[
                "FEDERATION_HTTP_TIMEOUT_SECONDS",
                "SLSKR_FEDERATION_HTTP_TIMEOUT_SECONDS",
            ],
            file.http_timeout_seconds,
            30_u32,
        )?;
        if !(5..=120).contains(&http_timeout_seconds) {
            return Err("federation.http_timeout_seconds must be between 5 and 120".to_owned());
        }
        let mode = env
            .var("FEDERATION_MODE")
            .or_else(|| env.var("SLSKR_FEDERATION_MODE"))
            .or(file.mode)
            .unwrap_or_else(|| "Hermit".to_owned());
        let mode = mode.trim().to_owned();
        let domain = env
            .var("FEDERATION_DOMAIN")
            .or_else(|| env.var("SLSKR_FEDERATION_DOMAIN"))
            .or(file.domain)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let base_url = env
            .var("FEDERATION_BASE_URL")
            .or_else(|| env.var("SLSKR_FEDERATION_BASE_URL"))
            .or(file.base_url)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        Ok(Self {
            enabled: env_bool_any_layer(
                env,
                &["FEDERATION_ENABLED", "SLSKR_FEDERATION_ENABLED"],
                file.enabled.unwrap_or(false),
            )?,
            mode,
            domain,
            base_url,
            approved_peers: string_array_any_layer(
                env,
                &[
                    "FEDERATION_APPROVED_PEERS",
                    "SLSKR_FEDERATION_APPROVED_PEERS",
                ],
                file.approved_peers,
            ),
            outbox_max_activities,
            page_size,
            verify_signatures: env_bool_any_layer(
                env,
                &[
                    "FEDERATION_VERIFY_SIGNATURES",
                    "SLSKR_FEDERATION_VERIFY_SIGNATURES",
                ],
                file.verify_signatures.unwrap_or(true),
            )?,
            http_timeout_seconds,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FederationPublishingSettings {
    pub enabled: bool,
    pub publishable_domains: Vec<String>,
    pub default_visibility: String,
    pub approved_circles: Vec<String>,
    pub require_moderation_approval: bool,
    pub include_external_links: bool,
    pub max_metadata_size_kb: u32,
}

impl FederationPublishingSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        file: FederationPublishingFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let max_metadata_size_kb = env_parse_any_layer(
            env,
            &[
                "FEDERATION_PUBLISHING_MAX_METADATA_SIZE_KB",
                "SLSKR_FEDERATION_PUBLISHING_MAX_METADATA_SIZE_KB",
            ],
            file.max_metadata_size_kb,
            10_u32,
        )?;
        if !(1..=100).contains(&max_metadata_size_kb) {
            return Err(
                "federation_publishing.max_metadata_size_kb must be between 1 and 100".to_owned(),
            );
        }
        let default_visibility = env
            .var("FEDERATION_PUBLISHING_DEFAULT_VISIBILITY")
            .or_else(|| env.var("SLSKR_FEDERATION_PUBLISHING_DEFAULT_VISIBILITY"))
            .or(file.default_visibility)
            .unwrap_or_else(|| "public".to_owned())
            .trim()
            .to_owned();
        Ok(Self {
            enabled: env_bool_any_layer(
                env,
                &[
                    "FEDERATION_PUBLISHING_ENABLED",
                    "SLSKR_FEDERATION_PUBLISHING_ENABLED",
                ],
                file.enabled.unwrap_or(false),
            )?,
            publishable_domains: string_array_any_layer(
                env,
                &[
                    "FEDERATION_PUBLISHING_PUBLISHABLE_DOMAINS",
                    "SLSKR_FEDERATION_PUBLISHING_PUBLISHABLE_DOMAINS",
                ],
                if file.publishable_domains.is_empty() {
                    vec!["music".to_owned()]
                } else {
                    file.publishable_domains
                },
            ),
            default_visibility,
            approved_circles: string_array_any_layer(
                env,
                &[
                    "FEDERATION_PUBLISHING_APPROVED_CIRCLES",
                    "SLSKR_FEDERATION_PUBLISHING_APPROVED_CIRCLES",
                ],
                file.approved_circles,
            ),
            require_moderation_approval: env_bool_any_layer(
                env,
                &[
                    "FEDERATION_PUBLISHING_REQUIRE_MODERATION",
                    "SLSKR_FEDERATION_PUBLISHING_REQUIRE_MODERATION",
                ],
                file.require_moderation_approval.unwrap_or(true),
            )?,
            include_external_links: env_bool_any_layer(
                env,
                &[
                    "FEDERATION_PUBLISHING_INCLUDE_EXTERNAL_LINKS",
                    "SLSKR_FEDERATION_PUBLISHING_INCLUDE_EXTERNAL_LINKS",
                ],
                file.include_external_links.unwrap_or(true),
            )?,
            max_metadata_size_kb,
        })
    }
}

impl PodSignatureMode {
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "warn" => Ok(Self::Warn),
            "enforce" => Ok(Self::Enforce),
            _ => Err("SLSKR_POD_JOIN_SIGNATURE_MODE must be off, warn, or enforce".to_owned()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Warn => "warn",
            Self::Enforce => "enforce",
        }
    }
}
