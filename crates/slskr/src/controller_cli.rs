use super::config::{ConfigEnv, ProcessEnv};
use std::{collections::BTreeMap, ffi::OsString};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct ServeInvocation {
    pub(super) once: bool,
    pub(super) config_environment: BTreeMap<String, String>,
}

pub(super) struct ControllerCliEnv<'a> {
    pub(super) values: &'a BTreeMap<String, String>,
}

impl ConfigEnv for ControllerCliEnv<'_> {
    fn var(&self, name: &str) -> Option<String> {
        self.values
            .get(name)
            .cloned()
            .or_else(|| ProcessEnv.var(name))
    }

    fn command_line_var(&self, name: &str) -> Option<String> {
        self.values.get(name).cloned()
    }
}

fn controller_cli_environment_name(flag: &str) -> Option<(&'static str, bool)> {
    Some(match flag {
        "-d" | "--debug" => ("SLSKD_DEBUG", false),
        "-H" | "--headless" => ("SLSKD_HEADLESS", false),
        "--remote-configuration" => ("SLSKD_REMOTE_CONFIGURATION", false),
        "--remote-file-management" => ("SLSKD_REMOTE_FILE_MANAGEMENT", false),
        "--no-config-watch" => ("SLSKD_NO_CONFIG_WATCH", false),
        "--no-connect" => ("SLSKD_NO_CONNECT", false),
        "-n" | "--no-logo" => ("SLSKD_NO_LOGO", false),
        "-x" | "--no-start" => ("SLSKD_NO_START", false),
        "--no-version-check" => ("SLSKD_NO_VERSION_CHECK", false),
        "--experimental" => ("SLSKD_EXPERIMENTAL", false),
        "--case-sensitive-regex" => ("SLSKD_CASE_SENSITIVE_REGEX", false),
        "--no-share-scan" => ("SLSKD_NO_SHARE_SCAN", false),
        "--force-share-scan" => ("SLSKD_FORCE_SHARE_SCAN", false),
        "--rooms" => ("SLSKD_ROOMS", true),
        "--share-cache-storage-mode" => ("SLSKD_SHARE_CACHE_STORAGE_MODE", true),
        "--share-cache-workers" => ("SLSKD_SHARE_CACHE_WORKERS", true),
        "--share-cache-retention" => ("SLSKD_SHARE_CACHE_RETENTION", true),
        "--shares-probe-media-attributes" => ("SLSKD_SHARES_PROBE_MEDIA_ATTRIBUTES", false),
        "--slsk-liked-interests" => ("SLSKD_SLSK_LIKED_INTERESTS", true),
        "--slsk-hated-interests" => ("SLSKD_SLSK_HATED_INTERESTS", true),
        "--wishlist-enabled" => ("SLSKD_WISHLIST_ENABLED", false),
        "--wishlist-interval" => ("SLSKD_WISHLIST_INTERVAL", true),
        "--wishlist-auto-download" => ("SLSKD_WISHLIST_AUTO_DOWNLOAD", false),
        "--wishlist-max-results" => ("SLSKD_WISHLIST_MAX_RESULTS", true),
        "--throttling-search-incoming-concurrency" => {
            ("SLSKD_THROTTLING_SEARCH_INCOMING_CONCURRENCY", true)
        }
        "--throttling-search-incoming-circuit-breaker" => {
            ("SLSKD_THROTTLING_SEARCH_INCOMING_CIRCUIT_BREAKER", true)
        }
        "--throttling-search-incoming-response-file-limit" => {
            ("SLSKD_THROTTLING_SEARCH_INCOMING_RESPONSE_FILE_LIMIT", true)
        }
        "--force-migrations" => ("SLSKD_FORCE_MIGRATIONS", false),
        "--legacy-windows-tcp-keepalive" => ("SLSKD_LEGACY_WINDOWS_TCP_KEEPALIVE", false),
        "--log-sql" => ("SLSKD_LOG_SQL", false),
        "--log-unobserved-exceptions" => ("SLSKD_LOG_UNOBSERVED_EXCEPTIONS", false),
        "--optimistic-relay-file-info" => ("SLSKD_OPTIMISTIC_RELAY_FILE_INFO", false),
        "--volatile" => ("SLSKD_VOLATILE", false),
        "--disk-logger" => ("SLSKD_DISK_LOGGER", false),
        "--loki" => ("SLSKD_LOKI", true),
        "--no-color" => ("SLSKD_NO_COLOR", false),
        "--file-permission-mode" => ("SLSKD_FILE_PERMISSION_MODE", true),
        "--telemetry-tracing" => ("SLSKD_TELEMETRY_TRACING", false),
        "--telemetry-tracing-exporter" => ("SLSKD_TELEMETRY_TRACING_EXPORTER", true),
        "--telemetry-jaeger-endpoint" => ("SLSKD_TELEMETRY_JAEGER_ENDPOINT", true),
        "--telemetry-jaeger-port" => ("SLSKD_TELEMETRY_JAEGER_PORT", true),
        "--telemetry-otlp-endpoint" => ("SLSKD_TELEMETRY_OTLP_ENDPOINT", true),
        "-r" | "--relay" => ("SLSKD_RELAY", false),
        "-m" | "--relay-mode" => ("SLSKD_RELAY_MODE", true),
        "--controller-address" => ("SLSKD_CONTROLLER_ADDRESS", true),
        "--controller-ignore-certificate-errors" => {
            ("SLSKD_CONTROLLER_IGNORE_CERTIFICATE_ERRORS", false)
        }
        "--controller-pinned-spki" => ("SLSKD_CONTROLLER_PINNED_SPKI", true),
        "--controller-api-key" => ("SLSKD_CONTROLLER_API_KEY", true),
        "--controller-secret" => ("SLSKD_CONTROLLER_SECRET", true),
        "--controller-downloads" => ("SLSKD_CONTROLLER_DOWNLOADS", false),
        "--songid-max-concurrent-runs" => ("SLSKD_SONGID_MAX_CONCURRENT_RUNS", true),
        "--enable-blacklist" => ("SLSKD_BLACKLIST", false),
        "--blacklist-file" => ("SLSKD_BLACKLIST_FILE", true),
        "--swagger" => ("SLSKD_SWAGGER", false),
        "--metrics" => ("SLSKD_METRICS", false),
        "--metrics-url" => ("SLSKD_METRICS_URL", true),
        "--metrics-no-auth" => ("SLSKD_METRICS_NO_AUTH", false),
        "--metrics-username" => ("SLSKD_METRICS_USERNAME", true),
        "--metrics-password" => ("SLSKD_METRICS_PASSWORD", true),
        "-X" | "--no-auth" => ("SLSKR_AUTH_DISABLED", false),
        "-u" | "--username" => ("SLSKD_USERNAME", true),
        "-p" | "--password" => ("SLSKD_PASSWORD", true),
        "--jwt-key" => ("SLSKD_JWT_KEY", true),
        "--jwt-ttl" => ("SLSKD_JWT_TTL", true),
        "--enforce-security" => ("SLSKD_ENFORCE_SECURITY", false),
        "--allow-remote-no-auth" => ("SLSKD_ALLOW_REMOTE_NO_AUTH", false),
        "--app-dir" => ("SLSKD_APP_DIR", true),
        "-i" | "--instance-name" => ("SLSKD_INSTANCE_NAME", true),
        "-s" | "--shared" => ("SLSKD_SHARED_DIR", true),
        "--share-filter" => ("SLSKD_SHARE_FILTER", true),
        "--search-request-filter" => ("SLSKD_SEARCH_REQUEST_FILTER", true),
        "--search-retention-max-age-days" => ("SLSKD_SEARCH_RETENTION_MAX_AGE_DAYS", true),
        "--search-retention-max-count" => ("SLSKD_SEARCH_RETENTION_MAX_COUNT", true),
        "--search-retention-cleanup-interval" => ("SLSKD_SEARCH_RETENTION_CLEANUP_INTERVAL", true),
        "-o" | "--downloads" => ("SLSKD_DOWNLOADS_DIR", true),
        "--incomplete" => ("SLSKD_INCOMPLETE_DIR", true),
        "--http-ip-address" => ("SLSKD_HTTP_IP_ADDRESS", true),
        "--http-address" => ("SLSKD_HTTP_ADDRESS", true),
        "--http-port" => ("SLSKD_HTTP_PORT", true),
        "--http-socket" => ("SLSKD_HTTP_SOCKET", true),
        "--no-https" => ("SLSKD_NO_HTTPS", false),
        "-L" | "--https-port" => ("SLSKD_HTTPS_PORT", true),
        "--https-ip-address" => ("SLSKD_HTTPS_IP_ADDRESS", true),
        "-f" | "--force-https" => ("SLSKD_HTTPS_FORCE", false),
        "--https-cert-pfx" => ("SLSKD_HTTPS_CERT_PFX", true),
        "--https-cert-password" => ("SLSKD_HTTPS_CERT_PASSWORD", true),
        "--url-base" => ("SLSKD_URL_BASE", true),
        "--content-path" => ("SLSKD_CONTENT_PATH", true),
        "--http-logging" => ("SLSKD_HTTP_LOGGING", false),
        "--slsk-address" => ("SLSKD_SLSK_ADDRESS", true),
        "--slsk-port" => ("SLSKD_SLSK_PORT", true),
        "--slsk-username" => ("SLSKD_SLSK_USERNAME", true),
        "--slsk-password" => ("SLSKD_SLSK_PASSWORD", true),
        "--slsk-listen-ip-address" => ("SLSKD_SLSK_LISTEN_IP_ADDRESS", true),
        "--slsk-listen-port" => ("SLSKD_SLSK_LISTEN_PORT", true),
        "--slsk-description" => ("SLSKD_SLSK_DESCRIPTION", true),
        "--slsk-picture" => ("SLSKD_SLSK_PICTURE", true),
        "--slsk-diag-level" => ("SLSKD_SLSK_DIAG_LEVEL", true),
        "--slsk-no-dnet" => ("SLSKD_SLSK_NO_DNET", false),
        "--slsk-dnet-no-children" => ("SLSKD_SLSK_DNET_NO_CHILDREN", false),
        "--slsk-dnet-children" => ("SLSKD_SLSK_DNET_CHILDREN", true),
        "--slsk-dnet-logging" => ("SLSKD_SLSK_DNET_LOGGING", false),
        "--slsk-read-buffer" => ("SLSKD_SLSK_READ_BUFFER", true),
        "--slsk-write-buffer" => ("SLSKD_SLSK_WRITE_BUFFER", true),
        "--slsk-transfer-buffer" => ("SLSKD_SLSK_TRANSFER_BUFFER", true),
        "--slsk-write-queue" => ("SLSKD_SLSK_WRITE_QUEUE", true),
        "--slsk-connection-timeout" => ("SLSKD_SLSK_CONNECTION_TIMEOUT", true),
        "--slsk-inactivity-timeout" => ("SLSKD_SLSK_INACTIVITY_TIMEOUT", true),
        "--slsk-transfer-timeout" => ("SLSKD_SLSK_TRANSFER_TIMEOUT", true),
        "--slsk-proxy" => ("SLSKD_SLSK_PROXY_ENABLED", false),
        "--slsk-proxy-address" => ("SLSKD_SLSK_PROXY_ADDRESS", true),
        "--slsk-proxy-port" => ("SLSKD_SLSK_PROXY_PORT", true),
        "--slsk-proxy-username" => ("SLSKD_SLSK_PROXY_USERNAME", true),
        "--slsk-proxy-password" => ("SLSKD_SLSK_PROXY_PASSWORD", true),
        "--slsk-obfuscation" => ("SLSKD_SLSK_OBFUSCATION", false),
        "--slsk-obfuscation-mode" => ("SLSKD_SLSK_OBFUSCATION_MODE", true),
        "--slsk-obfuscation-listen-port" => ("SLSKD_SLSK_OBFUSCATION_LISTEN_PORT", true),
        "--slsk-obfuscation-advertise-regular-port" => {
            ("SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT", false)
        }
        "--slsk-obfuscation-prefer-outbound" => ("SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND", false),
        "--download-completed-path-template" => ("SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE", true),
        "--download-completed-layout" => ("SLSKD_DOWNLOAD_COMPLETED_LAYOUT", true),
        "--download-filter-exclude" => ("SLSKD_DOWNLOAD_FILTER_EXCLUDE", true),
        "--download-slots" => ("SLSKD_DOWNLOAD_SLOTS", true),
        "--download-speed-limit" => ("SLSKD_DOWNLOAD_SPEED_LIMIT", true),
        "--auto-replace-stuck" => ("SLSKD_AUTO_REPLACE_STUCK", false),
        "--auto-replace-threshold" => ("SLSKD_AUTO_REPLACE_THRESHOLD", true),
        "--auto-replace-interval" => ("SLSKD_AUTO_REPLACE_INTERVAL", true),
        "--auto-replace-max-retries" => ("SLSKD_AUTO_REPLACE_MAX_RETRIES", true),
        "--upload-slots" => ("SLSKD_UPLOAD_SLOTS", true),
        "--upload-speed-limit" => ("SLSKD_UPLOAD_SPEED_LIMIT", true),
        "--slsk-private-message-auto-response" => {
            ("SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE", false)
        }
        "--slsk-private-message-auto-response-message" => {
            ("SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE", true)
        }
        "--slsk-private-message-auto-response-cooldown-minutes" => (
            "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES",
            true,
        ),
        "--spotify" => ("SLSKD_SPOTIFY", false),
        "--spotify-client-id" => ("SLSKD_SPOTIFY_CLIENT_ID", true),
        "--spotify-client-secret" => ("SLSKD_SPOTIFY_CLIENT_SECRET", true),
        "--spotify-redirect-uri" => ("SLSKD_SPOTIFY_REDIRECT_URI", true),
        "--spotify-timeout" => ("SLSKD_SPOTIFY_TIMEOUT", true),
        "--spotify-max-items-per-import" => ("SLSKD_SPOTIFY_MAX_ITEMS_PER_IMPORT", true),
        "--spotify-market" => ("SLSKD_SPOTIFY_MARKET", true),
        "--youtube" => ("SLSKD_YOUTUBE", false),
        "--youtube-api-key" => ("SLSKD_YOUTUBE_API_KEY", true),
        "--lastfm" => ("SLSKD_LASTFM", false),
        "--lastfm-api-key" => ("SLSKD_LASTFM_API_KEY", true),
        "--ntfy" => ("SLSKD_NTFY", false),
        "--ntfy-url" => ("SLSKD_NTFY_URL", true),
        "--ntfy-token" => ("SLSKD_NTFY_TOKEN", true),
        "--ntfy-prefix" => ("SLSKD_NTFY_NOTIFICATION_PREFIX", true),
        "--ntfy-notify-on-pm" => ("SLSKD_NTFY_NOTIFY_ON_PRIVATE_MESSAGE", false),
        "--ntfy-notify-on-room-mention" => ("SLSKD_NTFY_NOTIFY_ON_ROOM_MENTION", false),
        "--pushover" => ("SLSKD_PUSHOVER", false),
        "--pushover-user-key" => ("SLSKD_PUSHOVER_USER_KEY", true),
        "--pushover-token" => ("SLSKD_PUSHOVER_TOKEN", true),
        "--pushover-prefix" => ("SLSKD_PUSHOVER_NOTIFICATION_PREFIX", true),
        "--pushover-notify-on-pm" => ("SLSKD_PUSHOVER_NOTIFY_ON_PRIVATE_MESSAGE", false),
        "--pushover-notify-on-room-mention" => ("SLSKD_PUSHOVER_NOTIFY_ON_ROOM_MENTION", false),
        "--pushbullet" => ("SLSKD_PUSHBULLET", false),
        "--pushbullet-token" => ("SLSKD_PUSHBULLET_ACCESS_TOKEN", true),
        "--pushbullet-prefix" => ("SLSKD_PUSHBULLET_NOTIFICATION_PREFIX", true),
        "--pushbullet-notify-on-pm" => ("SLSKD_PUSHBULLET_NOTIFY_ON_PRIVATE_MESSAGE", false),
        "--pushbullet-notify-on-room-mention" => ("SLSKD_PUSHBULLET_NOTIFY_ON_ROOM_MENTION", false),
        "--pushbullet-retry-attempts" => ("SLSKD_PUSHBULLET_RETRY_ATTEMPTS", true),
        "--pushbullet-cooldown" => ("SLSKD_PUSHBULLET_COOLDOWN_TIME", true),
        "--ftp" => ("SLSKD_FTP", false),
        "--ftp-address" => ("SLSKD_FTP_ADDRESS", true),
        "--ftp-port" => ("SLSKD_FTP_PORT", true),
        "--ftp-encryption-mode" => ("SLSKD_FTP_ENCRYPTION_MODE", true),
        "--ftp-ignore-certificate-errors" => ("SLSKD_FTP_IGNORE_CERTIFICATE_ERRORS", false),
        "--ftp-username" => ("SLSKD_FTP_USERNAME", true),
        "--ftp-password" => ("SLSKD_FTP_PASSWORD", true),
        "--ftp-remote-path" => ("SLSKD_FTP_REMOTE_PATH", true),
        "--ftp-overwrite-existing" => ("SLSKD_FTP_OVERWRITE_EXISTING", false),
        "--ftp-connection-timeout" => ("SLSKD_FTP_CONNECTION_TIMEOUT", true),
        "--ftp-retry-attempts" => ("SLSKD_FTP_RETRY_ATTEMPTS", true),
        "--vpn" => ("SLSKD_VPN", false),
        "--vpn-port-forwarding" => ("SLSKD_VPN_PORT_FORWARDING", false),
        "--vpn-self-hosted-relay" => ("SLSKD_VPN_SELF_HOSTED_RELAY", false),
        "--vpn-polling-interval" => ("SLSKD_VPN_POLLING_INTERVAL", true),
        "--vpn-gluetun-url" => ("SLSKD_VPN_GLUETUN_URL", true),
        "--vpn-gluetun-timeout" => ("SLSKD_VPN_GLUETUN_TIMEOUT", true),
        "--vpn-gluetun-username" => ("SLSKD_VPN_GLUETUN_USERNAME", true),
        "--vpn-gluetun-password" => ("SLSKD_VPN_GLUETUN_PASSWORD", true),
        "--vpn-gluetun-api-key" => ("SLSKD_VPN_GLUETUN_API_KEY", true),
        "--lidarr" => ("SLSKD_LIDARR", false),
        "--lidarr-url" => ("SLSKD_LIDARR_URL", true),
        "--lidarr-api-key" => ("SLSKD_LIDARR_API_KEY", true),
        "--lidarr-timeout" => ("SLSKD_LIDARR_TIMEOUT", true),
        "--lidarr-sync-wanted" => ("SLSKD_LIDARR_SYNC_WANTED", false),
        "--lidarr-sync-interval" => ("SLSKD_LIDARR_SYNC_INTERVAL", true),
        "--lidarr-sync-max-items" => ("SLSKD_LIDARR_SYNC_MAX_ITEMS", true),
        "--lidarr-auto-download" => ("SLSKD_LIDARR_AUTO_DOWNLOAD", false),
        "--lidarr-wishlist-filter" => ("SLSKD_LIDARR_WISHLIST_FILTER", true),
        "--lidarr-wishlist-max-results" => ("SLSKD_LIDARR_WISHLIST_MAX_RESULTS", true),
        "--lidarr-auto-import-completed" => ("SLSKD_LIDARR_AUTO_IMPORT_COMPLETED", false),
        "--lidarr-import-path-from" => ("SLSKD_LIDARR_IMPORT_PATH_FROM", true),
        "--lidarr-import-path-to" => ("SLSKD_LIDARR_IMPORT_PATH_TO", true),
        "--lidarr-import-mode" => ("SLSKD_LIDARR_IMPORT_MODE", true),
        "--lidarr-import-replace-existing" => ("SLSKD_LIDARR_IMPORT_REPLACE_EXISTING", false),
        "--lidarr-import-delay" => ("SLSKD_LIDARR_IMPORT_DELAY", true),
        "--lidarr-import-retry-max-attempts" => ("SLSKD_LIDARR_IMPORT_RETRY_MAX_ATTEMPTS", true),
        "--lidarr-import-retry-delay" => ("SLSKD_LIDARR_IMPORT_RETRY_DELAY", true),
        "--lidarr-skip-already-owned-albums" => ("SLSKD_LIDARR_SKIP_ALREADY_OWNED_ALBUMS", false),
        "--lidarr-delete-rejected-downloads" => ("SLSKD_LIDARR_DELETE_REJECTED_DOWNLOADS", false),
        "--lidarr-blacklist-rejected-downloads" => {
            ("SLSKD_LIDARR_BLACKLIST_REJECTED_DOWNLOADS", false)
        }
        "--lidarr-edition-match-mode" => ("SLSKD_LIDARR_EDITION_MATCH_MODE", true),
        _ => return None,
    })
}

pub(super) fn parse_serve_args(args: &[OsString]) -> Result<Option<ServeInvocation>, String> {
    let direct_controller_invocation =
        args.first()
            .and_then(|value| value.to_str())
            .is_some_and(|value| {
                let flag = value.split_once('=').map_or(value, |(flag, _)| flag);
                controller_cli_environment_name(flag).is_some()
            });
    if args.first().is_none_or(|arg| arg != "serve") && !direct_controller_invocation {
        return Ok(None);
    }

    let mut invocation = ServeInvocation::default();
    let mut index = usize::from(args.first().is_some_and(|arg| arg == "serve"));
    while index < args.len() {
        let raw = args[index]
            .to_str()
            .ok_or_else(|| "serve arguments must be valid UTF-8".to_owned())?;
        if raw == "--once" {
            if invocation.once {
                return Err("duplicate serve option --once".to_owned());
            }
            invocation.once = true;
            index += 1;
            continue;
        }
        let (flag, inline_value) = raw
            .split_once('=')
            .map_or((raw, None), |(flag, value)| (flag, Some(value)));
        let Some((environment_name, takes_value)) = controller_cli_environment_name(flag) else {
            return Err(format!("unknown serve option {flag}"));
        };
        let multi_valued = matches!(
            environment_name,
            "SLSKD_SHARED_DIR"
                | "SLSKD_SHARE_FILTER"
                | "SLSKD_SEARCH_REQUEST_FILTER"
                | "SLSKD_DOWNLOAD_FILTER_EXCLUDE"
        );
        if invocation.config_environment.contains_key(environment_name) && !multi_valued {
            return Err(format!("duplicate serve option {flag}"));
        }
        let value = if takes_value {
            if let Some(value) = inline_value {
                if value.is_empty() {
                    return Err(format!("{flag} requires a value"));
                }
                value.to_owned()
            } else {
                index += 1;
                args.get(index)
                    .and_then(|value| value.to_str())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| format!("{flag} requires a value"))?
                    .to_owned()
            }
        } else {
            if inline_value.is_some() {
                return Err(format!("{flag} does not accept a value"));
            }
            "true".to_owned()
        };
        invocation
            .config_environment
            .entry(environment_name.to_owned())
            .and_modify(|current| {
                current.push(';');
                current.push_str(&value);
            })
            .or_insert(value);
        index += 1;
    }
    Ok(Some(invocation))
}
