use super::*;

pub(super) struct ControllerYamlEnv<'a, E> {
    pub(super) base: &'a E,
    pub(super) yaml: BTreeMap<String, String>,
}

impl<E: ConfigEnv> ConfigEnv for ControllerYamlEnv<'_, E> {
    fn var(&self, name: &str) -> Option<String> {
        self.base
            .command_line_var(name)
            .or_else(|| self.yaml.get(name).cloned())
            .or_else(|| self.base.var(name))
    }

    fn command_line_var(&self, name: &str) -> Option<String> {
        self.base.command_line_var(name)
    }
}

const CONTROLLER_YAML_CORE_MAPPINGS: &[(&str, &str)] = &[
    ("headless", "SLSKD_HEADLESS"),
    ("blacklist.enabled", "SLSKD_BLACKLIST"),
    ("blacklist.file", "SLSKD_BLACKLIST_FILE"),
    ("feature.swagger", "SLSKD_SWAGGER"),
    ("socialFederation.enabled", "FEDERATION_ENABLED"),
    ("socialFederation.mode", "FEDERATION_MODE"),
    ("socialFederation.domain", "FEDERATION_DOMAIN"),
    ("socialFederation.baseUrl", "FEDERATION_BASE_URL"),
    (
        "socialFederation.approvedPeers",
        "FEDERATION_APPROVED_PEERS",
    ),
    (
        "socialFederation.outboxMaxActivities",
        "FEDERATION_OUTBOX_MAX_ACTIVITIES",
    ),
    ("socialFederation.pageSize", "FEDERATION_PAGE_SIZE"),
    (
        "socialFederation.verifySignatures",
        "FEDERATION_VERIFY_SIGNATURES",
    ),
    (
        "socialFederation.httpTimeoutSeconds",
        "FEDERATION_HTTP_TIMEOUT_SECONDS",
    ),
    (
        "federationPublishing.enabled",
        "FEDERATION_PUBLISHING_ENABLED",
    ),
    (
        "federationPublishing.publishableDomains",
        "FEDERATION_PUBLISHING_PUBLISHABLE_DOMAINS",
    ),
    (
        "federationPublishing.defaultVisibility",
        "FEDERATION_PUBLISHING_DEFAULT_VISIBILITY",
    ),
    (
        "federationPublishing.approvedCircles",
        "FEDERATION_PUBLISHING_APPROVED_CIRCLES",
    ),
    (
        "federationPublishing.requireModerationApproval",
        "FEDERATION_PUBLISHING_REQUIRE_MODERATION",
    ),
    (
        "federationPublishing.includeExternalLinks",
        "FEDERATION_PUBLISHING_INCLUDE_EXTERNAL_LINKS",
    ),
    (
        "federationPublishing.maxMetadataSizeKb",
        "FEDERATION_PUBLISHING_MAX_METADATA_SIZE_KB",
    ),
    ("SignalSystem.Enabled", "SLSKD_SIGNALSYSTEM_ENABLED"),
    (
        "SignalSystem.DeduplicationCacheSize",
        "SLSKD_SIGNALSYSTEM_DEDUPLICATIONCACHESIZE",
    ),
    ("SignalSystem.DefaultTtl", "SLSKD_SIGNALSYSTEM_DEFAULTTTL"),
    (
        "SignalSystem.MeshChannel.Enabled",
        "SLSKD_SIGNALSYSTEM_MESHCHANNEL_ENABLED",
    ),
    (
        "SignalSystem.MeshChannel.Priority",
        "SLSKD_SIGNALSYSTEM_MESHCHANNEL_PRIORITY",
    ),
    (
        "SignalSystem.MeshChannel.RequireActiveSession",
        "SLSKD_SIGNALSYSTEM_MESHCHANNEL_REQUIREACTIVESESSION",
    ),
    (
        "SignalSystem.BtExtensionChannel.Enabled",
        "SLSKD_SIGNALSYSTEM_BTEXTENSIONCHANNEL_ENABLED",
    ),
    (
        "SignalSystem.BtExtensionChannel.Priority",
        "SLSKD_SIGNALSYSTEM_BTEXTENSIONCHANNEL_PRIORITY",
    ),
    (
        "SignalSystem.BtExtensionChannel.RequireActiveSession",
        "SLSKD_SIGNALSYSTEM_BTEXTENSIONCHANNEL_REQUIREACTIVESESSION",
    ),
    ("flags.force_migrations", "SLSKD_FORCE_MIGRATIONS"),
    (
        "flags.legacy_windows_tcp_keepalive",
        "SLSKD_LEGACY_WINDOWS_TCP_KEEPALIVE",
    ),
    ("flags.log_sql", "SLSKD_LOG_SQL"),
    (
        "flags.log_unobserved_exceptions",
        "SLSKD_LOG_UNOBSERVED_EXCEPTIONS",
    ),
    (
        "flags.optimistic_relay_file_info",
        "SLSKD_OPTIMISTIC_RELAY_FILE_INFO",
    ),
    ("flags.volatile", "SLSKD_VOLATILE"),
    ("rooms", "SLSKD_ROOMS"),
    ("logger.disk", "SLSKD_DISK_LOGGER"),
    ("logger.loki", "SLSKD_LOKI"),
    ("logger.no_color", "SLSKD_NO_COLOR"),
    (
        "shares.cache.storage_mode",
        "SLSKD_SHARE_CACHE_STORAGE_MODE",
    ),
    ("shares.cache.workers", "SLSKD_SHARE_CACHE_WORKERS"),
    ("shares.cache.retention", "SLSKD_SHARE_CACHE_RETENTION"),
    (
        "shares.probe_media_attributes",
        "SLSKD_SHARES_PROBE_MEDIA_ATTRIBUTES",
    ),
    ("soulseek.liked_interests", "SLSKD_SLSK_LIKED_INTERESTS"),
    ("soulseek.hated_interests", "SLSKD_SLSK_HATED_INTERESTS"),
    ("wishlist.enabled", "SLSKD_WISHLIST_ENABLED"),
    ("wishlist.interval_seconds", "SLSKD_WISHLIST_INTERVAL"),
    ("wishlist.auto_download", "SLSKD_WISHLIST_AUTO_DOWNLOAD"),
    ("wishlist.max_results", "SLSKD_WISHLIST_MAX_RESULTS"),
    (
        "throttling.search.incoming.concurrency",
        "SLSKD_THROTTLING_SEARCH_INCOMING_CONCURRENCY",
    ),
    (
        "throttling.search.incoming.circuit_breaker",
        "SLSKD_THROTTLING_SEARCH_INCOMING_CIRCUIT_BREAKER",
    ),
    (
        "throttling.search.incoming.response_file_limit",
        "SLSKD_THROTTLING_SEARCH_INCOMING_RESPONSE_FILE_LIMIT",
    ),
    ("permissions.file.mode", "SLSKD_FILE_PERMISSION_MODE"),
    ("telemetry.tracing.enabled", "SLSKD_TELEMETRY_TRACING"),
    (
        "telemetry.tracing.exporter",
        "SLSKD_TELEMETRY_TRACING_EXPORTER",
    ),
    (
        "telemetry.tracing.jaeger_endpoint",
        "SLSKD_TELEMETRY_JAEGER_ENDPOINT",
    ),
    (
        "telemetry.tracing.jaeger_port",
        "SLSKD_TELEMETRY_JAEGER_PORT",
    ),
    (
        "telemetry.tracing.otlp_endpoint",
        "SLSKD_TELEMETRY_OTLP_ENDPOINT",
    ),
    ("retention.search", "SLSKR_RETENTION_SEARCH"),
    ("retention.logs", "SLSKR_RETENTION_LOGS"),
    ("retention.files.complete", "SLSKR_RETENTION_FILES_COMPLETE"),
    (
        "retention.files.incomplete",
        "SLSKR_RETENTION_FILES_INCOMPLETE",
    ),
    (
        "retention.transfers.upload.succeeded",
        "SLSKR_RETENTION_UPLOAD_SUCCEEDED",
    ),
    (
        "retention.transfers.upload.errored",
        "SLSKR_RETENTION_UPLOAD_ERRORED",
    ),
    (
        "retention.transfers.upload.cancelled",
        "SLSKR_RETENTION_UPLOAD_CANCELLED",
    ),
    (
        "retention.transfers.upload.failed",
        "SLSKR_RETENTION_UPLOAD_FAILED",
    ),
    (
        "retention.transfers.download.succeeded",
        "SLSKR_RETENTION_DOWNLOAD_SUCCEEDED",
    ),
    (
        "retention.transfers.download.errored",
        "SLSKR_RETENTION_DOWNLOAD_ERRORED",
    ),
    (
        "retention.transfers.download.cancelled",
        "SLSKR_RETENTION_DOWNLOAD_CANCELLED",
    ),
    (
        "retention.transfers.download.failed",
        "SLSKR_RETENTION_DOWNLOAD_FAILED",
    ),
    (
        "filters.search_retention.max_age_days",
        "SLSKD_SEARCH_RETENTION_MAX_AGE_DAYS",
    ),
    (
        "filters.search_retention.max_count",
        "SLSKD_SEARCH_RETENTION_MAX_COUNT",
    ),
    (
        "filters.search_retention.cleanup_interval_seconds",
        "SLSKD_SEARCH_RETENTION_CLEANUP_INTERVAL",
    ),
    ("metrics.enabled", "SLSKD_METRICS"),
    ("metrics.url", "SLSKD_METRICS_URL"),
    ("metrics.authentication.disabled", "SLSKD_METRICS_NO_AUTH"),
    ("metrics.authentication.username", "SLSKD_METRICS_USERNAME"),
    ("metrics.authentication.password", "SLSKD_METRICS_PASSWORD"),
    (
        "web.max_request_body_size",
        "SLSKD_WEB_MAX_REQUEST_BODY_SIZE",
    ),
    ("web.socket", "SLSKD_HTTP_SOCKET"),
    ("web.url_base", "SLSKD_URL_BASE"),
    ("web.content_path", "SLSKD_CONTENT_PATH"),
    ("web.logging", "SLSKD_HTTP_LOGGING"),
    ("web.https.disabled", "SLSKD_NO_HTTPS"),
    ("web.https.port", "SLSKD_HTTPS_PORT"),
    ("web.https.ip_address", "SLSKD_HTTPS_IP_ADDRESS"),
    ("web.https.force", "SLSKD_HTTPS_FORCE"),
    ("web.https.certificate.pfx", "SLSKD_HTTPS_CERT_PFX"),
    (
        "web.https.certificate.password",
        "SLSKD_HTTPS_CERT_PASSWORD",
    ),
    ("web.enforce_security", "SLSKD_ENFORCE_SECURITY"),
    ("web.allow_remote_no_auth", "SLSKD_ALLOW_REMOTE_NO_AUTH"),
    ("web.authentication.disabled", "SLSKR_AUTH_DISABLED"),
    ("web.authentication.username", "SLSKD_USERNAME"),
    ("web.authentication.password", "SLSKD_PASSWORD"),
    ("web.authentication.jwt.key", "SLSKD_JWT_KEY"),
    ("web.authentication.jwt.ttl", "SLSKD_JWT_TTL"),
    (
        "flags.hash_from_audio_file_enabled",
        "SLSKR_CONTROLLER_YAML_HASH_FROM_AUDIO_FILE_ENABLED",
    ),
    ("diagnostics.allow_memory_dump", "SLSKD_ALLOW_MEMORY_DUMP"),
    ("diagnostics.allow_remote_dump", "SLSKD_ALLOW_REMOTE_DUMP"),
    (
        "web.authentication.passthrough.allowed_cidrs",
        "SLSKD_PASSTHROUGH_ALLOWED_CIDRS",
    ),
    ("web.cors.enabled", "SLSKD_WEB_CORS_ENABLED"),
    (
        "web.cors.allow_credentials",
        "SLSKD_WEB_CORS_ALLOW_CREDENTIALS",
    ),
    ("web.cors.allowed_origins", "SLSKD_WEB_CORS_ALLOWED_ORIGINS"),
    ("web.cors.allowed_headers", "SLSKD_WEB_CORS_ALLOWED_HEADERS"),
    ("web.cors.allowed_methods", "SLSKD_WEB_CORS_ALLOWED_METHODS"),
    ("web.rate_limiting.enabled", "SLSKD_WEB_RATE_LIMITING"),
    (
        "web.rate_limiting.api_permit_limit",
        "SLSKD_WEB_API_PERMIT_LIMIT",
    ),
    (
        "web.rate_limiting.api_window_seconds",
        "SLSKD_WEB_API_WINDOW_SECONDS",
    ),
    (
        "web.rate_limiting.federation_permit_limit",
        "SLSKD_WEB_FEDERATION_PERMIT_LIMIT",
    ),
    (
        "web.rate_limiting.federation_window_seconds",
        "SLSKD_WEB_FEDERATION_WINDOW_SECONDS",
    ),
    (
        "web.rate_limiting.mesh_gateway_permit_limit",
        "SLSKD_WEB_MESH_GATEWAY_PERMIT_LIMIT",
    ),
    (
        "web.rate_limiting.mesh_gateway_window_seconds",
        "SLSKD_WEB_MESH_GATEWAY_WINDOW_SECONDS",
    ),
    ("instance_name", "SLSKD_INSTANCE_NAME"),
    ("directories.downloads", "SLSKD_DOWNLOADS_DIR"),
    ("directories.incomplete", "SLSKD_INCOMPLETE_DIR"),
    ("dht.dht_port", "SLSKR_DHT_PORT"),
    ("dht.enabled", "SLSKR_DHT_ENABLED"),
    ("debug", "SLSKD_DEBUG"),
    ("flags.no_config_watch", "SLSKD_NO_CONFIG_WATCH"),
    ("flags.no_connect", "SLSKD_NO_CONNECT"),
    ("flags.no_logo", "SLSKD_NO_LOGO"),
    ("flags.no_start", "SLSKD_NO_START"),
    ("flags.no_version_check", "SLSKD_NO_VERSION_CHECK"),
    ("flags.experimental", "SLSKD_EXPERIMENTAL"),
    ("flags.case_sensitive_reg_ex", "SLSKD_CASE_SENSITIVE_REGEX"),
    ("flags.no_share_scan", "SLSKD_NO_SHARE_SCAN"),
    ("flags.force_share_scan", "SLSKD_FORCE_SHARE_SCAN"),
    ("remote_configuration", "SLSKD_REMOTE_CONFIGURATION"),
    ("remote_file_management", "SLSKD_REMOTE_FILE_MANAGEMENT"),
    ("shares.directories", "SLSKD_SHARED_DIR"),
    ("shares.filters", "SLSKD_SHARE_FILTER"),
    ("filters.search.request", "SLSKD_SEARCH_REQUEST_FILTER"),
    ("filters.download.exclude", "SLSKR_DOWNLOAD_FILTER_EXCLUDE"),
    ("auto_replace.interval_seconds", "AUTO_REPLACE_INTERVAL"),
    (
        "auto_replace.size_threshold_percent",
        "AUTO_REPLACE_THRESHOLD",
    ),
    ("auto_replace.max_retries", "AUTO_REPLACE_MAX_RETRIES"),
    (
        "transfers.groups.blacklisted.members",
        "SLSKD_BLACKLISTED_MEMBERS",
    ),
    (
        "transfers.groups.blacklisted.patterns",
        "SLSKD_BLACKLISTED_PATTERNS",
    ),
    (
        "transfers.groups.blacklisted.cidrs",
        "SLSKD_BLACKLISTED_CIDRS",
    ),
    ("groups.blacklisted.members", "SLSKD_BLACKLISTED_MEMBERS"),
    ("groups.blacklisted.patterns", "SLSKD_BLACKLISTED_PATTERNS"),
    ("groups.blacklisted.cidrs", "SLSKD_BLACKLISTED_CIDRS"),
    ("soulseek.address", "SLSKD_SLSK_ADDRESS"),
    ("soulseek.port", "SLSKD_SLSK_PORT"),
    ("soulseek.username", "SLSKD_SLSK_USERNAME"),
    ("soulseek.password", "SLSKD_SLSK_PASSWORD"),
    ("soulseek.description", "SLSKD_SLSK_DESCRIPTION"),
    ("soulseek.picture", "SLSKD_SLSK_PICTURE"),
    ("soulseek.diagnostic_level", "SLSKD_SLSK_DIAG_LEVEL"),
    (
        "soulseek.distributed_network.disabled",
        "SLSKD_SLSK_NO_DNET",
    ),
    (
        "soulseek.distributed_network.disable_children",
        "SLSKD_SLSK_DNET_NO_CHILDREN",
    ),
    (
        "soulseek.distributed_network.child_limit",
        "SLSKD_SLSK_DNET_CHILDREN",
    ),
    (
        "soulseek.distributed_network.logging",
        "SLSKD_SLSK_DNET_LOGGING",
    ),
    ("soulseek.listen_ip_address", "SLSKD_SLSK_LISTEN_IP_ADDRESS"),
    ("soulseek.listen_port", "SLSKD_SLSK_LISTEN_PORT"),
    ("soulseek.connection.buffer.read", "SLSKD_SLSK_READ_BUFFER"),
    (
        "soulseek.connection.buffer.write",
        "SLSKD_SLSK_WRITE_BUFFER",
    ),
    (
        "soulseek.connection.buffer.transfer",
        "SLSKD_SLSK_TRANSFER_BUFFER",
    ),
    (
        "soulseek.connection.buffer.write_queue",
        "SLSKD_SLSK_WRITE_QUEUE",
    ),
    (
        "soulseek.connection.timeout.connect",
        "SLSKD_SLSK_CONNECTION_TIMEOUT",
    ),
    (
        "soulseek.connection.timeout.inactivity",
        "SLSKD_SLSK_INACTIVITY_TIMEOUT",
    ),
    (
        "soulseek.connection.timeout.transfer",
        "SLSKD_SLSK_TRANSFER_TIMEOUT",
    ),
    (
        "soulseek.connection.proxy.enabled",
        "SLSKD_SLSK_PROXY_ENABLED",
    ),
    (
        "soulseek.connection.proxy.address",
        "SLSKD_SLSK_PROXY_ADDRESS",
    ),
    ("soulseek.connection.proxy.port", "SLSKD_SLSK_PROXY_PORT"),
    (
        "soulseek.connection.proxy.username",
        "SLSKD_SLSK_PROXY_USERNAME",
    ),
    (
        "soulseek.connection.proxy.password",
        "SLSKD_SLSK_PROXY_PASSWORD",
    ),
    ("soulseek.obfuscation.enabled", "SLSKD_SLSK_OBFUSCATION"),
    ("soulseek.obfuscation.mode", "SLSKD_SLSK_OBFUSCATION_MODE"),
    (
        "soulseek.obfuscation.listen_port",
        "SLSKD_SLSK_OBFUSCATION_LISTEN_PORT",
    ),
    (
        "soulseek.obfuscation.advertise_regular_port",
        "SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT",
    ),
    (
        "soulseek.obfuscation.prefer_outbound",
        "SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND",
    ),
    (
        "soulseek.private_message_auto_response.cooldown_minutes",
        "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES",
    ),
    (
        "soulseek.private_message_auto_response.enabled",
        "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE",
    ),
    (
        "soulseek.private_message_auto_response.message",
        "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE",
    ),
    ("integrations.lidarr.api_key", "SLSKD_LIDARR_API_KEY"),
    ("integrations.lidarr.enabled", "SLSKD_LIDARR"),
    (
        "integrations.lidarr.timeout_seconds",
        "SLSKD_LIDARR_TIMEOUT",
    ),
    ("integrations.lidarr.url", "SLSKD_LIDARR_URL"),
    (
        "integrations.lidarr.sync_wanted_to_wishlist",
        "SLSKD_LIDARR_SYNC_WANTED",
    ),
    (
        "integrations.lidarr.sync_interval_seconds",
        "SLSKD_LIDARR_SYNC_INTERVAL",
    ),
    (
        "integrations.lidarr.max_items_per_sync",
        "SLSKD_LIDARR_SYNC_MAX_ITEMS",
    ),
    (
        "integrations.lidarr.auto_download",
        "SLSKD_LIDARR_AUTO_DOWNLOAD",
    ),
    (
        "integrations.lidarr.wishlist_filter",
        "SLSKD_LIDARR_WISHLIST_FILTER",
    ),
    (
        "integrations.lidarr.wishlist_max_results",
        "SLSKD_LIDARR_WISHLIST_MAX_RESULTS",
    ),
    (
        "integrations.lidarr.auto_import_completed",
        "SLSKD_LIDARR_AUTO_IMPORT_COMPLETED",
    ),
    (
        "integrations.lidarr.import_path_from",
        "SLSKD_LIDARR_IMPORT_PATH_FROM",
    ),
    (
        "integrations.lidarr.import_path_to",
        "SLSKD_LIDARR_IMPORT_PATH_TO",
    ),
    (
        "integrations.lidarr.import_mode",
        "SLSKD_LIDARR_IMPORT_MODE",
    ),
    (
        "integrations.lidarr.import_replace_existing_files",
        "SLSKD_LIDARR_IMPORT_REPLACE_EXISTING",
    ),
    (
        "integrations.lidarr.import_delay_seconds",
        "SLSKR_LIDARR_IMPORT_DELAY",
    ),
    (
        "integrations.lidarr.import_retry_max_attempts",
        "SLSKR_LIDARR_IMPORT_RETRY_MAX_ATTEMPTS",
    ),
    (
        "integrations.lidarr.import_retry_delay_seconds",
        "SLSKR_LIDARR_IMPORT_RETRY_DELAY",
    ),
    (
        "integrations.lidarr.skip_already_owned_albums",
        "SLSKR_LIDARR_SKIP_ALREADY_OWNED_ALBUMS",
    ),
    (
        "integrations.lidarr.delete_rejected_downloads",
        "SLSKD_LIDARR_DELETE_REJECTED_DOWNLOADS",
    ),
    (
        "integrations.lidarr.blacklist_rejected_downloads",
        "SLSKD_LIDARR_BLACKLIST_REJECTED_DOWNLOADS",
    ),
    (
        "integrations.chromaprint.enabled",
        "SLSKD_CHROMAPRINT_ENABLED",
    ),
    (
        "integrations.chromaprint.algorithm",
        "SLSKD_CHROMAPRINT_ALGORITHM",
    ),
    (
        "integrations.chromaprint.ffmpeg_path",
        "SLSKD_CHROMAPRINT_FFMPEG_PATH",
    ),
    (
        "integrations.chromaprint.sample_rate",
        "SLSKD_CHROMAPRINT_SAMPLE_RATE",
    ),
    (
        "integrations.chromaprint.channels",
        "SLSKD_CHROMAPRINT_CHANNELS",
    ),
    (
        "integrations.chromaprint.duration_seconds",
        "SLSKD_CHROMAPRINT_DURATION_SECONDS",
    ),
    ("integrations.acoustid.enabled", "SLSKD_ACOUSTID_ENABLED"),
    (
        "integrations.acoustid.client_id",
        "SLSKD_ACOUSTID_CLIENT_ID",
    ),
    ("integrations.acoustid.base_url", "SLSKD_ACOUSTID_BASE_URL"),
    (
        "integrations.musicbrainz.base_url",
        "SLSKD_MUSICBRAINZ_BASE_URL",
    ),
    (
        "integrations.musicbrainz.baseUrl",
        "SLSKD_MUSICBRAINZ_BASE_URL",
    ),
    (
        "integrations.musicBrainz.baseUrl",
        "SLSKD_MUSICBRAINZ_BASE_URL",
    ),
    (
        "integration.musicBrainz.baseUrl",
        "SLSKD_MUSICBRAINZ_BASE_URL",
    ),
    (
        "integrations.musicbrainz.user_agent",
        "SLSKD_MUSICBRAINZ_USER_AGENT",
    ),
    (
        "integrations.musicbrainz.userAgent",
        "SLSKD_MUSICBRAINZ_USER_AGENT",
    ),
    (
        "integrations.musicBrainz.userAgent",
        "SLSKD_MUSICBRAINZ_USER_AGENT",
    ),
    (
        "integration.musicBrainz.userAgent",
        "SLSKD_MUSICBRAINZ_USER_AGENT",
    ),
    (
        "integrations.musicbrainz.timeout_seconds",
        "SLSKD_MUSICBRAINZ_TIMEOUT_SECONDS",
    ),
    (
        "integrations.musicbrainz.timeoutSeconds",
        "SLSKD_MUSICBRAINZ_TIMEOUT_SECONDS",
    ),
    (
        "integrations.musicBrainz.timeoutSeconds",
        "SLSKD_MUSICBRAINZ_TIMEOUT_SECONDS",
    ),
    (
        "integration.musicBrainz.timeoutSeconds",
        "SLSKD_MUSICBRAINZ_TIMEOUT_SECONDS",
    ),
    (
        "integrations.musicbrainz.retry_attempts",
        "SLSKD_MUSICBRAINZ_RETRY_ATTEMPTS",
    ),
    (
        "integrations.musicbrainz.retryAttempts",
        "SLSKD_MUSICBRAINZ_RETRY_ATTEMPTS",
    ),
    (
        "integrations.musicBrainz.retryAttempts",
        "SLSKD_MUSICBRAINZ_RETRY_ATTEMPTS",
    ),
    (
        "integration.musicBrainz.retryAttempts",
        "SLSKD_MUSICBRAINZ_RETRY_ATTEMPTS",
    ),
    ("integrations.spotify.client_id", "SLSKD_SPOTIFY_CLIENT_ID"),
    (
        "integrations.spotify.client_secret",
        "SLSKD_SPOTIFY_CLIENT_SECRET",
    ),
    ("integrations.spotify.enabled", "SLSKD_SPOTIFY"),
    ("integrations.spotify.market", "SLSKD_SPOTIFY_MARKET"),
    (
        "integrations.spotify.redirect_uri",
        "SLSKD_SPOTIFY_REDIRECT_URI",
    ),
    (
        "integrations.spotify.timeout_seconds",
        "SLSKD_SPOTIFY_TIMEOUT",
    ),
    (
        "integrations.spotify.max_items_per_import",
        "SLSKD_SPOTIFY_MAX_ITEMS_PER_IMPORT",
    ),
    (
        "solid.allow_insecure_http",
        "SLSKD_SOLID_ALLOW_INSECURE_HTTP",
    ),
    (
        "solid.allow_localhost_for_web_id",
        "SLSKD_SOLID_ALLOW_LOCALHOST_FOR_WEB_ID",
    ),
    ("solid.client_id_url", "SLSKD_SOLID_CLIENT_ID_URL"),
    ("integrations.youtube.enabled", "SLSKD_YOUTUBE"),
    ("integrations.youtube.api_key", "SLSKD_YOUTUBE_API_KEY"),
    ("integrations.lastfm.enabled", "SLSKD_LASTFM"),
    ("integrations.lastfm.api_key", "SLSKD_LASTFM_API_KEY"),
    ("integrations.ntfy.enabled", "SLSKD_NTFY"),
    ("integrations.ntfy.url", "SLSKD_NTFY_URL"),
    ("integrations.ntfy.access_token", "SLSKD_NTFY_TOKEN"),
    (
        "integrations.ntfy.notification_prefix",
        "SLSKD_NTFY_NOTIFICATION_PREFIX",
    ),
    (
        "integrations.ntfy.notify_on_private_message",
        "SLSKD_NTFY_NOTIFY_ON_PRIVATE_MESSAGE",
    ),
    (
        "integrations.ntfy.notify_on_room_mention",
        "SLSKD_NTFY_NOTIFY_ON_ROOM_MENTION",
    ),
    ("integrations.pushover.enabled", "SLSKD_PUSHOVER"),
    ("integrations.pushover.user_key", "SLSKD_PUSHOVER_USER_KEY"),
    ("integrations.pushover.token", "SLSKD_PUSHOVER_TOKEN"),
    (
        "integrations.pushover.notification_prefix",
        "SLSKD_PUSHOVER_NOTIFICATION_PREFIX",
    ),
    (
        "integrations.pushover.notify_on_private_message",
        "SLSKD_PUSHOVER_NOTIFY_ON_PRIVATE_MESSAGE",
    ),
    (
        "integrations.pushover.notify_on_room_mention",
        "SLSKD_PUSHOVER_NOTIFY_ON_ROOM_MENTION",
    ),
    ("integrations.pushbullet.enabled", "SLSKD_PUSHBULLET"),
    (
        "integrations.pushbullet.access_token",
        "SLSKD_PUSHBULLET_ACCESS_TOKEN",
    ),
    (
        "integrations.pushbullet.notification_prefix",
        "SLSKD_PUSHBULLET_NOTIFICATION_PREFIX",
    ),
    (
        "integrations.pushbullet.notify_on_private_message",
        "SLSKD_PUSHBULLET_NOTIFY_ON_PRIVATE_MESSAGE",
    ),
    (
        "integrations.pushbullet.notify_on_room_mention",
        "SLSKD_PUSHBULLET_NOTIFY_ON_ROOM_MENTION",
    ),
    (
        "integrations.pushbullet.retry_attempts",
        "SLSKD_PUSHBULLET_RETRY_ATTEMPTS",
    ),
    (
        "integrations.pushbullet.cooldown_time",
        "SLSKD_PUSHBULLET_COOLDOWN_TIME",
    ),
    ("integrations.ftp.enabled", "SLSKD_FTP"),
    ("integrations.ftp.address", "SLSKD_FTP_ADDRESS"),
    ("integrations.ftp.port", "SLSKD_FTP_PORT"),
    (
        "integrations.ftp.encryption_mode",
        "SLSKD_FTP_ENCRYPTION_MODE",
    ),
    (
        "integrations.ftp.ignore_certificate_errors",
        "SLSKD_FTP_IGNORE_CERTIFICATE_ERRORS",
    ),
    ("integrations.ftp.username", "SLSKD_FTP_USERNAME"),
    ("integrations.ftp.password", "SLSKD_FTP_PASSWORD"),
    ("integrations.ftp.remote_path", "SLSKD_FTP_REMOTE_PATH"),
    (
        "integrations.ftp.overwrite_existing",
        "SLSKD_FTP_OVERWRITE_EXISTING",
    ),
    (
        "integrations.ftp.connection_timeout",
        "SLSKD_FTP_CONNECTION_TIMEOUT",
    ),
    (
        "integrations.ftp.retry_attempts",
        "SLSKD_FTP_RETRY_ATTEMPTS",
    ),
    ("integrations.vpn.enabled", "SLSKD_VPN"),
    (
        "integrations.vpn.port_forwarding",
        "SLSKD_VPN_PORT_FORWARDING",
    ),
    (
        "integrations.vpn.self_hosted_relay",
        "SLSKD_VPN_SELF_HOSTED_RELAY",
    ),
    (
        "integrations.vpn.polling_interval",
        "SLSKD_VPN_POLLING_INTERVAL",
    ),
    ("integrations.vpn.gluetun.url", "SLSKD_VPN_GLUETUN_URL"),
    (
        "integrations.vpn.gluetun.timeout",
        "SLSKD_VPN_GLUETUN_TIMEOUT",
    ),
    (
        "integrations.vpn.gluetun.username",
        "SLSKD_VPN_GLUETUN_USERNAME",
    ),
    (
        "integrations.vpn.gluetun.password",
        "SLSKD_VPN_GLUETUN_PASSWORD",
    ),
    (
        "integrations.vpn.gluetun.api_key",
        "SLSKD_VPN_GLUETUN_API_KEY",
    ),
    ("transfers.download.slots", "SLSKD_DOWNLOAD_SLOTS"),
    (
        "transfers.download.speed_limit",
        "SLSKD_DOWNLOAD_SPEED_LIMIT",
    ),
    (
        "transfers.download.completed_layout",
        "SLSKD_DOWNLOAD_COMPLETED_LAYOUT",
    ),
    (
        "transfers.download.auto_replace_stuck",
        "SLSKD_AUTO_REPLACE_STUCK",
    ),
    (
        "transfers.download.auto_replace_threshold",
        "SLSKD_AUTO_REPLACE_THRESHOLD",
    ),
    (
        "transfers.download.auto_replace_interval",
        "SLSKD_AUTO_REPLACE_INTERVAL",
    ),
    (
        "transfers.download.completed_path_template",
        "SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE",
    ),
    (
        "transfers.download.auto_retry.alternate_source_size_tolerance_percent",
        "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
    ),
    (
        "transfers.download.auto_retry.alternate_sources_enabled",
        "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCES_ENABLED",
    ),
    (
        "transfers.download.auto_retry.check_interval_seconds",
        "SLSKR_TRANSFER_AUTO_RETRY_CHECK_INTERVAL_SECONDS",
    ),
    (
        "transfers.download.auto_retry.enabled",
        "SLSKR_TRANSFER_AUTO_RETRY_ENABLED",
    ),
    (
        "transfers.download.auto_retry.max_alternate_source_searches_per_cycle",
        "SLSKR_TRANSFER_AUTO_RETRY_MAX_ALTERNATE_SOURCE_SEARCHES_PER_CYCLE",
    ),
    (
        "transfers.download.auto_retry.max_attempts",
        "SLSKR_TRANSFER_AUTO_RETRY_MAX_ATTEMPTS",
    ),
    (
        "transfers.download.auto_retry.max_files_per_cycle",
        "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_CYCLE",
    ),
    (
        "transfers.download.auto_retry.max_files_per_peer_per_cycle",
        "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_PEER_PER_CYCLE",
    ),
    (
        "transfers.download.auto_retry.peer_cooldown_seconds",
        "SLSKR_TRANSFER_AUTO_RETRY_PEER_COOLDOWN_SECONDS",
    ),
    (
        "transfers.download.auto_retry.retry_delay_seconds",
        "SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS",
    ),
    ("web.ip_address", "SLSKD_HTTP_IP_ADDRESS"),
    ("web.address", "SLSKD_HTTP_ADDRESS"),
    ("web.port", "SLSKD_HTTP_PORT"),
];

pub(super) fn controller_yaml_environment(
    path: &std::path::Path,
    target: ControllerProfile,
) -> Result<BTreeMap<String, String>, String> {
    use std::io::Read;

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(BTreeMap::new());
        }
        Err(error) => return Err(format!("failed to inspect controller YAML: {error}")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("controller YAML must be a regular file".to_owned());
    }
    if metadata.len() > MAX_CONFIG_FILE_BYTES {
        return Err(format!(
            "controller YAML is too large: {} bytes, max is {MAX_CONFIG_FILE_BYTES}",
            metadata.len()
        ));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("failed to read controller YAML: {error}"))?;
    let mut body = String::new();
    file.by_ref()
        .take(MAX_CONFIG_FILE_BYTES + 1)
        .read_to_string(&mut body)
        .map_err(|error| format!("failed to read controller YAML: {error}"))?;
    if body.len() as u64 > MAX_CONFIG_FILE_BYTES {
        return Err(format!(
            "controller YAML is too large: max is {MAX_CONFIG_FILE_BYTES} bytes"
        ));
    }
    let mut root = serde_yaml::from_str::<serde_yaml::Value>(&body)
        .map_err(|_| "invalid controller YAML".to_owned())?;
    let mut nodes = 0;
    validate_controller_yaml_shape(&root, 0, &mut nodes)?;
    if !matches!(
        root,
        serde_yaml::Value::Mapping(_) | serde_yaml::Value::Null
    ) {
        return Err("controller YAML root must be a mapping".to_owned());
    }
    normalize_controller_yaml(&mut root);
    let mut normalized_nodes = 0;
    validate_controller_yaml_shape(&root, 0, &mut normalized_nodes)?;

    let mut values = BTreeMap::new();
    if target == ControllerProfile::Native {
        values.insert(
            "SLSKR_ADVANCED_NETWORKING_JSON".to_owned(),
            serde_json::to_string(&root)
                .map_err(|_| "invalid advanced networking controller YAML".to_owned())?,
        );
    }
    for (yaml_path, environment_name) in CONTROLLER_YAML_CORE_MAPPINGS {
        if matches!(
            *environment_name,
            "SLSKD_BLACKLISTED_MEMBERS" | "SLSKD_BLACKLISTED_PATTERNS" | "SLSKD_BLACKLISTED_CIDRS"
        ) {
            let target_path_matches = match target {
                ControllerProfile::Legacy => yaml_path.starts_with("transfers.groups.blacklisted."),
                ControllerProfile::Native => true,
            };
            if !target_path_matches {
                continue;
            }
        }
        let Some(value) = controller_yaml_value(&root, yaml_path)
            .or_else(|| controller_yaml_value_case_insensitive(&root, yaml_path))
        else {
            continue;
        };
        let value = match value {
            serde_yaml::Value::Null => continue,
            serde_yaml::Value::Bool(value) => value.to_string(),
            serde_yaml::Value::Number(value) => value.to_string(),
            serde_yaml::Value::String(value)
                if *environment_name == "SLSKD_INSTANCE_NAME" && value.is_empty() =>
            {
                continue;
            }
            serde_yaml::Value::String(value) => value.clone(),
            serde_yaml::Value::Sequence(values)
                if matches!(
                    *environment_name,
                    "SLSKD_SHARED_DIR"
                        | "SLSKD_SHARE_FILTER"
                        | "SLSKD_SEARCH_REQUEST_FILTER"
                        | "SLSKR_DOWNLOAD_FILTER_EXCLUDE"
                        | "SLSKD_BLACKLISTED_MEMBERS"
                        | "SLSKD_BLACKLISTED_PATTERNS"
                        | "SLSKD_BLACKLISTED_CIDRS"
                        | "SLSKD_WEB_CORS_ALLOWED_ORIGINS"
                        | "SLSKD_WEB_CORS_ALLOWED_HEADERS"
                        | "SLSKD_WEB_CORS_ALLOWED_METHODS"
                        | "FEDERATION_APPROVED_PEERS"
                        | "FEDERATION_PUBLISHING_PUBLISHABLE_DOMAINS"
                        | "FEDERATION_PUBLISHING_APPROVED_CIRCLES"
                        | "SLSKD_ROOMS"
                        | "SLSKD_SLSK_LIKED_INTERESTS"
                        | "SLSKD_SLSK_HATED_INTERESTS"
                ) =>
            {
                values
                    .iter()
                    .map(|value| {
                        if let Some(value) = value.as_str() {
                            return Ok(value.to_owned());
                        }
                        if matches!(
                            *environment_name,
                            "SLSKD_WEB_CORS_ALLOWED_ORIGINS"
                                | "SLSKD_WEB_CORS_ALLOWED_HEADERS"
                                | "SLSKD_WEB_CORS_ALLOWED_METHODS"
                        ) {
                            match value {
                                serde_yaml::Value::Bool(value) => return Ok(value.to_string()),
                                serde_yaml::Value::Number(value) => return Ok(value.to_string()),
                                serde_yaml::Value::Null => return Ok(String::new()),
                                _ => {}
                            }
                        }
                        Err(format!(
                            "invalid controller YAML value for {yaml_path}: expected string array"
                        ))
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(";")
            }
            _ => {
                return Err(format!(
                    "invalid controller YAML value for {yaml_path}: expected scalar"
                ));
            }
        };
        values.insert((*environment_name).to_owned(), value);
    }
    if let Some(webhooks) = controller_yaml_value(&root, "integrations.webhooks") {
        let json = serde_json::to_string(webhooks)
            .map_err(|_| "invalid integrations.webhooks configuration".to_owned())?;
        values.insert("SLSKR_FROZEN_WEBHOOKS_JSON".to_owned(), json);
    }
    if let Some(api_keys) = controller_yaml_value(&root, "web.authentication.api_keys") {
        let json = serde_json::to_string(api_keys)
            .map_err(|_| "invalid web.authentication.api_keys configuration".to_owned())?;
        values.insert("SLSKD_API_KEYS_JSON".to_owned(), json);
    }
    if let Some(destinations) = controller_yaml_value(&root, "destinations.folders") {
        let json = serde_json::to_string(destinations)
            .map_err(|_| "invalid destinations.folders configuration".to_owned())?;
        values.insert("SLSKD_DESTINATIONS_JSON".to_owned(), json);
    }
    if let Some(scripts) = controller_yaml_value(&root, "integrations.scripts") {
        let json = serde_json::to_string(scripts)
            .map_err(|_| "invalid integrations.scripts configuration".to_owned())?;
        values.insert("SLSKR_FROZEN_SCRIPTS_JSON".to_owned(), json);
    }
    let groups = match target {
        ControllerProfile::Legacy => controller_yaml_value(&root, "transfers.groups"),
        ControllerProfile::Native => controller_yaml_value(&root, "groups")
            .or_else(|| controller_yaml_value(&root, "transfers.groups")),
    };
    if let Some(groups) = groups {
        let json = serde_json::to_string(groups)
            .map_err(|_| "invalid transfers.groups configuration".to_owned())?;
        values.insert("SLSKR_FROZEN_TRANSFER_GROUPS_JSON".to_owned(), json);
    }
    if let Some(upload) = controller_yaml_value(&root, "transfers.upload") {
        let json = serde_json::to_string(upload)
            .map_err(|_| "invalid transfers.upload configuration".to_owned())?;
        values.insert("SLSKR_FROZEN_TRANSFER_UPLOAD_JSON".to_owned(), json);
    }
    if let Some(download) = controller_yaml_value(&root, "transfers.download") {
        let json = serde_json::to_string(download)
            .map_err(|_| "invalid transfers.download configuration".to_owned())?;
        values.insert("SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON".to_owned(), json);
    }
    Ok(values)
}

fn controller_yaml_value<'a>(
    root: &'a serde_yaml::Value,
    path: &str,
) -> Option<&'a serde_yaml::Value> {
    let mut current = root;
    for key in path.split('.') {
        let mapping = current.as_mapping()?;
        current = mapping.iter().find_map(|(candidate, value)| {
            candidate
                .as_str()
                .filter(|candidate| normalize_yaml_key(candidate) == normalize_yaml_key(key))
                .map(|_| value)
        })?;
    }
    Some(current)
}

fn controller_yaml_value_case_insensitive<'a>(
    root: &'a serde_yaml::Value,
    path: &str,
) -> Option<&'a serde_yaml::Value> {
    let mut current = root;
    for key in path.split('.') {
        let mapping = current.as_mapping()?;
        current = mapping.iter().find_map(|(candidate, value)| {
            candidate
                .as_str()
                .filter(|candidate| normalize_yaml_key(candidate) == normalize_yaml_key(key))
                .map(|_| value)
        })?;
    }
    Some(current)
}

fn normalize_yaml_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| !matches!(character, '_' | '-'))
        .flat_map(char::to_lowercase)
        .collect()
}

fn yaml_mapping_key(mapping: &serde_yaml::Mapping, name: &str) -> Option<serde_yaml::Value> {
    let normalized = normalize_yaml_key(name);
    mapping.keys().find_map(|key| {
        key.as_str()
            .filter(|candidate| normalize_yaml_key(candidate) == normalized)
            .map(|_| key.clone())
    })
}

fn yaml_mapping_value_mut<'a>(
    mapping: &'a mut serde_yaml::Mapping,
    name: &str,
) -> Option<&'a mut serde_yaml::Value> {
    let key = yaml_mapping_key(mapping, name)?;
    mapping.get_mut(&key)
}

fn yaml_take_mapping_value(
    mapping: &mut serde_yaml::Mapping,
    name: &str,
) -> Option<serde_yaml::Value> {
    let key = yaml_mapping_key(mapping, name)?;
    mapping.remove(&key)
}

fn merge_missing_yaml_values(target: &mut serde_yaml::Mapping, source: serde_yaml::Mapping) {
    for (key, value) in source {
        let Some(name) = key.as_str() else {
            continue;
        };
        if let Some(existing) = yaml_mapping_value_mut(target, name) {
            if let (serde_yaml::Value::Mapping(existing), serde_yaml::Value::Mapping(source)) =
                (existing, value)
            {
                merge_missing_yaml_values(existing, source);
            }
        } else {
            target.insert(key, value);
        }
    }
}

fn normalize_top_level_yaml_alias(
    root: &mut serde_yaml::Mapping,
    legacy_name: &str,
    canonical_name: &str,
) {
    let Some(legacy_value) = yaml_take_mapping_value(root, legacy_name) else {
        return;
    };
    if let Some(canonical) = yaml_mapping_value_mut(root, canonical_name) {
        if let (serde_yaml::Value::Mapping(canonical), serde_yaml::Value::Mapping(legacy)) =
            (canonical, legacy_value)
        {
            merge_missing_yaml_values(canonical, legacy);
        }
    } else {
        root.insert(
            serde_yaml::Value::String(canonical_name.to_owned()),
            legacy_value,
        );
    }
}

pub(super) fn normalize_controller_yaml(root: &mut serde_yaml::Value) {
    let Some(root) = root.as_mapping_mut() else {
        return;
    };

    normalize_top_level_yaml_alias(root, "global", "transfers");
    normalize_top_level_yaml_alias(root, "integration", "integrations");

    if let Some(shares) = yaml_mapping_value_mut(root, "shares") {
        if let serde_yaml::Value::Sequence(directories) = shares {
            let directories = std::mem::take(directories);
            let mut normalized = serde_yaml::Mapping::new();
            normalized.insert(
                serde_yaml::Value::String("directories".to_owned()),
                serde_yaml::Value::Sequence(directories),
            );
            *shares = serde_yaml::Value::Mapping(normalized);
        }
    }

    let upload_limits = {
        yaml_mapping_value_mut(root, "transfers")
            .and_then(serde_yaml::Value::as_mapping_mut)
            .and_then(|transfers| yaml_mapping_value_mut(transfers, "upload"))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .and_then(|upload| yaml_mapping_value_mut(upload, "limits"))
            .cloned()
    };
    if let Some(upload_limits) = upload_limits {
        if let Some(transfers) =
            yaml_mapping_value_mut(root, "transfers").and_then(serde_yaml::Value::as_mapping_mut)
        {
            if let Some(existing_limits) = yaml_mapping_value_mut(transfers, "limits") {
                if let (serde_yaml::Value::Mapping(existing), serde_yaml::Value::Mapping(upload)) =
                    (existing_limits, upload_limits)
                {
                    merge_missing_yaml_values(existing, upload);
                }
            } else {
                transfers.insert(
                    serde_yaml::Value::String("limits".to_owned()),
                    upload_limits,
                );
            }
        }
    }
}

pub(super) fn validate_controller_yaml_shape(
    value: &serde_yaml::Value,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), String> {
    if depth > 64 {
        return Err("controller YAML exceeds maximum nesting depth".to_owned());
    }
    *nodes = nodes.saturating_add(1);
    if *nodes > 65_536 {
        return Err("controller YAML contains too many values".to_owned());
    }
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            for (key, child) in mapping {
                if !matches!(key, serde_yaml::Value::String(_)) {
                    return Err("controller YAML keys must be strings".to_owned());
                }
                validate_controller_yaml_shape(child, depth + 1, nodes)?;
            }
        }
        serde_yaml::Value::Sequence(sequence) => {
            for child in sequence {
                validate_controller_yaml_shape(child, depth + 1, nodes)?;
            }
        }
        _ => {}
    }
    Ok(())
}
