use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct TransferAutoRetrySettings {
    pub enabled: bool,
    pub retry_delay: Duration,
    pub check_interval: Duration,
    pub max_attempts: usize,
    pub max_files_per_cycle: usize,
    pub max_files_per_peer_per_cycle: usize,
    pub peer_cooldown: Duration,
    pub alternate_sources_enabled: bool,
    pub max_alternate_source_searches_per_cycle: usize,
    pub alternate_source_size_tolerance_percent: f64,
}

impl TransferAutoRetrySettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        file: TransferAutoRetryFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let retry_delay_seconds = bounded_config_value(
            "SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS",
                file.retry_delay_seconds,
                1800_u64,
            )?,
            10,
            86_400,
        )?;
        let check_interval_seconds = bounded_config_value(
            "SLSKR_TRANSFER_AUTO_RETRY_CHECK_INTERVAL_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_AUTO_RETRY_CHECK_INTERVAL_SECONDS",
                file.check_interval_seconds,
                300_u64,
            )?,
            10,
            3_600,
        )?;
        let peer_cooldown_seconds = bounded_config_value(
            "SLSKR_TRANSFER_AUTO_RETRY_PEER_COOLDOWN_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_AUTO_RETRY_PEER_COOLDOWN_SECONDS",
                file.peer_cooldown_seconds,
                900_u64,
            )?,
            60,
            86_400,
        )?;
        Ok(Self {
            enabled: env_bool_layer(
                env,
                "SLSKR_TRANSFER_AUTO_RETRY_ENABLED",
                file.enabled.unwrap_or(true),
            )?,
            retry_delay: Duration::from_secs(retry_delay_seconds),
            check_interval: Duration::from_secs(check_interval_seconds),
            max_attempts: bounded_config_value(
                "SLSKR_TRANSFER_AUTO_RETRY_MAX_ATTEMPTS",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_AUTO_RETRY_MAX_ATTEMPTS",
                    file.max_attempts,
                    5_usize,
                )?,
                0,
                100,
            )?,
            max_files_per_cycle: bounded_config_value(
                "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_CYCLE",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_CYCLE",
                    file.max_files_per_cycle,
                    10_usize,
                )?,
                1,
                100,
            )?,
            max_files_per_peer_per_cycle: bounded_config_value(
                "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_PEER_PER_CYCLE",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_PEER_PER_CYCLE",
                    file.max_files_per_peer_per_cycle,
                    1_usize,
                )?,
                1,
                20,
            )?,
            peer_cooldown: Duration::from_secs(peer_cooldown_seconds),
            alternate_sources_enabled: env_bool_layer(
                env,
                "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCES_ENABLED",
                file.alternate_sources_enabled.unwrap_or(true),
            )?,
            max_alternate_source_searches_per_cycle: bounded_config_value(
                "SLSKR_TRANSFER_AUTO_RETRY_MAX_ALTERNATE_SOURCE_SEARCHES_PER_CYCLE",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_AUTO_RETRY_MAX_ALTERNATE_SOURCE_SEARCHES_PER_CYCLE",
                    file.max_alternate_source_searches_per_cycle,
                    1_usize,
                )?,
                0,
                10,
            )?,
            alternate_source_size_tolerance_percent: {
                let value = env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                    file.alternate_source_size_tolerance_percent,
                    5.0_f64,
                )?;
                // The frozen native profile snapshot applies an integer-operand
                // RangeAttribute to this double.  Its conversion rounds
                // fractional boundary values, accepting -0.5 through 100.5.
                // Upstream correction: snapetech/slskdN#271.
                if !value.is_finite() || !(-0.5..=100.5).contains(&value) {
                    return Err(
                        "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT must be between 0 and 100"
                            .to_owned(),
                    );
                }
                value
            },
        })
    }

    pub(super) fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"retry_delay_seconds\":{},\"check_interval_seconds\":{},\"max_attempts\":{},\"max_files_per_cycle\":{},\"max_files_per_peer_per_cycle\":{},\"peer_cooldown_seconds\":{},\"alternate_sources_enabled\":{},\"max_alternate_source_searches_per_cycle\":{},\"alternate_source_size_tolerance_percent\":{}}}",
            self.enabled,
            self.retry_delay.as_secs(),
            self.check_interval.as_secs(),
            self.max_attempts,
            self.max_files_per_cycle,
            self.max_files_per_peer_per_cycle,
            self.peer_cooldown.as_secs(),
            self.alternate_sources_enabled,
            self.max_alternate_source_searches_per_cycle,
            self.alternate_source_size_tolerance_percent,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferRescueSettings {
    pub enabled: bool,
    pub max_queue_time: Duration,
    pub min_throughput_bytes_per_second: u64,
    pub min_duration: Duration,
    pub stalled_timeout: Duration,
    pub check_interval: Duration,
    pub retry_cooldown: Duration,
    pub max_files_per_cycle: usize,
    pub alternate_source_size_tolerance_percent: u32,
}

impl TransferRescueSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        file: TransferRescueFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let max_queue_time_seconds = bounded_config_value(
            "SLSKR_TRANSFER_RESCUE_MAX_QUEUE_TIME_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_MAX_QUEUE_TIME_SECONDS",
                file.max_queue_time_seconds,
                1_800_u64,
            )?,
            60,
            86_400,
        )?;
        let min_throughput_kbps = bounded_config_value(
            "SLSKR_TRANSFER_RESCUE_MIN_THROUGHPUT_KBPS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_MIN_THROUGHPUT_KBPS",
                file.min_throughput_kbps,
                10_u64,
            )?,
            1,
            10_000,
        )?;
        let min_duration_seconds = bounded_config_value(
            "SLSKR_TRANSFER_RESCUE_MIN_DURATION_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_MIN_DURATION_SECONDS",
                file.min_duration_seconds,
                300_u64,
            )?,
            60,
            3_600,
        )?;
        let stalled_timeout_seconds = bounded_config_value(
            "SLSKR_TRANSFER_RESCUE_STALLED_TIMEOUT_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_STALLED_TIMEOUT_SECONDS",
                file.stalled_timeout_seconds,
                120_u64,
            )?,
            30,
            600,
        )?;
        let check_interval_seconds = bounded_config_value(
            "SLSKR_TRANSFER_RESCUE_CHECK_INTERVAL_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_CHECK_INTERVAL_SECONDS",
                file.check_interval_seconds,
                45_u64,
            )?,
            15,
            300,
        )?;
        let retry_cooldown_seconds = bounded_config_value(
            "SLSKR_TRANSFER_RESCUE_RETRY_COOLDOWN_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_RETRY_COOLDOWN_SECONDS",
                file.retry_cooldown_seconds,
                1_800_u64,
            )?,
            60,
            86_400,
        )?;
        Ok(Self {
            enabled: env_bool_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_ENABLED",
                file.enabled.unwrap_or(true),
            )?,
            max_queue_time: Duration::from_secs(max_queue_time_seconds),
            min_throughput_bytes_per_second: min_throughput_kbps.saturating_mul(1_024),
            min_duration: Duration::from_secs(min_duration_seconds),
            stalled_timeout: Duration::from_secs(stalled_timeout_seconds),
            check_interval: Duration::from_secs(check_interval_seconds),
            retry_cooldown: Duration::from_secs(retry_cooldown_seconds),
            max_files_per_cycle: bounded_config_value(
                "SLSKR_TRANSFER_RESCUE_MAX_FILES_PER_CYCLE",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_RESCUE_MAX_FILES_PER_CYCLE",
                    file.max_files_per_cycle,
                    2_usize,
                )?,
                1,
                20,
            )?,
            alternate_source_size_tolerance_percent: bounded_config_value(
                "SLSKR_TRANSFER_RESCUE_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_RESCUE_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                    file.alternate_source_size_tolerance_percent,
                    5_u32,
                )?,
                0,
                100,
            )?,
        })
    }

    pub(super) fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"max_queue_time_seconds\":{},\"min_throughput_kbps\":{},\"min_duration_seconds\":{},\"stalled_timeout_seconds\":{},\"check_interval_seconds\":{},\"retry_cooldown_seconds\":{},\"max_files_per_cycle\":{},\"alternate_source_size_tolerance_percent\":{}}}",
            self.enabled,
            self.max_queue_time.as_secs(),
            self.min_throughput_bytes_per_second / 1_024,
            self.min_duration.as_secs(),
            self.stalled_timeout.as_secs(),
            self.check_interval.as_secs(),
            self.retry_cooldown.as_secs(),
            self.max_files_per_cycle,
            self.alternate_source_size_tolerance_percent,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManagedBlacklistRange {
    pub(super) first: u32,
    pub(super) last: u32,
}

impl ManagedBlacklistRange {
    #[must_use]
    pub fn contains(self, address: Ipv4Addr) -> bool {
        let address = u32::from(address);
        (self.first..=self.last).contains(&address)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedBlacklistSettings {
    pub enabled: bool,
    pub file: Option<PathBuf>,
    pub ranges: Vec<ManagedBlacklistRange>,
    pub members: Vec<String>,
    pub patterns: Vec<String>,
    pub cidrs: Vec<TrustedProxyCidr>,
    pub cidr_values: Vec<String>,
}

impl ManagedBlacklistSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        file: ManagedBlacklistFileConfig,
        users: &UserBlacklistFileConfig,
        env: &E,
        target: ControllerProfile,
    ) -> Result<Self, String> {
        let enabled = env_bool_layer(env, "SLSKD_BLACKLIST", file.enabled.unwrap_or(false))?;
        let path = env
            .var("SLSKD_BLACKLIST_FILE")
            .map(PathBuf::from)
            .or(file.file);
        let ranges = if enabled {
            let path = path.as_deref().ok_or_else(|| {
                "Blacklist.Enabled is true, but no Blacklist.File has been specified".to_owned()
            })?;
            load_managed_blacklist_file(path, target)?
        } else {
            Vec::new()
        };
        let members =
            controller_string_array_layer(env, "SLSKD_BLACKLISTED_MEMBERS", users.members.clone());
        let patterns = controller_string_array_layer(
            env,
            "SLSKD_BLACKLISTED_PATTERNS",
            users.patterns.clone(),
        );
        for pattern in &patterns {
            crate::dotnet_regex::DotNetRegex::validate(pattern).map_err(|_| match target {
                ControllerProfile::Legacy => {
                    format!("Pattern '{pattern}' is not a valid regular expression")
                }
                ControllerProfile::Native => {
                    format!("Blacklist pattern {pattern} is invalid")
                }
            })?;
        }
        let cidr_values =
            controller_string_array_layer(env, "SLSKD_BLACKLISTED_CIDRS", users.cidrs.clone());
        let cidrs = cidr_values
            .iter()
            .map(|cidr| {
                if cidr.to_ascii_lowercase().starts_with("::ffff") {
                    return Err(format!("CIDR {cidr} is invalid"));
                }
                let normalized = if cidr.contains('/') {
                    cidr.clone()
                } else if cidr.contains(':') {
                    format!("{cidr}/128")
                } else {
                    format!("{cidr}/32")
                };
                TrustedProxyCidr::parse(&normalized).map_err(|error| match target {
                    ControllerProfile::Legacy => {
                        format!("CIDR {cidr} is invalid: {error}")
                    }
                    ControllerProfile::Native => format!("CIDR {cidr} is invalid"),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            enabled,
            file: path,
            ranges,
            members,
            patterns,
            cidrs,
            cidr_values,
        })
    }

    #[must_use]
    pub fn contains(&self, address: IpAddr) -> bool {
        if self.cidrs.iter().any(|cidr| cidr.contains(address)) {
            return true;
        }
        if !self.enabled {
            return false;
        }
        let address = match address {
            IpAddr::V4(address) => address,
            IpAddr::V6(address) => match address.to_ipv4_mapped() {
                Some(address) => address,
                None => return false,
            },
        };
        self.ranges.iter().any(|range| range.contains(address))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ManagedBlacklistFormat {
    Cidr,
    P2p,
    Dat,
}

pub fn validate_managed_blacklist_file_format(
    path: &std::path::Path,
    target: ControllerProfile,
) -> Result<(), String> {
    let body = fs::read_to_string(path)
        .map_err(|error| format!("failed to read blacklist file {}: {error}", path.display()))?;
    detect_managed_blacklist_format(&body, target).map(|_| ())
}

pub(super) fn load_managed_blacklist_file(
    path: &std::path::Path,
    target: ControllerProfile,
) -> Result<Vec<ManagedBlacklistRange>, String> {
    let body = fs::read_to_string(path)
        .map_err(|error| format!("failed to read blacklist file {}: {error}", path.display()))?;
    let format = detect_managed_blacklist_format(&body, target)?;
    let mut ranges = Vec::new();
    for (index, line) in body.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let raw_range = managed_blacklist_line_range(line, format, target).map_err(|error| {
            format!(
                "failed to parse managed blacklist line {} {line:?}: {error}",
                index + 1
            )
        })?;
        ranges.push(parse_ipv4_range(&raw_range).map_err(|error| {
            format!(
                "failed to parse managed blacklist line {} {line:?}: {error}",
                index + 1
            )
        })?);
    }
    ranges.sort_unstable_by_key(|range| (range.first, range.last));
    ranges.dedup();
    Ok(ranges)
}

pub(super) fn detect_managed_blacklist_format(
    body: &str,
    target: ControllerProfile,
) -> Result<ManagedBlacklistFormat, String> {
    for line in body.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        if parse_ipv4_range(line).is_ok() {
            return Ok(ManagedBlacklistFormat::Cidr);
        }
        if managed_blacklist_line_range(line, ManagedBlacklistFormat::P2p, target)
            .and_then(|range| parse_ipv4_range(&range))
            .is_ok()
        {
            return Ok(ManagedBlacklistFormat::P2p);
        }
        if managed_blacklist_line_range(line, ManagedBlacklistFormat::Dat, target)
            .and_then(|range| parse_ipv4_range(&range))
            .is_ok()
        {
            return Ok(ManagedBlacklistFormat::Dat);
        }
        break;
    }
    Err(
        "Failed to detect blacklist format. Only CIDR, P2P and DAT formats are supported"
            .to_owned(),
    )
}

pub(super) fn managed_blacklist_line_range(
    line: &str,
    format: ManagedBlacklistFormat,
    target: ControllerProfile,
) -> Result<String, String> {
    match format {
        ManagedBlacklistFormat::Cidr => Ok(line.to_owned()),
        ManagedBlacklistFormat::P2p => {
            let range = match target {
                ControllerProfile::Legacy => line.split(':').nth(1),
                ControllerProfile::Native => line.rsplit_once(':').map(|(_, range)| range),
            }
            .map(str::trim)
            .filter(|range| !range.is_empty())
            .ok_or_else(|| "invalid P2P blacklist line".to_owned())?;
            Ok(range.to_owned())
        }
        ManagedBlacklistFormat::Dat => {
            let range = line
                .split(',')
                .next()
                .map(|range| range.replace(' ', ""))
                .filter(|range| !range.is_empty())
                .ok_or_else(|| "invalid DAT blacklist line".to_owned())?;
            range
                .split('-')
                .map(trim_ipv4_leading_zeroes)
                .collect::<Result<Vec<_>, _>>()
                .map(|addresses| addresses.join("-"))
        }
    }
}

pub(super) fn trim_ipv4_leading_zeroes(address: &str) -> Result<String, String> {
    let octets = address
        .split('.')
        .map(|octet| {
            octet
                .parse::<u8>()
                .map(|octet| octet.to_string())
                .map_err(|_| "invalid IPv4 octet".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if octets.len() != 4 {
        return Err("invalid IPv4 address".to_owned());
    }
    Ok(octets.join("."))
}

pub(super) fn parse_ipv4_range(value: &str) -> Result<ManagedBlacklistRange, String> {
    let value = value.trim();
    if let Some((address, prefix)) = value.split_once('/') {
        let address = address
            .trim()
            .parse::<Ipv4Addr>()
            .map_err(|_| "invalid IPv4 address".to_owned())?;
        let prefix = prefix
            .trim()
            .parse::<u8>()
            .map_err(|_| "invalid IPv4 prefix".to_owned())?;
        if prefix > 32 {
            return Err("invalid IPv4 prefix".to_owned());
        }
        let mask = if prefix == 0 {
            0
        } else {
            u32::MAX << (32 - prefix)
        };
        let first = u32::from(address) & mask;
        return Ok(ManagedBlacklistRange {
            first,
            last: first | !mask,
        });
    }
    let (first, last) = value
        .split_once('-')
        .map_or((value, value), |(first, last)| (first.trim(), last.trim()));
    let first = u32::from(
        first
            .parse::<Ipv4Addr>()
            .map_err(|_| "invalid IPv4 range start".to_owned())?,
    );
    let last = u32::from(
        last.parse::<Ipv4Addr>()
            .map_err(|_| "invalid IPv4 range end".to_owned())?,
    );
    if first > last {
        return Err("IPv4 range start exceeds end".to_owned());
    }
    Ok(ManagedBlacklistRange { first, last })
}

#[derive(Clone, Debug)]
pub struct ShareSettings {
    pub fixture_entries: Vec<FileEntry>,
    pub directories: Vec<ShareDirectory>,
    pub roots: Vec<PathBuf>,
    pub follow_symlinks: bool,
    pub include_hidden: bool,
    pub max_files: usize,
    pub cache_tsv_enabled: bool,
    pub cache_storage_mode: String,
    pub cache_workers: usize,
    pub cache_retention: Option<Duration>,
    pub probe_media_attributes: bool,
    pub filters: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShareDirectory {
    pub raw: String,
    pub alias: String,
    pub local_path: PathBuf,
    pub is_excluded: bool,
}

impl ShareDirectory {
    pub fn from_path(path: &str, alias: Option<&str>) -> Result<Self, String> {
        let path = path.trim();
        if path.contains('\0') {
            return Err("share path contains an invalid NUL character".to_owned());
        }
        let raw = match alias.map(str::trim).filter(|alias| !alias.is_empty()) {
            Some(alias) => {
                if alias.contains('\0') {
                    return Err("share alias contains an invalid NUL character".to_owned());
                }
                format!("[{alias}]{path}")
            }
            None => path.to_owned(),
        };
        Self::parse(&raw)
    }

    pub(super) fn parse(value: &str) -> Result<Self, String> {
        let raw = value.trim().trim_end_matches(['/', '\\']).to_owned();
        let (is_excluded, share) = raw
            .strip_prefix(['!', '-'])
            .map_or((false, raw.as_str()), |share| (true, share));
        let (alias, local_path) = if let Some(rest) = share.strip_prefix('[') {
            let Some(close) = rest.find(']') else {
                return Err(format!(
                    "Share '{raw}' contains a relative path; only absolute paths are supported."
                ));
            };
            (rest[..close].to_owned(), PathBuf::from(&rest[close + 1..]))
        } else {
            let path = PathBuf::from(share);
            let alias = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned();
            (alias, path)
        };
        if alias.trim().is_empty() {
            return Err(format!(
                "Share '{raw}' is invalid; alias may not be null, empty or consist of only whitespace"
            ));
        }
        if alias.contains(['/', '\\']) {
            return Err(format!(
                "Share '{raw}' is invalid; aliases may not contain path separators '/' or '\\'"
            ));
        }
        if local_path.as_os_str().is_empty() {
            return Err(format!("Share {raw} does not specify a path"));
        }
        if !local_path.is_absolute() {
            return Err(format!(
                "Share {raw} contains a relative path; only absolute paths are supported."
            ));
        }
        if local_path
            .components()
            .any(|component| component == std::path::Component::ParentDir)
        {
            return Err(format!(
                "Share {raw} contains an unsafe path traversal segment."
            ));
        }
        Ok(Self {
            raw,
            alias,
            local_path,
            is_excluded,
        })
    }
}

impl ShareSettings {
    pub fn from_layers<E: ConfigEnv>(
        file_config: ShareFileConfig,
        env: &E,
        target: ControllerProfile,
    ) -> Result<Self, String> {
        let fixture = env
            .var("SLSKR_SHARE_FIXTURE")
            .or(file_config.fixture)
            .unwrap_or_default();
        let raw_directories = match optional_env_any(
            env,
            &["SLSKR_SHARE_DIRS", "SLSKR_SHARED_DIR", "SLSKD_SHARED_DIR"],
        ) {
            Some(value) => value,
            None => file_config.dirs.join(";"),
        };
        let directories = parse_share_directories(&raw_directories)?;
        let filters = controller_string_array_layer(env, "SLSKD_SHARE_FILTER", file_config.filters);
        for filter in &filters {
            crate::dotnet_regex::DotNetRegex::validate(filter).map_err(|_| {
                format!("Share filter '{filter}' is not a valid regular expression")
            })?;
        }
        let mut aliases = std::collections::BTreeSet::new();
        let mut paths = std::collections::BTreeSet::new();
        for directory in &directories {
            if !aliases.insert(directory.alias.clone()) {
                return Err(format!(
                    "Share alias '{}' collides with another configured share",
                    directory.alias
                ));
            }
            if !paths.insert(directory.local_path.clone()) {
                return Err(format!(
                    "Share path '{}' is configured more than once",
                    directory.local_path.display()
                ));
            }
        }
        let roots = directories
            .iter()
            .filter(|directory| !directory.is_excluded)
            .map(|directory| directory.local_path.clone())
            .collect();
        let storage_mode = env
            .var("SLSKD_SHARE_CACHE_STORAGE_MODE")
            .or(file_config.cache.storage_mode)
            .unwrap_or_else(|| "memory".to_owned())
            .to_ascii_lowercase();
        if !matches!(storage_mode.as_str(), "memory" | "disk") {
            return Err("shares.cache.storage_mode must be memory or disk".to_owned());
        }
        let processor_count = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1);
        let default_workers = if target == ControllerProfile::Native {
            if processor_count <= 2 {
                1
            } else {
                (processor_count / 2).clamp(2, 4)
            }
        } else {
            processor_count
        };
        let cache_workers = env_parse_layer(
            env,
            "SLSKD_SHARE_CACHE_WORKERS",
            file_config.cache.workers,
            default_workers,
        )?;
        if !(1..=128).contains(&cache_workers) {
            return Err("shares.cache.workers must be between 1 and 128".to_owned());
        }
        let cache_retention_minutes = env_parse_option_layer(
            env,
            "SLSKD_SHARE_CACHE_RETENTION",
            file_config.cache.retention,
        )?;
        if cache_retention_minutes.is_some_and(|minutes| minutes < 60) {
            return Err("shares.cache.retention must be at least 60 minutes".to_owned());
        }
        let legacy_cache_enabled = env_bool_layer(
            env,
            "SLSKR_SHARE_CACHE_TSV_ENABLED",
            file_config.cache_tsv_enabled.unwrap_or(true),
        )?;
        Ok(Self {
            fixture_entries: parse_share_entries(&fixture)?,
            directories,
            roots,
            follow_symlinks: env_bool_layer(
                env,
                "SLSKR_SHARE_FOLLOW_SYMLINKS",
                file_config.follow_symlinks.unwrap_or(false),
            )?,
            include_hidden: env_bool_layer(
                env,
                "SLSKR_SHARE_INCLUDE_HIDDEN",
                file_config.include_hidden.unwrap_or(false),
            )?,
            max_files: env_parse_layer(
                env,
                "SLSKR_SHARE_SCAN_MAX_FILES",
                file_config.scan_max_files,
                50_000_usize,
            )?,
            cache_tsv_enabled: legacy_cache_enabled || storage_mode == "disk",
            cache_storage_mode: storage_mode,
            cache_workers,
            cache_retention: cache_retention_minutes
                .map(|minutes| Duration::from_secs(minutes.saturating_mul(60))),
            probe_media_attributes: env_bool_any_layer(
                env,
                &[
                    "SLSKR_SHARES_PROBE_MEDIA_ATTRIBUTES",
                    "SHARES_PROBE_MEDIA_ATTRIBUTES",
                    "SLSKD_SHARES_PROBE_MEDIA_ATTRIBUTES",
                ],
                file_config.probe_media_attributes.unwrap_or(true),
            )?,
            filters,
        })
    }
}

pub fn parse_share_entries(value: &str) -> Result<Vec<FileEntry>, String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(parse_share_entry)
        .collect()
}

pub fn parse_share_entry(value: &str) -> Result<FileEntry, String> {
    let (filename, size) = value
        .rsplit_once('=')
        .ok_or_else(|| "SLSKR_SHARE_FIXTURE entries must be path=size".to_owned())?;
    let size = size
        .parse::<u64>()
        .map_err(|error| format!("invalid SLSKR_SHARE_FIXTURE size: {error}"))?;
    Ok(FileEntry {
        code: 1,
        filename: filename.trim().replace('\\', "/"),
        filename_encoding: slskr_client::protocol::ProtocolTextEncoding::Utf8,
        size,
        extension: extension_for(filename.trim()),
        extension_encoding: slskr_client::protocol::ProtocolTextEncoding::Utf8,
        attributes: Vec::new(),
    })
}

pub fn parse_share_directories(value: &str) -> Result<Vec<ShareDirectory>, String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ShareDirectory::parse)
        .collect()
}

pub(super) fn extension_for(filename: &str) -> String {
    filename
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default()
}
