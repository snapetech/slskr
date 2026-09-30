use super::*;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::config) struct TrustedMeshPeerInput {
    #[serde(alias = "peerId")]
    pub(in crate::config) peer_id: String,
    pub(in crate::config) username: String,
    #[serde(alias = "overlayEndpoint")]
    pub(in crate::config) overlay_endpoint: String,
    #[serde(alias = "certificateSha256")]
    pub(in crate::config) certificate_sha256: String,
    #[serde(default, alias = "rangeEndpoint")]
    pub(in crate::config) range_endpoint: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkFileConfig {
    pub(in crate::config) server_address: Option<String>,
    pub(in crate::config) listen_port: Option<u32>,
    pub(in crate::config) username: Option<String>,
    pub(in crate::config) password: Option<String>,
    pub(in crate::config) credential_store: Option<String>,
    pub(in crate::config) credential_file: Option<PathBuf>,
    pub(in crate::config) private_message_auto_response: PrivateMessageAutoResponseFileConfig,
    pub(in crate::config) obfuscation: SoulseekObfuscationFileConfig,
    pub(in crate::config) connection: SoulseekConnectionFileConfig,
    pub(in crate::config) distributed_network: SoulseekDistributedFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekDistributedFileConfig {
    pub(in crate::config) disabled: Option<bool>,
    pub(in crate::config) disable_children: Option<bool>,
    pub(in crate::config) child_limit: Option<usize>,
    pub(in crate::config) logging: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekConnectionFileConfig {
    pub(in crate::config) timeout: SoulseekConnectionTimeoutFileConfig,
    pub(in crate::config) buffer: SoulseekConnectionBufferFileConfig,
    pub(in crate::config) proxy: SoulseekProxyFileConfig,
    pub(in crate::config) auto_acknowledge_private_messages: Option<bool>,
    pub(in crate::config) auto_acknowledge_privilege_notifications: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekConnectionTimeoutFileConfig {
    pub(in crate::config) connect: Option<u64>,
    pub(in crate::config) inactivity: Option<u64>,
    pub(in crate::config) transfer: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekConnectionBufferFileConfig {
    pub(in crate::config) read: Option<usize>,
    pub(in crate::config) write: Option<usize>,
    pub(in crate::config) transfer: Option<usize>,
    pub(in crate::config) write_queue: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekProxyFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) address: Option<String>,
    pub(in crate::config) port: Option<u16>,
    pub(in crate::config) username: Option<String>,
    pub(in crate::config) password: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekObfuscationFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) mode: Option<String>,
    pub(in crate::config) advertise_regular_port: Option<bool>,
    pub(in crate::config) prefer_outbound: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PrivateMessageAutoResponseFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) message: Option<String>,
    pub(in crate::config) cooldown_minutes: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ListenerFileConfig {
    pub(in crate::config) regular_bind: Option<String>,
    pub(in crate::config) advertised_port: Option<u32>,
    pub(in crate::config) obfuscated_bind: Option<String>,
    pub(in crate::config) obfuscated_advertised_port: Option<u32>,
    pub(in crate::config) overlay_bind: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProfileFileConfig {
    pub(in crate::config) user_info_description: Option<String>,
    pub(in crate::config) user_info_picture: Option<String>,
    pub(in crate::config) soulseek_diagnostic_level: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TimeoutFileConfig {
    pub(in crate::config) peer_response_seconds: Option<u64>,
}

impl SoulseekConnectionSettings {
    pub(in crate::config) fn from_layers<E: ConfigEnv>(
        file: SoulseekConnectionFileConfig,
        env: &E,
        target: ControllerProfile,
    ) -> Result<Self, String> {
        let parse_bounded = |name: &str,
                             aliases: &[&str],
                             file_value: Option<usize>,
                             default: usize,
                             minimum: usize,
                             maximum: usize|
         -> Result<usize, String> {
            bounded_config_value(
                name,
                env_parse_any_layer(env, aliases, file_value, default)?,
                minimum,
                maximum,
            )
        };
        let buffer_read = parse_bounded(
            "SLSK_READ_BUFFER",
            &[
                "SLSKR_SLSK_READ_BUFFER",
                "SLSKD_SLSK_READ_BUFFER",
                "SLSK_READ_BUFFER",
            ],
            file.buffer.read,
            16_384,
            1_024,
            i32::MAX as usize,
        )?;
        let buffer_write = parse_bounded(
            "SLSK_WRITE_BUFFER",
            &[
                "SLSKR_SLSK_WRITE_BUFFER",
                "SLSKD_SLSK_WRITE_BUFFER",
                "SLSK_WRITE_BUFFER",
            ],
            file.buffer.write,
            16_384,
            1_024,
            i32::MAX as usize,
        )?;
        let buffer_transfer = parse_bounded(
            "SLSK_TRANSFER_BUFFER",
            &[
                "SLSKR_SLSK_TRANSFER_BUFFER",
                "SLSKD_SLSK_TRANSFER_BUFFER",
                "SLSK_TRANSFER_BUFFER",
            ],
            file.buffer.transfer,
            262_144,
            81_920,
            i32::MAX as usize,
        )?;
        let buffer_write_queue = parse_bounded(
            "SLSK_WRITE_QUEUE",
            &[
                "SLSKR_SLSK_WRITE_QUEUE",
                "SLSKD_SLSK_WRITE_QUEUE",
                "SLSK_WRITE_QUEUE",
            ],
            file.buffer.write_queue,
            50,
            5,
            5_000,
        )?;
        let parse_timeout = |name: &str,
                             aliases: &[&str],
                             file_value: Option<u64>,
                             default: u64,
                             minimum: u64|
         -> Result<Duration, String> {
            bounded_config_value(
                name,
                env_parse_any_layer(env, aliases, file_value, default)?,
                minimum,
                i32::MAX as u64,
            )
            .map(Duration::from_millis)
        };
        let timeout_connect = parse_timeout(
            "SLSK_CONNECTION_TIMEOUT",
            &[
                "SLSKR_SLSK_CONNECTION_TIMEOUT",
                "SLSKD_SLSK_CONNECTION_TIMEOUT",
                "SLSK_CONNECTION_TIMEOUT",
            ],
            file.timeout.connect,
            10_000,
            1_000,
        )?;
        let timeout_inactivity = parse_timeout(
            "SLSK_INACTIVITY_TIMEOUT",
            &[
                "SLSKR_SLSK_INACTIVITY_TIMEOUT",
                "SLSKD_SLSK_INACTIVITY_TIMEOUT",
                "SLSK_INACTIVITY_TIMEOUT",
            ],
            file.timeout.inactivity,
            if target == ControllerProfile::Legacy {
                15_000
            } else {
                60_000
            },
            1_000,
        )?;
        let timeout_transfer = parse_timeout(
            "SLSK_TRANSFER_TIMEOUT",
            &[
                "SLSKR_SLSK_TRANSFER_TIMEOUT",
                "SLSKD_SLSK_TRANSFER_TIMEOUT",
                "SLSK_TRANSFER_TIMEOUT",
            ],
            file.timeout.transfer,
            60_000,
            30_000,
        )?;
        let proxy_enabled = env_bool_any_layer(
            env,
            &[
                "SLSKR_SLSK_PROXY_ENABLED",
                "SLSKD_SLSK_PROXY_ENABLED",
                "SLSK_PROXY_ENABLED",
            ],
            file.proxy.enabled.unwrap_or(false),
        )?;
        let layered_string = |aliases: &[&str], file_value: Option<String>| {
            optional_env_any(env, aliases)
                .or(file_value)
                .unwrap_or_default()
        };
        let proxy_address = layered_string(
            &[
                "SLSKR_SLSK_PROXY_ADDRESS",
                "SLSKD_SLSK_PROXY_ADDRESS",
                "SLSK_PROXY_ADDRESS",
            ],
            file.proxy.address,
        );
        let proxy_username = layered_string(
            &[
                "SLSKR_SLSK_PROXY_USERNAME",
                "SLSKD_SLSK_PROXY_USERNAME",
                "SLSK_PROXY_USERNAME",
            ],
            file.proxy.username,
        );
        let proxy_password = layered_string(
            &[
                "SLSKR_SLSK_PROXY_PASSWORD",
                "SLSKD_SLSK_PROXY_PASSWORD",
                "SLSK_PROXY_PASSWORD",
            ],
            file.proxy.password,
        );
        let proxy_port = optional_env_any(
            env,
            &[
                "SLSKR_SLSK_PROXY_PORT",
                "SLSKD_SLSK_PROXY_PORT",
                "SLSK_PROXY_PORT",
            ],
        )
        .map(|value| {
            value
                .parse::<u16>()
                .map_err(|error| format!("invalid SLSK_PROXY_PORT: {error}"))
        })
        .transpose()?
        .or(file.proxy.port);
        for (field, value) in [
            ("Address", proxy_address.as_str()),
            ("Username", proxy_username.as_str()),
            ("Password", proxy_password.as_str()),
        ] {
            if value.encode_utf16().count() > 255 {
                return Err(format!("Soulseek proxy {field} exceeds 255 characters"));
            }
        }
        if proxy_enabled && proxy_address.trim().is_empty() {
            return Err("Soulseek proxy is enabled but no address is configured".to_owned());
        }
        if proxy_enabled && proxy_port.is_none() {
            return Err("Soulseek proxy is enabled but no port is configured".to_owned());
        }
        let auto_acknowledge_private_messages = env_bool_any_layer(
            env,
            &[
                "SLSKR_SLSK_AUTO_ACKNOWLEDGE_PRIVATE_MESSAGES",
                "SLSKD_SLSK_AUTO_ACKNOWLEDGE_PRIVATE_MESSAGES",
                "SLSK_AUTO_ACKNOWLEDGE_PRIVATE_MESSAGES",
            ],
            file.auto_acknowledge_private_messages.unwrap_or(false),
        )?;
        let auto_acknowledge_privilege_notifications = env_bool_any_layer(
            env,
            &[
                "SLSKR_SLSK_AUTO_ACKNOWLEDGE_PRIVILEGE_NOTIFICATIONS",
                "SLSKD_SLSK_AUTO_ACKNOWLEDGE_PRIVILEGE_NOTIFICATIONS",
                "SLSK_AUTO_ACKNOWLEDGE_PRIVILEGE_NOTIFICATIONS",
            ],
            file.auto_acknowledge_privilege_notifications
                .unwrap_or(false),
        )?;
        Ok(Self {
            buffer_read,
            buffer_write,
            buffer_transfer,
            buffer_write_queue,
            timeout_connect,
            timeout_inactivity,
            timeout_transfer,
            proxy: SoulseekProxySettings {
                enabled: proxy_enabled,
                address: proxy_address,
                port: proxy_port,
                username: proxy_username,
                password: proxy_password,
            },
            auto_acknowledge_private_messages,
            auto_acknowledge_privilege_notifications,
        })
    }
}
