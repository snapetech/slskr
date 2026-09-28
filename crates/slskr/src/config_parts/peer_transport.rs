use super::*;

pub fn parse_compat_ip_address(value: &str) -> Result<IpAddr, String> {
    let value = value.trim();
    let unbracketed = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(value);
    if let Ok(address) = unbracketed.parse::<IpAddr>() {
        return Ok(address);
    }
    if unbracketed.is_empty() || unbracketed.contains(':') {
        return Err("invalid IPv4 or IPv6 address".to_owned());
    }
    let parts = unbracketed
        .split('.')
        .map(|part| {
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err("invalid IPv4 or IPv6 address".to_owned());
            }
            part.parse::<u32>()
                .map_err(|_| "invalid IPv4 or IPv6 address".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let value = match parts.as_slice() {
        [value] => *value,
        [a, b] if *a <= u8::MAX.into() && *b <= 0x00ff_ffff => (a << 24) | b,
        [a, b, c] if *a <= u8::MAX.into() && *b <= u8::MAX.into() && *c <= u16::MAX.into() => {
            (a << 24) | (b << 16) | c
        }
        [a, b, c, d] if [a, b, c, d].into_iter().all(|part| *part <= u8::MAX.into()) => {
            (a << 24) | (b << 16) | (c << 8) | d
        }
        _ => return Err("invalid IPv4 or IPv6 address".to_owned()),
    };
    Ok(IpAddr::V4(Ipv4Addr::from(value)))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedMeshPeer {
    pub peer_id: String,
    pub username: String,
    pub overlay_endpoint: SocketAddr,
    pub certificate_sha256: [u8; 32],
    pub range_endpoint: Option<String>,
}

impl TrustedMeshPeer {
    pub fn matches(&self, identity: &str) -> bool {
        self.peer_id.eq_ignore_ascii_case(identity) || self.username.eq_ignore_ascii_case(identity)
    }

    pub fn range_url(
        &self,
        expected_hash: &str,
        size: u64,
        recording_id: Option<&str>,
    ) -> Option<String> {
        if expected_hash.len() != 64 || !expected_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return None;
        }
        let endpoint = self.range_endpoint.as_deref()?;
        Some(
            endpoint
                .replace("{sha256}", expected_hash)
                .replace("{size}", &size.to_string())
                .replace(
                    "{recordingId}",
                    &crate::url_encode(recording_id.unwrap_or_default()),
                ),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoulseekObfuscationMode {
    Compatibility,
    Prefer,
}

impl SoulseekObfuscationMode {
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "compatibility" => Ok(Self::Compatibility),
            "prefer" => Ok(Self::Prefer),
            "only" => Err("Soulseek obfuscation only mode is not supported because regular fallback is required for legacy compatibility".to_owned()),
            _ => Err("SLSK_OBFUSCATION_MODE must be compatibility or prefer".to_owned()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compatibility => "compatibility",
            Self::Prefer => "prefer",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PrivateMessageAutoResponseSettings {
    pub enabled: bool,
    pub message: String,
    pub cooldown_minutes: u64,
}

impl PrivateMessageAutoResponseSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        file_config: PrivateMessageAutoResponseFileConfig,
        env: &E,
        default_message: &str,
    ) -> Result<Self, String> {
        let enabled = env_bool_any_layer(
            env,
            &[
                "SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE",
                "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE",
            ],
            file_config.enabled.unwrap_or(false),
        )?;
        let message = optional_env_any(
            env,
            &[
                "SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE",
                "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE",
            ],
        )
        .or(file_config.message)
        .unwrap_or_else(|| default_message.to_owned());
        if message.len() > MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_BYTES {
            return Err(format!(
                "private-message auto response exceeds {MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_BYTES} bytes"
            ));
        }
        let cooldown_minutes = env_parse_any_layer(
            env,
            &[
                "SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES",
                "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES",
            ],
            file_config.cooldown_minutes,
            360_u64,
        )?;
        if !(1..=1_440).contains(&cooldown_minutes) {
            return Err(
                "private-message auto-response cooldown must be between 1 and 1440 minutes"
                    .to_owned(),
            );
        }
        Ok(Self {
            enabled,
            message,
            cooldown_minutes,
        })
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"message_configured\":true,\"cooldown_minutes\":{}}}",
            self.enabled, self.cooldown_minutes
        )
    }
}

pub(super) fn trusted_mesh_peers_from_layers(
    env_value: Option<String>,
    file_value: Vec<TrustedMeshPeerInput>,
) -> Result<Vec<TrustedMeshPeer>, String> {
    let values = match env_value {
        Some(value) => serde_json::from_str::<Vec<TrustedMeshPeerInput>>(&value)
            .map_err(|error| format!("invalid SLSKR_TRUSTED_MESH_PEERS JSON: {error}"))?,
        None => file_value,
    };
    if values.len() > MAX_TRUSTED_MESH_PEERS {
        return Err(format!(
            "trusted mesh peer count exceeds {MAX_TRUSTED_MESH_PEERS}"
        ));
    }

    let mut peers = Vec::with_capacity(values.len());
    for value in values {
        let peer_id = bounded_mesh_identity(&value.peer_id, "peer_id")?;
        let username = bounded_mesh_identity(&value.username, "username")?;
        if peers.iter().any(|peer: &TrustedMeshPeer| {
            peer.peer_id.eq_ignore_ascii_case(&peer_id)
                || peer.username.eq_ignore_ascii_case(&username)
                || peer.peer_id.eq_ignore_ascii_case(&username)
                || peer.username.eq_ignore_ascii_case(&peer_id)
        }) {
            return Err(format!(
                "trusted mesh peer identity {peer_id:?}/{username:?} is duplicated"
            ));
        }
        let overlay_endpoint = value
            .overlay_endpoint
            .trim()
            .parse::<SocketAddr>()
            .map_err(|error| format!("trusted mesh overlay endpoint is invalid: {error}"))?;
        if overlay_endpoint.port() == 0 {
            return Err("trusted mesh overlay endpoint port must be non-zero".to_owned());
        }
        if overlay_endpoint.ip().is_unspecified() || overlay_endpoint.ip().is_multicast() {
            return Err(
                "trusted mesh overlay endpoint must be a unicast destination address".to_owned(),
            );
        }
        let certificate_sha256 = decode_mesh_certificate_pin(&value.certificate_sha256)?;
        let range_endpoint = value
            .range_endpoint
            .as_deref()
            .map(validate_mesh_range_endpoint)
            .transpose()?;
        peers.push(TrustedMeshPeer {
            peer_id,
            username,
            overlay_endpoint,
            certificate_sha256,
            range_endpoint,
        });
    }
    Ok(peers)
}

pub(super) fn bounded_mesh_identity(value: &str, field: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("trusted mesh peer {field} is required"));
    }
    if value.len() > MAX_MESH_IDENTITY_BYTES {
        return Err(format!(
            "trusted mesh peer {field} exceeds {MAX_MESH_IDENTITY_BYTES} bytes"
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(format!(
            "trusted mesh peer {field} contains a control character"
        ));
    }
    Ok(value.to_owned())
}

pub(super) fn decode_mesh_certificate_pin(value: &str) -> Result<[u8; 32], String> {
    let value = value.trim();
    if value.len() != 64 {
        return Err(
            "trusted mesh certificate_sha256 must contain exactly 64 hex digits".to_owned(),
        );
    }
    let bytes = hex::decode(value)
        .map_err(|_| "trusted mesh certificate_sha256 must be hexadecimal".to_owned())?;
    let pin: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "trusted mesh certificate_sha256 must contain 32 bytes".to_owned())?;
    if pin.iter().all(|byte| *byte == 0) {
        return Err("trusted mesh certificate_sha256 must not be all zeroes".to_owned());
    }
    Ok(pin)
}

pub(super) fn validate_mesh_range_endpoint(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("trusted mesh range_endpoint must not be blank".to_owned());
    }
    if value.len() > MAX_MESH_RANGE_ENDPOINT_BYTES {
        return Err(format!(
            "trusted mesh range_endpoint exceeds {MAX_MESH_RANGE_ENDPOINT_BYTES} bytes"
        ));
    }
    if value.chars().any(char::is_control) {
        return Err("trusted mesh range_endpoint contains a control character".to_owned());
    }
    let scheme_end = value
        .find("://")
        .ok_or_else(|| "trusted mesh range_endpoint is missing an authority".to_owned())?;
    let path_start = value[scheme_end + 3..]
        .find('/')
        .map_or(value.len(), |offset| scheme_end + 3 + offset);
    if value[..path_start].contains(['{', '}']) {
        return Err(
            "trusted mesh range_endpoint placeholders are allowed only in the path".to_owned(),
        );
    }
    let parseable = value
        .replace("{sha256}", &"0".repeat(64))
        .replace("{size}", "1")
        .replace("{recordingId}", "recording-id");
    if parseable.contains(['{', '}']) {
        return Err("trusted mesh range_endpoint contains an unknown placeholder".to_owned());
    }
    let url = reqwest::Url::parse(&parseable)
        .map_err(|error| format!("trusted mesh range_endpoint is invalid: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("trusted mesh range_endpoint must use http or https".to_owned());
    }
    if url.host_str().is_none() {
        return Err("trusted mesh range_endpoint must include a host".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("trusted mesh range_endpoint must not contain embedded credentials".to_owned());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("trusted mesh range_endpoint must not contain a query or fragment".to_owned());
    }
    Ok(value.to_owned())
}

pub(super) fn parse_user_endpoint_overrides(
    value: Option<String>,
) -> Result<BTreeMap<String, SocketAddr>, String> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let mut overrides = BTreeMap::new();
    for entry in value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let (username, endpoint) = entry.split_once('=').ok_or_else(|| {
            format!(
                "invalid SLSKR_TEST_USER_ENDPOINT_OVERRIDES entry {entry:?}; expected user=host:port"
            )
        })?;
        let username = username.trim();
        if username.is_empty() {
            return Err("SLSKR_TEST_USER_ENDPOINT_OVERRIDES contains an empty username".to_owned());
        }
        let endpoint = endpoint
            .trim()
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid endpoint override for {username}: {error}"))?;
        overrides.insert(username.to_owned(), endpoint);
    }
    Ok(overrides)
}

pub(super) fn split_server_address(value: &str) -> Result<(String, u16), String> {
    let value = value.trim();
    let (host, port) = if let Some(rest) = value.strip_prefix('[') {
        let (host, suffix) = rest
            .split_once(']')
            .ok_or_else(|| "invalid Soulseek server address: missing closing bracket".to_owned())?;
        let port = suffix.strip_prefix(':').ok_or_else(|| {
            "invalid Soulseek server address: missing port after bracket".to_owned()
        })?;
        (host, port)
    } else {
        value
            .rsplit_once(':')
            .ok_or_else(|| "invalid Soulseek server address: expected host:port".to_owned())?
    };
    if host.trim().is_empty() {
        return Err("invalid Soulseek server address: host is empty".to_owned());
    }
    let port = port
        .parse::<u16>()
        .map_err(|error| format!("invalid Soulseek server port: {error}"))?;
    if port == 0 {
        return Err("invalid Soulseek server port: must be between 1 and 65535".to_owned());
    }
    Ok((host.trim().to_owned(), port))
}

pub(super) fn format_host_port(host: &str, port: u16) -> String {
    let host = host.trim().trim_matches(['[', ']']);
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

pub(super) struct SoulseekIdentity {
    pub(super) server_address: String,
    pub(super) listen_port: u32,
    pub(super) username: Option<String>,
    pub(super) password: Option<String>,
    pub(super) credential_store: CredentialStoreMode,
    pub(super) credential_file: PathBuf,
    pub(super) auto_connect: bool,
}

pub(super) struct PeerProfileSettings {
    pub(super) peer_host_override: Option<Ipv4Addr>,
    pub(super) distributed_parent_override: Option<SocketAddr>,
    pub(super) test_user_endpoint_overrides: BTreeMap<String, SocketAddr>,
    pub(super) user_info_description: String,
    pub(super) user_info_picture: Option<PathBuf>,
    pub(super) soulseek_diagnostic_level: SoulseekDiagnosticLevel,
}

pub(super) fn resolve_peer_profile<E: ConfigEnv>(
    env: &E,
    profile_user_info_description: Option<String>,
    profile_user_info_picture: Option<String>,
    profile_soulseek_diagnostic_level: Option<String>,
) -> Result<PeerProfileSettings, String> {
    let peer_host_override = env
        .var("SLSKR_PEER_HOST_OVERRIDE")
        .map(|value| {
            value
                .parse::<Ipv4Addr>()
                .map_err(|error| format!("invalid SLSKR_PEER_HOST_OVERRIDE: {error}"))
        })
        .transpose()?;
    let distributed_parent_override = env
        .var("SLSKR_DISTRIBUTED_PARENT_OVERRIDE")
        .map(|value| {
            let address = value
                .parse::<SocketAddr>()
                .map_err(|error| format!("invalid SLSKR_DISTRIBUTED_PARENT_OVERRIDE: {error}"))?;
            if address.port() == 0 {
                return Err("SLSKR_DISTRIBUTED_PARENT_OVERRIDE port must be non-zero".to_owned());
            }
            Ok(address)
        })
        .transpose()?;
    let test_user_endpoint_overrides =
        parse_user_endpoint_overrides(env.var("SLSKR_TEST_USER_ENDPOINT_OVERRIDES"))?;
    let user_info_description = optional_env_any(
        env,
        &["SLSKR_USER_INFO_DESCRIPTION", "SLSKD_SLSK_DESCRIPTION"],
    )
    .or(profile_user_info_description)
    .unwrap_or_else(|| "A slskR user. https://github.com/snapetech/slskr".to_owned());
    let user_info_picture = optional_env_any(
        env,
        &[
            "SLSKR_USER_INFO_PICTURE",
            "SLSKD_SLSK_PICTURE",
            "SLSK_PICTURE",
        ],
    )
    .or(profile_user_info_picture)
    .filter(|value| !value.is_empty())
    .map(PathBuf::from);
    if let Some(path) = user_info_picture.as_deref() {
        let metadata = fs::metadata(path).map_err(|error| {
            format!(
                "Soulseek picture '{}' is not readable: {error}",
                path.display()
            )
        })?;
        if !metadata.is_file() {
            return Err(format!(
                "Soulseek picture '{}' is not a regular file",
                path.display()
            ));
        }
        fs::File::open(path).map_err(|error| {
            format!(
                "Soulseek picture '{}' is not readable: {error}",
                path.display()
            )
        })?;
    }
    let soulseek_diagnostic_level = SoulseekDiagnosticLevel::parse(
        optional_env_any(
            env,
            &[
                "SLSKR_SLSK_DIAG_LEVEL",
                "SLSKD_SLSK_DIAG_LEVEL",
                "SLSK_DIAG_LEVEL",
            ],
        )
        .or(profile_soulseek_diagnostic_level)
        .as_deref()
        .unwrap_or("info"),
    )?;
    Ok(PeerProfileSettings {
        peer_host_override,
        distributed_parent_override,
        test_user_endpoint_overrides,
        user_info_description,
        user_info_picture,
        soulseek_diagnostic_level,
    })
}

pub(super) struct ListenerAndObfuscationResolution {
    pub(super) listener_bind: Option<String>,
    pub(super) advertised_port: u32,
    pub(super) obfuscated_listener_bind: Option<String>,
    pub(super) obfuscated_advertised_port: Option<u32>,
    pub(super) overlay_bind: Option<SocketAddr>,
    pub(super) dht_enabled: bool,
    pub(super) dht_port: u16,
    pub(super) trusted_mesh_peers: Vec<TrustedMeshPeer>,
    pub(super) obfuscation_enabled: bool,
    pub(super) obfuscation_mode: SoulseekObfuscationMode,
    pub(super) obfuscation_listen_port: u32,
    pub(super) obfuscation_advertise_regular_port: bool,
    pub(super) obfuscation_prefer_outbound: bool,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_listener_and_obfuscation<E: ConfigEnv>(
    env: &E,
    controller_profile: ControllerProfile,
    current_upstream_behavior: bool,
    advanced_networking: &AdvancedNetworkingSettings,
    listen_port: u32,
    auto_connect: bool,
    listeners_regular_bind: Option<String>,
    listeners_advertised_port: Option<u32>,
    listeners_obfuscated_bind: Option<String>,
    listeners_obfuscated_advertised_port: Option<u32>,
    listeners_overlay_bind: Option<String>,
    mesh_trusted_peers: Vec<TrustedMeshPeerInput>,
    obfuscation_enabled_file: Option<bool>,
    obfuscation_mode_file: Option<String>,
    obfuscation_advertise_regular_port_file: Option<bool>,
    obfuscation_prefer_outbound_file: Option<bool>,
) -> Result<ListenerAndObfuscationResolution, String> {
    let listener_bind = optional_env_any(
        env,
        &["SLSKR_LISTENER_BIND", "SLSKD_SLSK_LISTEN_IP_ADDRESS"],
    )
    .map(|value| {
        if env.var("SLSKR_LISTENER_BIND").is_none() && value.parse::<IpAddr>().is_ok() {
            format_host_port(&value, u16::try_from(listen_port).unwrap_or(u16::MAX))
        } else {
            value
        }
    })
    .or(listeners_regular_bind)
    .or_else(|| {
        Some(
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                u16::try_from(listen_port).unwrap_or(u16::MAX),
            )
            .to_string(),
        )
    });
    if env.var("SLSKR_LISTENER_BIND").is_none() {
        if let Some(address) = env.var("SLSKD_SLSK_LISTEN_IP_ADDRESS") {
            address.parse::<IpAddr>().map_err(|_| {
                "Soulseek.ListenIpAddress specifies an invalid IPv4 or IPv6 IP address".to_owned()
            })?;
        }
    }
    if controller_profile == ControllerProfile::Native
        && auto_connect
        && listener_bind.as_deref().is_some_and(|value| {
            value
                .parse::<SocketAddr>()
                .map(|address| address.ip().is_loopback())
                .or_else(|_| value.parse::<IpAddr>().map(|address| address.is_loopback()))
                .unwrap_or(false)
        })
    {
        return Err(
            "Soulseek.ListenIpAddress must not be a loopback address when the client is connecting. Use 0.0.0.0 or a reachable LAN/VPN interface instead."
                .to_owned(),
        );
    }
    let advertised_port = env_parse_layer(
        env,
        "SLSKR_ADVERTISED_PORT",
        listeners_advertised_port,
        listen_port,
    )?;
    // The legacy profile has no Soulseek type-1 obfuscation option or listener.
    // The fields are accepted by the shared configuration model because they
    // are part of the native profile, but they must not silently turn the
    // legacy profile into a different network endpoint. Ignore those native
    // profile-only layers for the legacy profile and keep the runtime projection
    // disabled below.
    let supports_soulseek_obfuscation = controller_profile == ControllerProfile::Native;
    let upstream_obfuscated_port = if supports_soulseek_obfuscation {
        env_parse_any_option(env, &["SLSKD_SLSK_OBFUSCATION_LISTEN_PORT"])?
    } else {
        None
    };
    let mut obfuscated_listener_bind = if supports_soulseek_obfuscation {
        // Current upstream uses a zero obfuscation port as the shared-listener
        // sentinel. Frozen slskdN compatibility retains the historical
        // adjacent dedicated listener when no explicit obfuscation bind was
        // configured.
        env.var("SLSKR_OBFUSCATED_LISTENER_BIND")
            .or(listeners_obfuscated_bind)
            .or_else(|| {
                upstream_obfuscated_port
                    .filter(|port| *port != 0)
                    .map(|port| {
                        let host = listener_bind
                            .as_deref()
                            .and_then(|value| value.parse::<SocketAddr>().ok())
                            .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |bind| bind.ip());
                        SocketAddr::new(host, port).to_string()
                    })
            })
            .or_else(|| {
                (!current_upstream_behavior && listen_port < 65_535).then(|| {
                    let host = listener_bind
                        .as_deref()
                        .and_then(|value| value.parse::<SocketAddr>().ok())
                        .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |bind| bind.ip());
                    SocketAddr::new(host, (listen_port + 1) as u16).to_string()
                })
            })
    } else {
        None
    };
    if supports_soulseek_obfuscation && current_upstream_behavior {
        if let Some(obfuscated) = obfuscated_listener_bind.as_deref() {
            if Some(obfuscated) != listener_bind.as_deref() {
                return Err(
                    "native/current obfuscation must share the Soulseek listener bind".to_owned(),
                );
            }
            obfuscated_listener_bind = None;
        }
    }
    let obfuscated_advertised_port = if supports_soulseek_obfuscation {
        if env.var("SLSKR_OBFUSCATED_ADVERTISED_PORT").is_some() {
            env_parse_option_layer(
                env,
                "SLSKR_OBFUSCATED_ADVERTISED_PORT",
                listeners_obfuscated_advertised_port,
            )?
        } else {
            upstream_obfuscated_port
                .filter(|port| *port != 0)
                .map(u32::from)
                .or(listeners_obfuscated_advertised_port)
                .or_else(|| {
                    obfuscated_listener_bind
                        .as_deref()
                        .and_then(|value| value.parse::<SocketAddr>().ok())
                        .map(|address| u32::from(address.port()))
                })
                .or_else(|| {
                    if current_upstream_behavior {
                        Some(advertised_port)
                    } else {
                        (listen_port < 65_535).then_some(listen_port + 1)
                    }
                })
        }
    } else {
        None
    };
    if supports_soulseek_obfuscation
        && current_upstream_behavior
        && obfuscated_advertised_port != Some(advertised_port)
    {
        return Err(
            "native/current obfuscation must share the advertised Soulseek peer port".to_owned(),
        );
    }
    let explicit_overlay_bind = env.var("SLSKR_OVERLAY_BIND").or(listeners_overlay_bind);
    let overlay_bind = explicit_overlay_bind
        .clone()
        .map(|value| {
            let address = value
                .parse::<SocketAddr>()
                .map_err(|error| format!("invalid SLSKR_OVERLAY_BIND: {error}"))?;
            if address.port() == 0 {
                return Err("SLSKR_OVERLAY_BIND port must be non-zero".to_owned());
            }
            Ok(address)
        })
        .transpose()?
        .or_else(|| {
            if controller_profile == ControllerProfile::Native
                && current_upstream_behavior
                && advanced_networking.mesh.enabled
                && advanced_networking.mesh.enable_overlay
            {
                // Current upstream owns the mesh TCP handshake on the
                // Soulseek listen socket. Keep the legacy explicit
                // `listeners.overlay_bind` escape hatch above, but make the
                // stock/native projection use one public TCP endpoint.
                listener_bind
                    .as_deref()
                    .and_then(|value| value.parse::<SocketAddr>().ok())
            } else {
                (controller_profile == ControllerProfile::Native
                    && advanced_networking.overlay.enable)
                    .then_some(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                        advanced_networking.dht.overlay_port,
                    ))
            }
        });
    let dht_enabled = advanced_networking.dht.enabled;
    let dht_port = advanced_networking.dht.dht_port;
    let trusted_mesh_peers =
        trusted_mesh_peers_from_layers(env.var("SLSKR_TRUSTED_MESH_PEERS"), mesh_trusted_peers)?;
    let (
        obfuscation_enabled,
        obfuscation_mode,
        obfuscation_listen_port,
        obfuscation_advertise_regular_port,
        obfuscation_prefer_outbound,
    ) = if supports_soulseek_obfuscation {
        let enabled = env_bool_any_layer(
            env,
            &["SLSK_OBFUSCATION", "SLSKD_SLSK_OBFUSCATION"],
            obfuscation_enabled_file.unwrap_or(true),
        )?;
        let mode = SoulseekObfuscationMode::parse(
            optional_env_any(
                env,
                &["SLSK_OBFUSCATION_MODE", "SLSKD_SLSK_OBFUSCATION_MODE"],
            )
            .or(obfuscation_mode_file)
            .as_deref()
            .unwrap_or("compatibility"),
        )?;
        let advertise_regular_port = env_bool_any_layer(
            env,
            &[
                "SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT",
                "SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT",
            ],
            obfuscation_advertise_regular_port_file.unwrap_or(true),
        )?;
        let prefer_outbound = env_bool_any_layer(
            env,
            &[
                "SLSK_OBFUSCATION_PREFER_OUTBOUND",
                "SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND",
            ],
            obfuscation_prefer_outbound_file.unwrap_or(true),
        )?;
        if enabled && !advertise_regular_port && current_upstream_behavior {
            return Err(
                "The regular peer port must be advertised when peer obfuscation is enabled"
                    .to_owned(),
            );
        }
        (
            enabled,
            mode,
            u32::from(upstream_obfuscated_port.unwrap_or_default()),
            advertise_regular_port,
            prefer_outbound,
        )
    } else {
        (
            false,
            SoulseekObfuscationMode::Compatibility,
            0,
            true,
            false,
        )
    };
    Ok(ListenerAndObfuscationResolution {
        listener_bind,
        advertised_port,
        obfuscated_listener_bind,
        obfuscated_advertised_port,
        overlay_bind,
        dht_enabled,
        dht_port,
        trusted_mesh_peers,
        obfuscation_enabled,
        obfuscation_mode,
        obfuscation_listen_port,
        obfuscation_advertise_regular_port,
        obfuscation_prefer_outbound,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_soulseek_identity<E: ConfigEnv>(
    env: &E,
    state_dir: &Path,
    network_server_address: Option<String>,
    network_listen_port: Option<u32>,
    network_username: Option<String>,
    network_password: Option<String>,
    network_credential_store: Option<String>,
    network_credential_file: Option<PathBuf>,
    app_auto_connect: Option<bool>,
) -> Result<SoulseekIdentity, String> {
    let configured_server_address = env
        .var("SLSK_SERVER")
        .or(network_server_address)
        .unwrap_or_else(|| CONTROLLER_DEFAULT_SERVER_ADDRESS.to_owned());
    let (configured_server_host, configured_server_port) =
        split_server_address(&configured_server_address)?;
    let server_host = env
        .var("SLSKD_SLSK_ADDRESS")
        .unwrap_or(configured_server_host);
    let server_port = env_parse_any_layer(env, &["SLSKD_SLSK_PORT"], None, configured_server_port)?;
    let server_address = format_host_port(&server_host, server_port);
    let listen_port = env_parse_any_layer(
        env,
        &["SLSK_LISTEN_PORT", "SLSKD_SLSK_LISTEN_PORT"],
        network_listen_port,
        CONTROLLER_DEFAULT_LISTEN_PORT,
    )?;
    if !(1024..=65_535).contains(&listen_port) {
        return Err("Soulseek.ListenPort must be between 1024 and 65535".to_owned());
    }
    let username =
        optional_env_any(env, &["SLSK_USERNAME", "SLSKD_SLSK_USERNAME"]).or(network_username);
    let password =
        optional_env_any(env, &["SLSK_PASSWORD", "SLSKD_SLSK_PASSWORD"]).or(network_password);
    let credential_store = CredentialStoreMode::parse(
        env.var("SLSKR_CREDENTIAL_STORE")
            .or(network_credential_store)
            .unwrap_or_else(|| "os".to_owned())
            .as_str(),
    )?;
    let credential_file = env
        .var("SLSKR_CREDENTIAL_FILE")
        .map(PathBuf::from)
        .or(network_credential_file)
        .unwrap_or_else(|| state_dir.join("soulseek-credentials.json"));
    let auto_connect_default = app_auto_connect.unwrap_or(
        username.is_some() && password.is_some() || credential_store.auto_connect_default(),
    );
    let auto_connect = if env.var("SLSKR_AUTO_CONNECT").is_some() {
        env_bool_layer(env, "SLSKR_AUTO_CONNECT", auto_connect_default)?
    } else if env.var("SLSKD_NO_CONNECT").is_some() {
        !env_bool_layer(env, "SLSKD_NO_CONNECT", false)?
    } else {
        auto_connect_default
    };
    Ok(SoulseekIdentity {
        server_address,
        listen_port,
        username,
        password,
        credential_store,
        credential_file,
        auto_connect,
    })
}
