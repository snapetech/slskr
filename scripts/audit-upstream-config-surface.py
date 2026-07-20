#!/usr/bin/env python3
"""Inventory the frozen slskd/slskdN documented and startup config surfaces."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path


KEY = re.compile(r"^(\s*)([A-Za-z_][A-Za-z0-9_-]*):(?:\s*(.*))?$")
TOML_TABLE = re.compile(r"^\s*\[\[?([^\]]+)\]\]?\s*(?:#.*)?$")
TOML_KEY = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_-]*)\s*=")
PROPERTY = re.compile(
    r"((?:\s*\[[^\]]+\]\s*)*)"
    r"public\s+[\w.<>,?\[\]]+\s+(\w+)\s*\{\s*get\s*;",
    re.MULTILINE,
)
EXPECTED = {
    "slskd": ("16e5d86ec9a91120f3ef40b85cb22036566b788a", 215, 126, 131, 60, 4),
    "slskdn": ("65a14a8b821de4df4ab7ef3ab3b156d7206837a3", 416, 214, 217, 74, 11),
}

# This list is deliberately narrow.  A path belongs here only when the frozen
# startup spelling reaches the real slskR subsystem, not merely the options
# projection.  YAML upload is tracked separately because accepting and echoing
# a key is not runtime parity.
SLSKR_RUNTIME_CONFIG_MAPPINGS: dict[str, dict[str, object]] = {
    "instance_name": {
        "environment": ["SLSKD_INSTANCE_NAME"],
        "commandLine": ["-i", "--instance-name"],
        "runtime": "instance identity in controller options and daemon diagnostics",
    },
    "headless": {
        "environment": ["SLSKD_HEADLESS"],
        "commandLine": ["-H", "--headless"],
        "runtime": "static UI suppression and browser-session login denial",
    },
    "directories.downloads": {
        "environment": ["SLSKD_DOWNLOADS_DIR"],
        "commandLine": ["-o", "--downloads"],
        "runtime": "completed download root, streaming, preview, and file-management storage",
    },
    "directories.incomplete": {
        "environment": ["SLSKD_INCOMPLETE_DIR"],
        "commandLine": ["--incomplete"],
        "runtime": "incomplete file-management storage root",
    },
    "dht.dht_port": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN DHT UDP bind and rendezvous service",
    },
    "dht.enabled": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN DHT rendezvous enablement",
    },
    "debug": {
        "environment": ["SLSKD_DEBUG"],
        "commandLine": ["-d", "--debug"],
        "runtime": "logging/debug posture",
    },
    "metrics.enabled": {
        "environment": ["SLSKD_METRICS"],
        "commandLine": ["--metrics"],
        "runtime": "startup-bound Prometheus route publication",
    },
    "metrics.url": {
        "environment": ["SLSKD_METRICS_URL"],
        "commandLine": ["--metrics-url"],
        "runtime": "startup-bound Prometheus route path",
    },
    "metrics.authentication.disabled": {
        "environment": ["SLSKD_METRICS_NO_AUTH"],
        "commandLine": ["--metrics-no-auth"],
        "runtime": "Prometheus route Basic-auth bypass",
    },
    "metrics.authentication.username": {
        "environment": ["SLSKD_METRICS_USERNAME"],
        "commandLine": ["--metrics-username"],
        "runtime": "Prometheus route Basic-auth username",
    },
    "metrics.authentication.password": {
        "environment": ["SLSKD_METRICS_PASSWORD"],
        "commandLine": ["--metrics-password"],
        "runtime": "Prometheus route Basic-auth secret",
    },
    "web.max_request_body_size": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN startup-bound HTTP request-body limit",
    },
    "web.enforce_security": {
        "environment": ["SLSKD_ENFORCE_SECURITY"],
        "commandLine": ["--enforce-security"],
        "runtime": "all frozen slskdN startup hardening rules, watched projection, restart failure, warnings, weak-metrics prevalidation, and unsupported-audio-hash rejection",
    },
    "diagnostics.allow_memory_dump": {
        "environment": ["SLSKD_ALLOW_MEMORY_DUMP"],
        "commandLine": [],
        "runtime": "slskdN gated administrator memory-dump stream and watched controller policy",
    },
    "diagnostics.allow_remote_dump": {
        "environment": ["SLSKD_ALLOW_REMOTE_DUMP"],
        "commandLine": [],
        "runtime": "slskdN loopback-only versus explicit remote memory-dump policy",
    },
    "web.allow_remote_no_auth": {
        "environment": ["SLSKD_ALLOW_REMOTE_NO_AUTH"],
        "commandLine": ["--allow-remote-no-auth"],
        "runtime": "slskdN startup-bound non-loopback passthrough authorization gate",
    },
    "web.authentication.disabled": {
        "environment": ["SLSKD_NO_AUTH"],
        "commandLine": ["-X", "--no-auth"],
        "runtime": "dual-target startup authentication/passthrough mode, session bootstrap, login, JWT authorization, and restart lifecycle",
    },
    "web.authentication.username": {
        "environment": ["SLSKD_USERNAME"],
        "commandLine": ["-u", "--username"],
        "runtime": "controller browser-session login username with live reload and dual-target differential proof",
    },
    "web.authentication.password": {
        "environment": ["SLSKD_PASSWORD"],
        "commandLine": ["-p", "--password"],
        "runtime": "controller browser-session login password with secret projection, live reload, validation, and dual-target differential proof",
    },
    "web.authentication.jwt.key": {
        "environment": ["SLSKD_JWT_KEY"],
        "commandLine": ["--jwt-key"],
        "runtime": "controller browser-session JWT signing key with secret projection, restart-only lifecycle, and dual-target differential proof",
    },
    "web.authentication.jwt.ttl": {
        "environment": ["SLSKD_JWT_TTL"],
        "commandLine": ["--jwt-ttl"],
        "runtime": "target-specific browser-session JWT lifetime with validation, restart-only lifecycle, and dual-target differential proof",
    },
    "web.authentication.passthrough.allowed_cidrs": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN startup-bound explicit remote no-auth CIDR allowlist",
    },
    "web.cors.enabled": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN startup-bound configured CORS middleware publication",
    },
    "web.cors.allow_credentials": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN configured CORS credential response header",
    },
    "web.cors.allowed_origins": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN explicit and wildcard CORS origin policy",
    },
    "web.cors.allowed_headers": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN configured or request-echoed preflight header policy",
    },
    "web.cors.allowed_methods": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN configured or request-echoed preflight method policy",
    },
    "web.rate_limiting.enabled": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN startup-bound global HTTP rate-limiter publication",
    },
    "web.rate_limiting.api_permit_limit": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN anonymous API fixed-window permit limit",
    },
    "web.rate_limiting.api_window_seconds": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN anonymous API fixed-window duration with zero-to-60 coercion",
    },
    "web.rate_limiting.federation_permit_limit": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN POST inbox fixed-window permit limit",
    },
    "web.rate_limiting.federation_window_seconds": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN POST inbox fixed-window duration with zero-to-60 coercion",
    },
    "web.rate_limiting.mesh_gateway_permit_limit": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN mesh-gateway fixed-window permit limit",
    },
    "web.rate_limiting.mesh_gateway_window_seconds": {
        "environment": [],
        "commandLine": [],
        "runtime": "slskdN mesh-gateway fixed-window duration with zero-to-60 coercion",
    },
    "flags.no_config_watch": {
        "environment": ["SLSKD_NO_CONFIG_WATCH"],
        "commandLine": ["--no-config-watch"],
        "runtime": "remote YAML restart signaling",
    },
    "flags.no_connect": {
        "environment": ["SLSKD_NO_CONNECT"],
        "commandLine": ["--no-connect"],
        "runtime": "Soulseek startup connection policy",
    },
    "flags.no_logo": {
        "environment": ["SLSKD_NO_LOGO"],
        "commandLine": ["-n", "--no-logo"],
        "runtime": "target-specific startup banner emission and suppression",
    },
    "flags.no_start": {
        "environment": ["SLSKD_NO_START"],
        "commandLine": ["-x", "--no-start"],
        "runtime": "successful post-bootstrap exit before hosted application and HTTP listener startup",
    },
    "flags.no_version_check": {
        "environment": ["SLSKD_NO_VERSION_CHECK"],
        "commandLine": ["--no-version-check"],
        "runtime": "target-specific startup GitHub release check suppression and version-state projection",
    },
    "flags.no_share_scan": {
        "environment": ["SLSKD_NO_SHARE_SCAN"],
        "commandLine": ["--no-share-scan"],
        "runtime": "startup share scanner suppression and uninitialized share state",
    },
    "flags.force_share_scan": {
        "environment": ["SLSKD_FORCE_SHARE_SCAN"],
        "commandLine": ["--force-share-scan"],
        "runtime": "startup share cache bypass and forced filesystem rescan",
    },
    "flags.experimental": {
        "environment": ["SLSKD_EXPERIMENTAL"],
        "commandLine": ["--experimental"],
        "runtime": "restart-only compatibility marker without independent feature activation",
    },
    "flags.case_sensitive_reg_ex": {
        "environment": ["SLSKD_CASE_SENSITIVE_REGEX"],
        "commandLine": ["--case-sensitive-regex"],
        "runtime": "startup regex mode for search/share filters and target-specific blacklist matching",
    },
    "remote_configuration": {
        "environment": ["SLSKD_REMOTE_CONFIGURATION"],
        "commandLine": ["--remote-configuration"],
        "runtime": "remote options authorization gate",
    },
    "remote_file_management": {
        "environment": ["SLSKD_REMOTE_FILE_MANAGEMENT"],
        "commandLine": ["--remote-file-management"],
        "runtime": "download and incomplete file-management delete authorization gate",
    },
    "shares.directories": {
        "environment": ["SLSKD_SHARED_DIR"],
        "commandLine": ["-s", "--shared"],
        "runtime": "startup share scanner, share list/search responses, and upload confinement",
    },
    "shares.filters": {
        "environment": ["SLSKD_SHARE_FILTER"],
        "commandLine": ["--share-filter"],
        "runtime": "original-path directory and file exclusion during share scans with .NET-style lookaround coverage",
    },
    "filters.search.request": {
        "environment": ["SLSKD_SEARCH_REQUEST_FILTER"],
        "commandLine": ["--search-request-filter"],
        "runtime": "incoming Soulseek search request suppression with .NET-style backreference coverage",
    },
    "transfers.groups.blacklisted.members": {
        "environment": [],
        "commandLine": [],
        "runtime": "target-specific runtime YAML placement and username blacklist matching",
    },
    "transfers.groups.blacklisted.patterns": {
        "environment": [],
        "commandLine": [],
        "runtime": "target-specific case mode and .NET-style named-backreference username matching",
    },
    "transfers.groups.blacklisted.cidrs": {
        "environment": [],
        "commandLine": [],
        "runtime": "target-specific runtime YAML placement and IPv4/IPv6 user blacklist matching",
    },
    "soulseek.address": {
        "environment": ["SLSKD_SLSK_ADDRESS"],
        "commandLine": ["--slsk-address"],
        "runtime": "Soulseek server endpoint",
    },
    "soulseek.port": {
        "environment": ["SLSKD_SLSK_PORT"],
        "commandLine": ["--slsk-port"],
        "runtime": "Soulseek server endpoint",
    },
    "soulseek.username": {
        "environment": ["SLSKD_SLSK_USERNAME"],
        "commandLine": ["--slsk-username"],
        "runtime": "Soulseek login credentials",
    },
    "soulseek.password": {
        "environment": ["SLSKD_SLSK_PASSWORD"],
        "commandLine": ["--slsk-password"],
        "runtime": "Soulseek login credentials",
    },
    "soulseek.description": {
        "environment": ["SLSKD_SLSK_DESCRIPTION"],
        "commandLine": ["--slsk-description"],
        "runtime": "Soulseek user-info response",
    },
    "soulseek.listen_ip_address": {
        "environment": ["SLSKD_SLSK_LISTEN_IP_ADDRESS"],
        "commandLine": ["--slsk-listen-ip-address"],
        "runtime": "regular inbound listener bind",
    },
    "soulseek.listen_port": {
        "environment": ["SLSKD_SLSK_LISTEN_PORT"],
        "commandLine": ["--slsk-listen-port"],
        "runtime": "regular inbound listener bind and advertisement",
    },
    "soulseek.obfuscation.enabled": {
        "environment": ["SLSKD_SLSK_OBFUSCATION"],
        "commandLine": ["--slsk-obfuscation"],
        "runtime": "type-1 listener and outbound dial policy",
    },
    "soulseek.obfuscation.mode": {
        "environment": ["SLSKD_SLSK_OBFUSCATION_MODE"],
        "commandLine": ["--slsk-obfuscation-mode"],
        "runtime": "regular/obfuscated outbound ordering",
    },
    "soulseek.obfuscation.listen_port": {
        "environment": ["SLSKD_SLSK_OBFUSCATION_LISTEN_PORT"],
        "commandLine": ["--slsk-obfuscation-listen-port"],
        "runtime": "type-1 listener bind and advertisement",
    },
    "soulseek.obfuscation.prefer_outbound": {
        "environment": ["SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND"],
        "commandLine": ["--slsk-obfuscation-prefer-outbound"],
        "runtime": "type-1 outbound ordering",
    },
    "soulseek.obfuscation.advertise_regular_port": {
        "environment": ["SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT"],
        "commandLine": ["--slsk-obfuscation-advertise-regular-port"],
        "runtime": "validated regular-port fallback policy",
    },
    "soulseek.private_message_auto_response.cooldown_minutes": {
        "environment": ["SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES"],
        "commandLine": ["--slsk-private-message-auto-response-cooldown-minutes"],
        "runtime": "private-message auto-response cooldown",
    },
    "soulseek.private_message_auto_response.enabled": {
        "environment": ["SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE"],
        "commandLine": ["--slsk-private-message-auto-response"],
        "runtime": "private-message auto-response dispatch gate",
    },
    "soulseek.private_message_auto_response.message": {
        "environment": ["SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE"],
        "commandLine": ["--slsk-private-message-auto-response-message"],
        "runtime": "private-message auto-response payload",
    },
    "integrations.lidarr.api_key": {
        "environment": ["SLSKD_LIDARR_API_KEY"],
        "commandLine": ["--lidarr-api-key"],
        "runtime": "Lidarr authenticated client",
    },
    "integrations.lidarr.enabled": {
        "environment": ["SLSKD_LIDARR"],
        "commandLine": ["--lidarr"],
        "runtime": "Lidarr integration gate",
    },
    "integrations.lidarr.timeout_seconds": {
        "environment": ["SLSKD_LIDARR_TIMEOUT"],
        "commandLine": ["--lidarr-timeout"],
        "runtime": "Lidarr request deadline",
    },
    "integrations.lidarr.url": {
        "environment": ["SLSKD_LIDARR_URL"],
        "commandLine": ["--lidarr-url"],
        "runtime": "Lidarr service endpoint",
    },
    "integrations.lidarr.sync_wanted_to_wishlist": {
        "environment": ["SLSKD_LIDARR_SYNC_WANTED"],
        "commandLine": ["--lidarr-sync-wanted"],
        "runtime": "Lidarr wanted-to-wishlist synchronization gate",
    },
    "integrations.lidarr.sync_interval_seconds": {
        "environment": ["SLSKD_LIDARR_SYNC_INTERVAL"],
        "commandLine": ["--lidarr-sync-interval"],
        "runtime": "Lidarr wanted synchronization interval",
    },
    "integrations.lidarr.max_items_per_sync": {
        "environment": ["SLSKD_LIDARR_SYNC_MAX_ITEMS"],
        "commandLine": ["--lidarr-sync-max-items"],
        "runtime": "Lidarr wanted synchronization batch limit",
    },
    "integrations.lidarr.auto_download": {
        "environment": ["SLSKD_LIDARR_AUTO_DOWNLOAD"],
        "commandLine": ["--lidarr-auto-download"],
        "runtime": "Lidarr-created wishlist auto-download policy",
    },
    "integrations.lidarr.wishlist_filter": {
        "environment": ["SLSKD_LIDARR_WISHLIST_FILTER"],
        "commandLine": ["--lidarr-wishlist-filter"],
        "runtime": "Lidarr-created wishlist search filter",
    },
    "integrations.lidarr.wishlist_max_results": {
        "environment": ["SLSKD_LIDARR_WISHLIST_MAX_RESULTS"],
        "commandLine": ["--lidarr-wishlist-max-results"],
        "runtime": "Lidarr-created wishlist result limit",
    },
    "integrations.lidarr.auto_import_completed": {
        "environment": ["SLSKD_LIDARR_AUTO_IMPORT_COMPLETED"],
        "commandLine": ["--lidarr-auto-import-completed"],
        "runtime": "completed-download Lidarr import gate",
    },
    "integrations.lidarr.import_path_from": {
        "environment": ["SLSKD_LIDARR_IMPORT_PATH_FROM"],
        "commandLine": ["--lidarr-import-path-from"],
        "runtime": "local completed-path import mapping",
    },
    "integrations.lidarr.import_path_to": {
        "environment": ["SLSKD_LIDARR_IMPORT_PATH_TO"],
        "commandLine": ["--lidarr-import-path-to"],
        "runtime": "Lidarr-visible completed-path import mapping",
    },
    "integrations.lidarr.import_mode": {
        "environment": ["SLSKD_LIDARR_IMPORT_MODE"],
        "commandLine": ["--lidarr-import-mode"],
        "runtime": "Lidarr manual-import mode",
    },
    "integrations.lidarr.import_replace_existing_files": {
        "environment": ["SLSKD_LIDARR_IMPORT_REPLACE_EXISTING"],
        "commandLine": ["--lidarr-import-replace-existing"],
        "runtime": "Lidarr manual-import replacement policy",
    },
    "integrations.spotify.client_id": {
        "environment": ["SLSKD_SPOTIFY_CLIENT_ID"],
        "commandLine": ["--spotify-client-id"],
        "runtime": "Spotify OAuth client identity",
    },
    "integrations.spotify.client_secret": {
        "environment": ["SLSKD_SPOTIFY_CLIENT_SECRET"],
        "commandLine": ["--spotify-client-secret"],
        "runtime": "Spotify OAuth client secret",
    },
    "integrations.spotify.enabled": {
        "environment": ["SLSKD_SPOTIFY"],
        "commandLine": ["--spotify"],
        "runtime": "Spotify integration gate",
    },
    "integrations.spotify.market": {
        "environment": ["SLSKD_SPOTIFY_MARKET"],
        "commandLine": ["--spotify-market"],
        "runtime": "Spotify market selection",
    },
    "integrations.spotify.redirect_uri": {
        "environment": ["SLSKD_SPOTIFY_REDIRECT_URI"],
        "commandLine": ["--spotify-redirect-uri"],
        "runtime": "Spotify OAuth redirect URI",
    },
    "integrations.spotify.timeout_seconds": {
        "environment": ["SLSKD_SPOTIFY_TIMEOUT"],
        "commandLine": ["--spotify-timeout"],
        "runtime": "Spotify request deadline",
    },
    "integrations.spotify.max_items_per_import": {
        "environment": ["SLSKD_SPOTIFY_MAX_ITEMS_PER_IMPORT"],
        "commandLine": ["--spotify-max-items-per-import"],
        "runtime": "Spotify source import item limit",
    },
    "integrations.youtube.enabled": {
        "environment": ["SLSKD_YOUTUBE"],
        "commandLine": ["--youtube"],
        "runtime": "YouTube source-feed provider gate",
    },
    "integrations.youtube.api_key": {
        "environment": ["SLSKD_YOUTUBE_API_KEY"],
        "commandLine": ["--youtube-api-key"],
        "runtime": "YouTube playlist API credential",
    },
    "integrations.lastfm.enabled": {
        "environment": ["SLSKD_LASTFM"],
        "commandLine": ["--lastfm"],
        "runtime": "Last.fm source-feed provider gate",
    },
    "integrations.lastfm.api_key": {
        "environment": ["SLSKD_LASTFM_API_KEY"],
        "commandLine": ["--lastfm-api-key"],
        "runtime": "Last.fm source-feed API credential",
    },
    "integrations.ntfy.enabled": {"environment": ["SLSKD_NTFY"], "commandLine": ["--ntfy"], "runtime": "Ntfy notification delivery gate"},
    "integrations.ntfy.url": {"environment": ["SLSKD_NTFY_URL"], "commandLine": ["--ntfy-url"], "runtime": "Ntfy topic delivery URL"},
    "integrations.ntfy.access_token": {"environment": ["SLSKD_NTFY_TOKEN"], "commandLine": ["--ntfy-token"], "runtime": "Ntfy bearer authorization"},
    "integrations.ntfy.notification_prefix": {"environment": ["SLSKD_NTFY_NOTIFICATION_PREFIX"], "commandLine": ["--ntfy-prefix"], "runtime": "Ntfy title prefix"},
    "integrations.ntfy.notify_on_private_message": {"environment": ["SLSKD_NTFY_NOTIFY_ON_PRIVATE_MESSAGE"], "commandLine": ["--ntfy-notify-on-pm"], "runtime": "Ntfy private-message notification gate"},
    "integrations.ntfy.notify_on_room_mention": {"environment": ["SLSKD_NTFY_NOTIFY_ON_ROOM_MENTION"], "commandLine": ["--ntfy-notify-on-room-mention"], "runtime": "Ntfy room-mention notification gate"},
    "integrations.pushover.enabled": {"environment": ["SLSKD_PUSHOVER"], "commandLine": ["--pushover"], "runtime": "Pushover notification delivery gate"},
    "integrations.pushover.user_key": {"environment": ["SLSKD_PUSHOVER_USER_KEY"], "commandLine": ["--pushover-user-key"], "runtime": "Pushover recipient credential"},
    "integrations.pushover.token": {"environment": ["SLSKD_PUSHOVER_TOKEN"], "commandLine": ["--pushover-token"], "runtime": "Pushover application credential"},
    "integrations.pushover.notification_prefix": {"environment": ["SLSKD_PUSHOVER_NOTIFICATION_PREFIX"], "commandLine": ["--pushover-prefix"], "runtime": "Pushover title prefix"},
    "integrations.pushover.notify_on_private_message": {"environment": ["SLSKD_PUSHOVER_NOTIFY_ON_PRIVATE_MESSAGE"], "commandLine": ["--pushover-notify-on-pm"], "runtime": "Pushover private-message notification gate"},
    "integrations.pushover.notify_on_room_mention": {"environment": ["SLSKD_PUSHOVER_NOTIFY_ON_ROOM_MENTION"], "commandLine": ["--pushover-notify-on-room-mention"], "runtime": "Pushover room-mention notification gate"},
    "integrations.pushbullet.enabled": {"environment": ["SLSKD_PUSHBULLET"], "commandLine": ["--pushbullet"], "runtime": "Pushbullet delivery gate"},
    "integrations.pushbullet.access_token": {"environment": ["SLSKD_PUSHBULLET_ACCESS_TOKEN"], "commandLine": ["--pushbullet-token"], "runtime": "Pushbullet Access-Token header"},
    "integrations.pushbullet.notification_prefix": {"environment": ["SLSKD_PUSHBULLET_NOTIFICATION_PREFIX"], "commandLine": ["--pushbullet-prefix"], "runtime": "Pushbullet title prefix"},
    "integrations.pushbullet.notify_on_private_message": {"environment": ["SLSKD_PUSHBULLET_NOTIFY_ON_PRIVATE_MESSAGE"], "commandLine": ["--pushbullet-notify-on-pm"], "runtime": "Pushbullet private-message notification gate"},
    "integrations.pushbullet.notify_on_room_mention": {"environment": ["SLSKD_PUSHBULLET_NOTIFY_ON_ROOM_MENTION"], "commandLine": ["--pushbullet-notify-on-room-mention"], "runtime": "Pushbullet room-mention notification gate"},
    "integrations.pushbullet.retry_attempts": {"environment": ["SLSKD_PUSHBULLET_RETRY_ATTEMPTS"], "commandLine": ["--pushbullet-retry-attempts"], "runtime": "Pushbullet bounded retry attempts"},
    "integrations.pushbullet.cooldown_time": {"environment": ["SLSKD_PUSHBULLET_COOLDOWN_TIME"], "commandLine": ["--pushbullet-cooldown"], "runtime": "Pushbullet notification-key cooldown"},
    "integrations.ftp.enabled": {"environment": ["SLSKD_FTP"], "commandLine": ["--ftp"], "runtime": "Completed-download FTP upload gate"},
    "integrations.ftp.address": {"environment": ["SLSKD_FTP_ADDRESS"], "commandLine": ["--ftp-address"], "runtime": "FTP control and passive-data endpoint"},
    "integrations.ftp.port": {"environment": ["SLSKD_FTP_PORT"], "commandLine": ["--ftp-port"], "runtime": "FTP control endpoint port"},
    "integrations.ftp.encryption_mode": {"environment": ["SLSKD_FTP_ENCRYPTION_MODE"], "commandLine": ["--ftp-encryption-mode"], "runtime": "None, implicit, explicit, and auto FTPS negotiation"},
    "integrations.ftp.ignore_certificate_errors": {"environment": ["SLSKD_FTP_IGNORE_CERTIFICATE_ERRORS"], "commandLine": ["--ftp-ignore-certificate-errors"], "runtime": "Target-specific FTPS certificate policy"},
    "integrations.ftp.username": {"environment": ["SLSKD_FTP_USERNAME"], "commandLine": ["--ftp-username"], "runtime": "FTP USER authentication"},
    "integrations.ftp.password": {"environment": ["SLSKD_FTP_PASSWORD"], "commandLine": ["--ftp-password"], "runtime": "FTP PASS authentication"},
    "integrations.ftp.remote_path": {"environment": ["SLSKD_FTP_REMOTE_PATH"], "commandLine": ["--ftp-remote-path"], "runtime": "FTP parent-directory remote upload path"},
    "integrations.ftp.overwrite_existing": {"environment": ["SLSKD_FTP_OVERWRITE_EXISTING"], "commandLine": ["--ftp-overwrite-existing"], "runtime": "FTP overwrite versus existing-file skip"},
    "integrations.ftp.connection_timeout": {"environment": ["SLSKD_FTP_CONNECTION_TIMEOUT"], "commandLine": ["--ftp-connection-timeout"], "runtime": "FTP control/TLS connection deadline"},
    "integrations.ftp.retry_attempts": {"environment": ["SLSKD_FTP_RETRY_ATTEMPTS"], "commandLine": ["--ftp-retry-attempts"], "runtime": "Target-specific bounded FTP upload attempts"},
    "integrations.vpn.enabled": {"environment": ["SLSKD_VPN"], "commandLine": ["--vpn"], "runtime": "Startup-bound Gluetun polling and Soulseek VPN readiness gate"},
    "integrations.vpn.port_forwarding": {"environment": ["SLSKD_VPN_PORT_FORWARDING"], "commandLine": ["--vpn-port-forwarding"], "runtime": "Forwarded-port readiness requirement and Gluetun port discovery"},
    "integrations.vpn.polling_interval": {"environment": ["SLSKD_VPN_POLLING_INTERVAL"], "commandLine": ["--vpn-polling-interval"], "runtime": "Startup-captured Gluetun polling interval"},
    "integrations.vpn.gluetun.url": {"environment": ["SLSKD_VPN_GLUETUN_URL"], "commandLine": ["--vpn-gluetun-url"], "runtime": "Gluetun control-server request root"},
    "integrations.vpn.gluetun.timeout": {"environment": ["SLSKD_VPN_GLUETUN_TIMEOUT"], "commandLine": ["--vpn-gluetun-timeout"], "runtime": "Per-poll Gluetun HTTP timeout"},
    "integrations.vpn.gluetun.auth": {"environment": [], "commandLine": [], "runtime": "Documented compatibility leaf accepted and ignored like the frozen options model"},
    "integrations.vpn.gluetun.username": {"environment": ["SLSKD_VPN_GLUETUN_USERNAME"], "commandLine": ["--vpn-gluetun-username"], "runtime": "Gluetun Basic authentication username"},
    "integrations.vpn.gluetun.password": {"environment": ["SLSKD_VPN_GLUETUN_PASSWORD"], "commandLine": ["--vpn-gluetun-password"], "runtime": "Gluetun Basic authentication password"},
    "integrations.vpn.gluetun.api_key": {"environment": ["SLSKD_VPN_GLUETUN_API_KEY"], "commandLine": ["--vpn-gluetun-api-key"], "runtime": "Precedence-winning Gluetun X-API-Key authentication"},
    "integrations.scripts.run_command_with_linux_system_shell": {"environment": [], "commandLine": [], "runtime": "Dynamic script registration and system-shell execution"},
    "integrations.scripts.run_command_with_linux_system_shell.on": {"environment": [], "commandLine": [], "runtime": "Case-insensitive event and Any trigger selection"},
    "integrations.scripts.run_command_with_linux_system_shell.run.command": {"environment": [], "commandLine": [], "runtime": "Target-specific system-shell command mode"},
    "integrations.scripts.run_command_with_windows_system_shell": {"environment": [], "commandLine": [], "runtime": "Dynamic Windows system-shell script registration"},
    "integrations.scripts.run_command_with_windows_system_shell.on": {"environment": [], "commandLine": [], "runtime": "Case-insensitive Windows script event selection"},
    "integrations.scripts.run_command_with_windows_system_shell.run.command": {"environment": [], "commandLine": [], "runtime": "Windows cmd.exe command mode"},
    "integrations.scripts.run_with_sh.on": {"environment": [], "commandLine": [], "runtime": "Executable args-mode event selection"},
    "integrations.scripts.run_with_sh.run.args": {"environment": [], "commandLine": [], "runtime": "Quoted executable argument-string parsing"},
    "integrations.scripts.run_with_sh.run.executable": {"environment": [], "commandLine": [], "runtime": "Explicit script executable selection"},
    "integrations.scripts.run_with_sh_and_args_list.on": {"environment": [], "commandLine": [], "runtime": "Executable arglist-mode event selection"},
    "integrations.scripts.run_with_sh_and_args_list.run.arglist": {"environment": [], "commandLine": [], "runtime": "Lossless executable argument-list passing"},
    "integrations.scripts.run_with_sh_and_args_list.run.executable": {"environment": [], "commandLine": [], "runtime": "Explicit arglist-mode executable selection"},
    "integrations.scripts.run_with_windows_cmd.on": {"environment": [], "commandLine": [], "runtime": "Windows executable args-mode event selection"},
    "integrations.scripts.run_with_windows_cmd.run.args": {"environment": [], "commandLine": [], "runtime": "Windows executable argument-string passing"},
    "integrations.scripts.run_with_windows_cmd.run.executable": {"environment": [], "commandLine": [], "runtime": "Windows cmd.exe selection"},
    "integrations.scripts.run_with_windows_cmd_and_args_list.on": {"environment": [], "commandLine": [], "runtime": "Windows arglist-mode event selection"},
    "integrations.scripts.run_with_windows_cmd_and_args_list.run.arglist": {"environment": [], "commandLine": [], "runtime": "Windows lossless argument-list passing"},
    "integrations.scripts.run_with_windows_cmd_and_args_list.run.executable": {"environment": [], "commandLine": [], "runtime": "Windows arglist executable selection"},
    "integrations.scripts.run_with_windows_git_bash.on": {"environment": [], "commandLine": [], "runtime": "Windows Git Bash event selection"},
    "integrations.scripts.run_with_windows_git_bash.run.args": {"environment": [], "commandLine": [], "runtime": "Windows Git Bash argument-string passing"},
    "integrations.scripts.run_with_windows_git_bash.run.executable": {"environment": [], "commandLine": [], "runtime": "Windows Git Bash executable selection"},
    "integrations.webhooks.my_webhook.on": {"environment": [], "commandLine": [], "runtime": "Frozen webhook event selection"},
    "integrations.webhooks.my_webhook.call.url": {"environment": [], "commandLine": [], "runtime": "Frozen webhook guarded delivery URL"},
    "integrations.webhooks.my_webhook.call.headers.value": {"environment": [], "commandLine": [], "runtime": "Frozen webhook arbitrary outbound headers"},
    "integrations.webhooks.my_webhook.call.ignore_certificate_errors": {"environment": [], "commandLine": [], "runtime": "Frozen webhook certificate policy"},
    "integrations.webhooks.my_webhook.timeout": {"environment": [], "commandLine": [], "runtime": "Frozen webhook millisecond request timeout"},
    "integrations.webhooks.my_webhook.retry.attempts": {"environment": [], "commandLine": [], "runtime": "Frozen webhook bounded retry attempts"},
    "transfers.download.completed_path_template": {
        "environment": ["SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE"],
        "commandLine": ["--download-completed-path-template"],
        "runtime": "completed download destination layout",
    },
    "transfers.download.auto_retry.alternate_source_size_tolerance_percent": {
        "environment": [],
        "commandLine": [],
        "runtime": "alternate-source size matching tolerance",
    },
    "transfers.download.auto_retry.alternate_sources_enabled": {
        "environment": [],
        "commandLine": [],
        "runtime": "alternate-source discovery gate",
    },
    "transfers.download.auto_retry.check_interval_seconds": {
        "environment": [],
        "commandLine": [],
        "runtime": "automatic retry scheduler interval",
    },
    "transfers.download.auto_retry.enabled": {
        "environment": [],
        "commandLine": [],
        "runtime": "automatic download retry gate",
    },
    "transfers.download.auto_retry.max_alternate_source_searches_per_cycle": {
        "environment": [],
        "commandLine": [],
        "runtime": "alternate-source search cycle bound",
    },
    "transfers.download.auto_retry.max_attempts": {
        "environment": [],
        "commandLine": [],
        "runtime": "automatic retry attempt bound",
    },
    "transfers.download.auto_retry.max_files_per_cycle": {
        "environment": [],
        "commandLine": [],
        "runtime": "automatic retry cycle file bound",
    },
    "transfers.download.auto_retry.max_files_per_peer_per_cycle": {
        "environment": [],
        "commandLine": [],
        "runtime": "automatic retry per-peer cycle bound",
    },
    "transfers.download.auto_retry.peer_cooldown_seconds": {
        "environment": [],
        "commandLine": [],
        "runtime": "automatic retry peer cooldown",
    },
    "transfers.download.auto_retry.retry_delay_seconds": {
        "environment": [],
        "commandLine": [],
        "runtime": "automatic retry delay",
    },
    "web.ip_address": {
        "environment": ["SLSKD_HTTP_IP_ADDRESS"],
        "commandLine": ["--http-ip-address"],
        "runtime": "HTTP listener bind",
    },
    "web.port": {
        "environment": ["SLSKD_HTTP_PORT"],
        "commandLine": ["--http-port"],
        "runtime": "HTTP listener bind",
    },
}

TRANSFER_UPLOAD_GROUP_CONFIG_PATHS = {
    "transfers.groups.default.upload.limits.daily",
    "transfers.groups.default.upload.limits.queued.files",
    "transfers.groups.default.upload.limits.queued.megabytes",
    "transfers.groups.default.upload.limits.weekly.failures",
    "transfers.groups.default.upload.limits.weekly.files",
    "transfers.groups.default.upload.limits.weekly.megabytes",
    "transfers.groups.default.upload.priority",
    "transfers.groups.default.upload.slots",
    "transfers.groups.default.upload.strategy",
    "transfers.groups.leechers.thresholds.directories",
    "transfers.groups.leechers.thresholds.files",
    "transfers.groups.leechers.upload.limits.daily.failures",
    "transfers.groups.leechers.upload.limits.daily.files",
    "transfers.groups.leechers.upload.limits.daily.megabytes",
    "transfers.groups.leechers.upload.limits.queued.files",
    "transfers.groups.leechers.upload.limits.queued.megabytes",
    "transfers.groups.leechers.upload.limits.weekly.failures",
    "transfers.groups.leechers.upload.limits.weekly.files",
    "transfers.groups.leechers.upload.limits.weekly.megabytes",
    "transfers.groups.leechers.upload.priority",
    "transfers.groups.leechers.upload.slots",
    "transfers.groups.leechers.upload.speed_limit",
    "transfers.groups.leechers.upload.strategy",
    "transfers.groups.user_defined.my_buddies.members",
    "transfers.groups.user_defined.my_buddies.upload.limits.queued.files",
    "transfers.groups.user_defined.my_buddies.upload.priority",
    "transfers.groups.user_defined.my_buddies.upload.slots",
    "transfers.groups.user_defined.my_buddies.upload.strategy",
    "transfers.upload.limits.daily.failures",
    "transfers.upload.limits.daily.files",
    "transfers.upload.limits.daily.megabytes",
    "transfers.upload.limits.queued.files",
    "transfers.upload.limits.queued.megabytes",
    "transfers.upload.limits.weekly.failures",
    "transfers.upload.limits.weekly.files",
    "transfers.upload.limits.weekly.megabytes",
    "transfers.upload.slots",
    "transfers.upload.speed_limit",
}

for path in TRANSFER_UPLOAD_GROUP_CONFIG_PATHS:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path] = {
        "environment": [],
        "commandLine": [],
        "runtime": "dual-target upload admission limits, group classification, priority/strategy queue scheduling, slot allocation, aggregate pacing, queue position, protocol negotiation, and watched lifecycle",
    }
SLSKR_RUNTIME_CONFIG_MAPPINGS["transfers.upload.slots"].update(
    environment=["SLSKD_UPLOAD_SLOTS"], commandLine=["--upload-slots"]
)
SLSKR_RUNTIME_CONFIG_MAPPINGS["transfers.upload.speed_limit"].update(
    environment=["SLSKD_UPLOAD_SPEED_LIMIT"], commandLine=["--upload-speed-limit"]
)

TRANSFER_DOWNLOAD_CONFIG_PATHS = {
    "transfers.download.auto_replace_interval",
    "transfers.download.auto_replace_stuck",
    "transfers.download.auto_replace_threshold",
    "transfers.download.completed_layout",
    "transfers.download.destination.exists",
    "transfers.download.destination.permissions.mode",
    "transfers.download.destination.subdirectory",
    "transfers.download.retry.attempts",
    "transfers.download.retry.delay",
    "transfers.download.retry.incomplete",
    "transfers.download.retry.max_delay",
    "transfers.download.retry.partial",
    "transfers.download.slots",
    "transfers.download.speed_limit",
}

for path in TRANSFER_DOWNLOAD_CONFIG_PATHS:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path] = {
        "environment": [],
        "commandLine": [],
        "runtime": "dual-target download retry/resume, incomplete-to-complete movement, collision policy, destination layout/permissions, slot admission, aggregate pacing, auto-replacement, watched lifecycle, and restart behavior",
    }
for path, environment, command_line in [
    ("transfers.download.slots", "SLSKD_DOWNLOAD_SLOTS", "--download-slots"),
    (
        "transfers.download.speed_limit",
        "SLSKD_DOWNLOAD_SPEED_LIMIT",
        "--download-speed-limit",
    ),
    (
        "transfers.download.completed_layout",
        "SLSKD_DOWNLOAD_COMPLETED_LAYOUT",
        "--download-completed-layout",
    ),
    (
        "transfers.download.auto_replace_stuck",
        "SLSKD_AUTO_REPLACE_STUCK",
        "--auto-replace-stuck",
    ),
    (
        "transfers.download.auto_replace_threshold",
        "SLSKD_AUTO_REPLACE_THRESHOLD",
        "--auto-replace-threshold",
    ),
    (
        "transfers.download.auto_replace_interval",
        "SLSKD_AUTO_REPLACE_INTERVAL",
        "--auto-replace-interval",
    ),
]:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path].update(
        environment=[environment], commandLine=[command_line]
    )

SOULSEEK_CONNECTION_CONFIG_PATHS = {
    "soulseek.connection.buffer.read",
    "soulseek.connection.buffer.transfer",
    "soulseek.connection.buffer.write",
    "soulseek.connection.buffer.write_queue",
    "soulseek.connection.proxy.address",
    "soulseek.connection.proxy.enabled",
    "soulseek.connection.proxy.password",
    "soulseek.connection.proxy.port",
    "soulseek.connection.proxy.username",
    "soulseek.connection.timeout.connect",
    "soulseek.connection.timeout.inactivity",
    "soulseek.connection.timeout.transfer",
}

for path in SOULSEEK_CONNECTION_CONFIG_PATHS:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path] = {
        "environment": [],
        "commandLine": [],
        "runtime": "dual-target startup layering and validation, options/watch/restart lifecycle, SOCKS5 no-auth and username/password negotiation, server/direct/indirect/obfuscated/inbound socket application, bounded outbound write work, and control/transfer buffer and timeout enforcement",
    }
for path, environment, command_line in [
    ("soulseek.connection.buffer.read", "SLSK_READ_BUFFER", "--slsk-read-buffer"),
    ("soulseek.connection.buffer.write", "SLSK_WRITE_BUFFER", "--slsk-write-buffer"),
    ("soulseek.connection.buffer.transfer", "SLSK_TRANSFER_BUFFER", "--slsk-transfer-buffer"),
    ("soulseek.connection.buffer.write_queue", "SLSK_WRITE_QUEUE", "--slsk-write-queue"),
    ("soulseek.connection.timeout.connect", "SLSK_CONNECTION_TIMEOUT", "--slsk-connection-timeout"),
    ("soulseek.connection.timeout.inactivity", "SLSK_INACTIVITY_TIMEOUT", "--slsk-inactivity-timeout"),
    ("soulseek.connection.timeout.transfer", "SLSK_TRANSFER_TIMEOUT", "--slsk-transfer-timeout"),
    ("soulseek.connection.proxy.enabled", "SLSK_PROXY_ENABLED", "--slsk-proxy"),
    ("soulseek.connection.proxy.address", "SLSK_PROXY_ADDRESS", "--slsk-proxy-address"),
    ("soulseek.connection.proxy.port", "SLSK_PROXY_PORT", "--slsk-proxy-port"),
    ("soulseek.connection.proxy.username", "SLSK_PROXY_USERNAME", "--slsk-proxy-username"),
    ("soulseek.connection.proxy.password", "SLSK_PROXY_PASSWORD", "--slsk-proxy-password"),
]:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path].update(
        environment=[environment], commandLine=[command_line]
    )

SOULSEEK_PROFILE_DISTRIBUTED_CONFIG_PATHS = {
    "soulseek.picture",
    "soulseek.diagnostic_level",
    "soulseek.distributed_network.disabled",
    "soulseek.distributed_network.disable_children",
    "soulseek.distributed_network.child_limit",
    "soulseek.distributed_network.logging",
}

for path in SOULSEEK_PROFILE_DISTRIBUTED_CONFIG_PATHS:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path] = {
        "environment": [],
        "commandLine": [],
        "runtime": "dual-target startup, validation, projection, watched/restart lifecycle, live picture peer bytes and read-failure handling, diagnostic filtering, distributed parent/child topology, branch/depth propagation, capacity, search forwarding, application state, and socket interoperability",
    }
for path, environment, command_line in [
    ("soulseek.picture", "SLSK_PICTURE", "--slsk-picture"),
    ("soulseek.diagnostic_level", "SLSK_DIAG_LEVEL", "--slsk-diag-level"),
    ("soulseek.distributed_network.disabled", "SLSK_NO_DNET", "--slsk-no-dnet"),
    (
        "soulseek.distributed_network.disable_children",
        "SLSK_DNET_NO_CHILDREN",
        "--slsk-dnet-no-children",
    ),
    (
        "soulseek.distributed_network.child_limit",
        "SLSK_DNET_CHILDREN",
        "--slsk-dnet-children",
    ),
    (
        "soulseek.distributed_network.logging",
        "SLSK_DNET_LOGGING",
        "--slsk-dnet-logging",
    ),
]:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path].update(
        environment=[environment], commandLine=[command_line]
    )

DAEMON_FOUNDATION_CONFIG_PATHS = {
    "filters.search_retention.cleanup_interval_seconds",
    "filters.search_retention.max_age_days",
    "filters.search_retention.max_count",
    "flags.force_migrations",
    "flags.legacy_windows_tcp_keepalive",
    "flags.log_sql",
    "flags.log_unobserved_exceptions",
    "flags.optimistic_relay_file_info",
    "flags.volatile",
    "logger.disk",
    "logger.loki",
    "logger.no_color",
    "permissions.file.mode",
    "retention.files.complete",
    "retention.files.incomplete",
    "retention.logs",
    "retention.search",
    "retention.transfers.download.cancelled",
    "retention.transfers.download.errored",
    "retention.transfers.download.failed",
    "retention.transfers.download.succeeded",
    "retention.transfers.upload.cancelled",
    "retention.transfers.upload.errored",
    "retention.transfers.upload.failed",
    "retention.transfers.upload.succeeded",
    "telemetry.tracing.enabled",
    "telemetry.tracing.exporter",
    "telemetry.tracing.jaeger_endpoint",
    "telemetry.tracing.jaeger_port",
    "telemetry.tracing.otlp_endpoint",
    "web.authentication.api_keys.my_api_key.cidr",
    "web.authentication.api_keys.my_api_key.key",
    "web.authentication.api_keys.my_api_key.role",
    "web.authentication.passthrough",
    "web.content_path",
    "web.https.certificate.password",
    "web.https.certificate.pfx",
    "web.https.disabled",
    "web.https.force",
    "web.https.ip_address",
    "web.https.port",
    "web.logging",
    "web.rate_limiting",
    "web.socket",
    "web.url_base",
}

for path in DAEMON_FOUNDATION_CONFIG_PATHS:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path] = {
        "environment": [],
        "commandLine": [],
        "runtime": "dual-target startup layering, validation, secret-safe options projection, listener/runtime policy, retention execution, watched lifecycle, and configured-PFX differential proof",
    }
for path, environment, command_line in [
    ("flags.force_migrations", "SLSKD_FORCE_MIGRATIONS", "--force-migrations"),
    ("flags.legacy_windows_tcp_keepalive", "SLSKD_LEGACY_WINDOWS_TCP_KEEPALIVE", "--legacy-windows-tcp-keepalive"),
    ("flags.log_sql", "SLSKD_LOG_SQL", "--log-sql"),
    ("flags.log_unobserved_exceptions", "SLSKD_LOG_UNOBSERVED_EXCEPTIONS", "--log-unobserved-exceptions"),
    ("flags.optimistic_relay_file_info", "SLSKD_OPTIMISTIC_RELAY_FILE_INFO", "--optimistic-relay-file-info"),
    ("flags.volatile", "SLSKD_VOLATILE", "--volatile"),
    ("logger.disk", "SLSKD_DISK_LOGGER", "--disk-logger"),
    ("logger.loki", "SLSKD_LOKI", "--loki"),
    ("logger.no_color", "SLSKD_NO_COLOR", "--no-color"),
    ("permissions.file.mode", "SLSKD_FILE_PERMISSION_MODE", "--file-permission-mode"),
    ("telemetry.tracing.enabled", "SLSKD_TELEMETRY_TRACING", "--telemetry-tracing"),
    ("telemetry.tracing.exporter", "SLSKD_TELEMETRY_TRACING_EXPORTER", "--telemetry-tracing-exporter"),
    ("telemetry.tracing.jaeger_endpoint", "SLSKD_TELEMETRY_JAEGER_ENDPOINT", "--telemetry-jaeger-endpoint"),
    ("telemetry.tracing.jaeger_port", "SLSKD_TELEMETRY_JAEGER_PORT", "--telemetry-jaeger-port"),
    ("telemetry.tracing.otlp_endpoint", "SLSKD_TELEMETRY_OTLP_ENDPOINT", "--telemetry-otlp-endpoint"),
    ("filters.search_retention.cleanup_interval_seconds", "SLSKD_SEARCH_RETENTION_CLEANUP_INTERVAL", "--search-retention-cleanup-interval"),
    ("filters.search_retention.max_age_days", "SLSKD_SEARCH_RETENTION_MAX_AGE_DAYS", "--search-retention-max-age-days"),
    ("filters.search_retention.max_count", "SLSKD_SEARCH_RETENTION_MAX_COUNT", "--search-retention-max-count"),
    ("web.content_path", "SLSKD_CONTENT_PATH", "--content-path"),
    ("web.https.certificate.password", "SLSKD_HTTPS_CERT_PASSWORD", "--https-cert-password"),
    ("web.https.certificate.pfx", "SLSKD_HTTPS_CERT_PFX", "--https-cert-pfx"),
    ("web.https.disabled", "SLSKD_NO_HTTPS", "--no-https"),
    ("web.https.force", "SLSKD_HTTPS_FORCE", "--force-https"),
    ("web.https.ip_address", "SLSKD_HTTPS_IP_ADDRESS", "--https-ip-address"),
    ("web.https.port", "SLSKD_HTTPS_PORT", "--https-port"),
    ("web.logging", "SLSKD_HTTP_LOGGING", "--http-logging"),
    ("web.socket", "SLSKD_HTTP_SOCKET", "--http-socket"),
    ("web.url_base", "SLSKD_URL_BASE", "--url-base"),
]:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path].update(
        environment=[environment], commandLine=[command_line]
    )

for path, environment in [
    ("retention.search", "SLSKR_RETENTION_SEARCH"),
    ("retention.logs", "SLSKR_RETENTION_LOGS"),
    ("retention.files.complete", "SLSKR_RETENTION_FILES_COMPLETE"),
    ("retention.files.incomplete", "SLSKR_RETENTION_FILES_INCOMPLETE"),
    ("retention.transfers.upload.succeeded", "SLSKR_RETENTION_UPLOAD_SUCCEEDED"),
    ("retention.transfers.upload.errored", "SLSKR_RETENTION_UPLOAD_ERRORED"),
    ("retention.transfers.upload.cancelled", "SLSKR_RETENTION_UPLOAD_CANCELLED"),
    ("retention.transfers.upload.failed", "SLSKR_RETENTION_UPLOAD_FAILED"),
    ("retention.transfers.download.succeeded", "SLSKR_RETENTION_DOWNLOAD_SUCCEEDED"),
    ("retention.transfers.download.errored", "SLSKR_RETENTION_DOWNLOAD_ERRORED"),
    ("retention.transfers.download.cancelled", "SLSKR_RETENTION_DOWNLOAD_CANCELLED"),
    ("retention.transfers.download.failed", "SLSKR_RETENTION_DOWNLOAD_FAILED"),
]:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path]["environment"] = [environment]

CORE_WORKFLOW_CONFIG_PATHS = {
    "destinations.folders.default",
    "destinations.folders.path",
    "rooms",
    "shares.cache.retention",
    "shares.cache.storage_mode",
    "shares.cache.workers",
    "shares.probe_media_attributes",
    "soulseek.hated_interests",
    "soulseek.liked_interests",
    "throttling.search.incoming.circuit_breaker",
    "throttling.search.incoming.concurrency",
    "throttling.search.incoming.response_file_limit",
    "wishlist.auto_download",
    "wishlist.enabled",
    "wishlist.interval_seconds",
    "wishlist.max_results",
}

for path in CORE_WORKFLOW_CONFIG_PATHS:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path] = {
        "environment": [],
        "commandLine": [],
        "runtime": "dual-target startup layering, validation, exact options/API projection, watched lifecycle, destination selection, room replay, native interest publication, bounded wishlist scheduling, parallel/probing share scans, and incoming search admission/response limits",
    }
for path, environment, command_line in [
    ("rooms", "SLSKD_ROOMS", "--rooms"),
    ("shares.cache.retention", "SLSKD_SHARE_CACHE_RETENTION", "--share-cache-retention"),
    ("shares.cache.storage_mode", "SLSKD_SHARE_CACHE_STORAGE_MODE", "--share-cache-storage-mode"),
    ("shares.cache.workers", "SLSKD_SHARE_CACHE_WORKERS", "--share-cache-workers"),
    ("shares.probe_media_attributes", "SLSKD_SHARES_PROBE_MEDIA_ATTRIBUTES", "--shares-probe-media-attributes"),
    ("soulseek.hated_interests", "SLSKD_SLSK_HATED_INTERESTS", "--slsk-hated-interests"),
    ("soulseek.liked_interests", "SLSKD_SLSK_LIKED_INTERESTS", "--slsk-liked-interests"),
    ("throttling.search.incoming.circuit_breaker", "SLSKD_THROTTLING_SEARCH_INCOMING_CIRCUIT_BREAKER", "--throttling-search-incoming-circuit-breaker"),
    ("throttling.search.incoming.concurrency", "SLSKD_THROTTLING_SEARCH_INCOMING_CONCURRENCY", "--throttling-search-incoming-concurrency"),
    ("throttling.search.incoming.response_file_limit", "SLSKD_THROTTLING_SEARCH_INCOMING_RESPONSE_FILE_LIMIT", "--throttling-search-incoming-response-file-limit"),
    ("wishlist.auto_download", "SLSKD_WISHLIST_AUTO_DOWNLOAD", "--wishlist-auto-download"),
    ("wishlist.enabled", "SLSKD_WISHLIST_ENABLED", "--wishlist-enabled"),
    ("wishlist.interval_seconds", "SLSKD_WISHLIST_INTERVAL", "--wishlist-interval"),
    ("wishlist.max_results", "SLSKD_WISHLIST_MAX_RESULTS", "--wishlist-max-results"),
]:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path].update(
        environment=[environment], commandLine=[command_line]
    )

ADVANCED_NETWORKING_SECURITY_CONFIG_PATHS = {
    "Mesh",
    "Mesh.sync_security.alert_threshold_quarantine_events",
    "Mesh.sync_security.alert_threshold_rate_limit_violations",
    "Mesh.sync_security.alert_threshold_signature_failures",
    "Mesh.sync_security.consensus_min_agreements",
    "Mesh.sync_security.consensus_min_peers",
    "Mesh.sync_security.max_invalid_entries_per_window",
    "Mesh.sync_security.max_invalid_messages_per_window",
    "Mesh.sync_security.proof_of_possession_enabled",
    "Mesh.sync_security.quarantine_duration_minutes",
    "Mesh.sync_security.quarantine_violation_threshold",
    "Mesh.sync_security.rate_limit_window_minutes",
    "PodCore.Join.SignatureMode",
    "PodCore.Security.SignatureMode",
    "dht.advertised_overlay_port",
    "dht.announce_interval_seconds",
    "dht.bootstrap_routers",
    "dht.bootstrap_timeout_seconds",
    "dht.cold_bootstrap_timeout_seconds",
    "dht.discovery_interval_seconds",
    "dht.enable_stun",
    "dht.enable_upnp",
    "dht.lan_only",
    "dht.lan_only_bootstrap_timeout_seconds",
    "dht.min_neighbors",
    "dht.overlay_port",
    "dht.vpn_port_sync",
    "mesh.dht.bootstrap_nodes",
    "mesh.enable_soulseek_capability_handshake",
    "mesh.enable_soulseek_rendezvous",
    "mesh.enabled",
    "mesh.overlay.quic_port",
    "mesh.overlay.udp_port",
    "mesh.probe_soulseek_rendezvous_capabilities",
    "mesh.security.enforceRemotePayloadLimits",
    "mesh.security.maxRemotePayloadSize",
    "overlay.enable",
    "overlay.enable_quic",
    "overlay.listen_port",
    "overlay.quic_backend_listen_port",
    "overlay.quic_listen_port",
    "overlay.share_quic_with_dht_port",
    "overlay.trusted_certificate_pins",
    "overlay_data.allowed_relay_destinations",
    "overlay_data.enable",
    "overlay_data.listen_port",
    "overlay_data.max_concurrent_relays",
    "overlay_data.max_relay_bytes_per_direction",
    "overlay_data.max_relay_duration_seconds",
    "overlay_data.relay_authentication_token",
    "overlay_data.trusted_certificate_pins",
    "relay.agents.my_agent.cidr",
    "relay.agents.my_agent.instance_name",
    "relay.agents.my_agent.secret",
    "relay.controller.address",
    "relay.controller.api_key",
    "relay.controller.downloads",
    "relay.controller.ignore_certificate_errors",
    "relay.controller.secret",
    "relay.enabled",
    "relay.mode",
    "security.adversarial",
    "security.adversarial.anonymity",
    "security.adversarial.anonymity.relay_only.relay_authentication_token",
    "security.adversarial.anonymity.relay_only.relay_peer_data_endpoints",
    "security.adversarial.privacy.padding.max_padded_bytes",
    "security.adversarial.privacy.padding.max_unpadded_bytes",
    "security.content_safety.block_executables",
    "security.content_safety.enabled",
    "security.content_safety.quarantine_directory",
    "security.content_safety.quarantine_suspicious",
    "security.content_safety.verify_magic_bytes",
    "security.enabled",
    "security.network_guard.enabled",
    "security.network_guard.max_connections_per_ip",
    "security.network_guard.max_global_connections",
    "security.network_guard.max_message_size",
    "security.network_guard.max_messages_per_minute",
    "security.path_guard.enabled",
    "security.path_guard.max_path_depth",
    "security.path_guard.max_path_length",
    "security.peer_reputation.enabled",
    "security.peer_reputation.trusted_threshold",
    "security.peer_reputation.untrusted_threshold",
    "security.profile",
    "security.violation_tracker.base_ban_duration_minutes",
    "security.violation_tracker.enabled",
    "security.violation_tracker.violations_before_auto_ban",
}

for path in ADVANCED_NETWORKING_SECURITY_CONFIG_PATHS:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path] = {
        "environment": [],
        "commandLine": [],
        "runtime": "frozen slskdn startup layering and validation, exact options projection and watched lifecycle, DHT bootstrap/discovery/VPN advertisement, mesh capability and sync-security enforcement, Pod signature policy, overlay/relay bounds, network/path/content guards, reputation, violations, and adversarial limits",
    }
for path, environment, command_line in [
    ("relay.enabled", "SLSKD_RELAY", "--relay"),
    ("relay.mode", "SLSKD_RELAY_MODE", "--relay-mode"),
    ("relay.controller.address", "SLSKD_CONTROLLER_ADDRESS", "--controller-address"),
    ("relay.controller.ignore_certificate_errors", "SLSKD_CONTROLLER_IGNORE_CERTIFICATE_ERRORS", "--controller-ignore-certificate-errors"),
    ("relay.controller.api_key", "SLSKD_CONTROLLER_API_KEY", "--controller-api-key"),
    ("relay.controller.secret", "SLSKD_CONTROLLER_SECRET", "--controller-secret"),
    ("relay.controller.downloads", "SLSKD_CONTROLLER_DOWNLOADS", "--controller-downloads"),
]:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path].update(
        environment=[environment], commandLine=[command_line]
    )

MEDIA_ADVANCED_SERVICE_CONFIG_PATHS = {
    "feature.CollectionsSharing",
    "feature.Dht",
    "feature.IdentityFriends",
    "feature.Mesh",
    "feature.MeshParallelSearch",
    "feature.MeshPublishAvailability",
    "feature.MultiSourceDownloads",
    "feature.Pods",
    "feature.ScenePodBridge",
    "feature.ScenePodBridgeOptions.ExportPodAvailability",
    "feature.ScenePodBridgeOptions.ProxyTransfers",
    "feature.SocialFederation",
    "feature.Solid",
    "feature.SongId",
    "feature.Streaming",
    "feature.StreamingRelayFallback",
    "feature.VirtualSoulfind",
    "player.external_visualizer.arguments",
    "player.external_visualizer.enabled",
    "player.external_visualizer.name",
    "player.external_visualizer.path",
    "player.external_visualizer.working_directory",
    "solid.allowInsecureHttp",
    "solid.allowedHosts",
    "solid.maxFetchBytes",
    "solid.redirectPath",
    "solid.timeoutSeconds",
    "song_id.max_concurrent_runs",
    "virtualSoulfind.bridge.bindAddress",
    "virtualSoulfind.bridge.enabled",
    "virtualSoulfind.bridge.maxClients",
    "virtualSoulfind.bridge.maxRequestsPerMinute",
    "virtualSoulfind.bridge.maxTransfersPerSession",
    "virtualSoulfind.bridge.password",
    "virtualSoulfind.bridge.port",
    "virtualSoulfind.bridge.requireAuth",
    "virtualSoulfind.disasterMode.auto",
    "virtualSoulfind.disasterMode.enableGracefulDegradation",
    "virtualSoulfind.disasterMode.force",
    "virtualSoulfind.disasterMode.recoveryCheckIntervalMinutes",
    "virtualSoulfind.disasterMode.recoveryHealthyChecksRequired",
    "virtualSoulfind.disasterMode.unavailableThresholdMinutes",
}

for path in MEDIA_ADVANCED_SERVICE_CONFIG_PATHS:
    SLSKR_RUNTIME_CONFIG_MAPPINGS[path] = {
        "environment": [],
        "commandLine": [],
        "runtime": "frozen slskdn startup layering and validation, exact options/API projection, watched lifecycle, feature-route enforcement, bounded Solid fetching and SongID admission, external visualizer launch policy, and VirtualSoulfind bridge/disaster runtime policy",
    }
SLSKR_RUNTIME_CONFIG_MAPPINGS["song_id.max_concurrent_runs"].update(
    environment=["SLSKD_SONGID_MAX_CONCURRENT_RUNS"],
    commandLine=["--songid-max-concurrent-runs"],
)

# Paths enter this set only after startup layering, validation, live mutation,
# lifecycle state, explicit runtime application, failure paths, persistence,
# restart, and both frozen target profiles are covered by executable proof.
FULLY_PROVEN_CONFIG_PATHS = {
    "blacklist.enabled",
    "blacklist.file",
    "debug",
    "diagnostics.allow_memory_dump",
    "diagnostics.allow_remote_dump",
    "directories.downloads",
    "directories.incomplete",
    "dht.dht_port",
    "dht.enabled",
    "feature.swagger",
    "filters.search.request",
    "flags.case_sensitive_reg_ex",
    "flags.no_config_watch",
    "flags.no_connect",
    "flags.experimental",
    "flags.no_logo",
    "flags.no_start",
    "flags.no_version_check",
    "flags.no_share_scan",
    "flags.force_share_scan",
    "instance_name",
    "headless",
    "integrations.lidarr.api_key",
    "integrations.lidarr.auto_download",
    "integrations.lidarr.auto_import_completed",
    "integrations.lidarr.enabled",
    "integrations.lidarr.import_mode",
    "integrations.lidarr.import_path_from",
    "integrations.lidarr.import_path_to",
    "integrations.lidarr.import_replace_existing_files",
    "integrations.lidarr.max_items_per_sync",
    "integrations.lidarr.sync_interval_seconds",
    "integrations.lidarr.sync_wanted_to_wishlist",
    "integrations.lidarr.timeout_seconds",
    "integrations.lidarr.url",
    "integrations.lidarr.wishlist_filter",
    "integrations.lidarr.wishlist_max_results",
    "integrations.spotify.client_id",
    "integrations.spotify.client_secret",
    "integrations.spotify.enabled",
    "integrations.spotify.market",
    "integrations.spotify.redirect_uri",
    "integrations.spotify.timeout_seconds",
    "integrations.spotify.max_items_per_import",
    "integrations.youtube.api_key",
    "integrations.youtube.enabled",
    "integrations.lastfm.api_key",
    "integrations.lastfm.enabled",
    "integrations.ntfy.enabled",
    "integrations.ntfy.url",
    "integrations.ntfy.access_token",
    "integrations.ntfy.notification_prefix",
    "integrations.ntfy.notify_on_private_message",
    "integrations.ntfy.notify_on_room_mention",
    "integrations.pushover.enabled",
    "integrations.pushover.user_key",
    "integrations.pushover.token",
    "integrations.pushover.notification_prefix",
    "integrations.pushover.notify_on_private_message",
    "integrations.pushover.notify_on_room_mention",
    "integrations.pushbullet.enabled",
    "integrations.pushbullet.access_token",
    "integrations.pushbullet.notification_prefix",
    "integrations.pushbullet.notify_on_private_message",
    "integrations.pushbullet.notify_on_room_mention",
    "integrations.pushbullet.retry_attempts",
    "integrations.pushbullet.cooldown_time",
    "integrations.ftp.enabled",
    "integrations.ftp.address",
    "integrations.ftp.port",
    "integrations.ftp.encryption_mode",
    "integrations.ftp.ignore_certificate_errors",
    "integrations.ftp.username",
    "integrations.ftp.password",
    "integrations.ftp.remote_path",
    "integrations.ftp.overwrite_existing",
    "integrations.ftp.connection_timeout",
    "integrations.ftp.retry_attempts",
    "integrations.vpn.enabled",
    "integrations.vpn.port_forwarding",
    "integrations.vpn.polling_interval",
    "integrations.vpn.gluetun.url",
    "integrations.vpn.gluetun.timeout",
    "integrations.vpn.gluetun.auth",
    "integrations.vpn.gluetun.username",
    "integrations.vpn.gluetun.password",
    "integrations.vpn.gluetun.api_key",
    "integrations.scripts.run_command_with_linux_system_shell",
    "integrations.scripts.run_command_with_linux_system_shell.on",
    "integrations.scripts.run_command_with_linux_system_shell.run.command",
    "integrations.scripts.run_command_with_windows_system_shell",
    "integrations.scripts.run_command_with_windows_system_shell.on",
    "integrations.scripts.run_command_with_windows_system_shell.run.command",
    "integrations.scripts.run_with_sh.on",
    "integrations.scripts.run_with_sh.run.args",
    "integrations.scripts.run_with_sh.run.executable",
    "integrations.scripts.run_with_sh_and_args_list.on",
    "integrations.scripts.run_with_sh_and_args_list.run.arglist",
    "integrations.scripts.run_with_sh_and_args_list.run.executable",
    "integrations.scripts.run_with_windows_cmd.on",
    "integrations.scripts.run_with_windows_cmd.run.args",
    "integrations.scripts.run_with_windows_cmd.run.executable",
    "integrations.scripts.run_with_windows_cmd_and_args_list.on",
    "integrations.scripts.run_with_windows_cmd_and_args_list.run.arglist",
    "integrations.scripts.run_with_windows_cmd_and_args_list.run.executable",
    "integrations.scripts.run_with_windows_git_bash.on",
    "integrations.scripts.run_with_windows_git_bash.run.args",
    "integrations.scripts.run_with_windows_git_bash.run.executable",
    "integrations.webhooks.my_webhook.on",
    "integrations.webhooks.my_webhook.call.url",
    "integrations.webhooks.my_webhook.call.headers.value",
    "integrations.webhooks.my_webhook.call.ignore_certificate_errors",
    "integrations.webhooks.my_webhook.timeout",
    "integrations.webhooks.my_webhook.retry.attempts",
    "metrics.authentication.disabled",
    "metrics.authentication.password",
    "metrics.authentication.username",
    "metrics.enabled",
    "metrics.url",
    "web.enforce_security",
    "remote_configuration",
    "remote_file_management",
    "shares.directories",
    "shares.filters",
    "soulseek.address",
    "soulseek.description",
    "soulseek.listen_ip_address",
    "soulseek.listen_port",
    "soulseek.obfuscation.enabled",
    "soulseek.obfuscation.advertise_regular_port",
    "soulseek.obfuscation.listen_port",
    "soulseek.obfuscation.mode",
    "soulseek.obfuscation.prefer_outbound",
    "soulseek.password",
    "soulseek.port",
    "soulseek.private_message_auto_response.cooldown_minutes",
    "soulseek.private_message_auto_response.enabled",
    "soulseek.private_message_auto_response.message",
    "soulseek.username",
    "transfers.download.auto_retry.alternate_source_size_tolerance_percent",
    "transfers.download.auto_retry.alternate_sources_enabled",
    "transfers.download.auto_retry.check_interval_seconds",
    "transfers.download.auto_retry.enabled",
    "transfers.download.auto_retry.max_alternate_source_searches_per_cycle",
    "transfers.download.auto_retry.max_attempts",
    "transfers.download.auto_retry.max_files_per_cycle",
    "transfers.download.auto_retry.max_files_per_peer_per_cycle",
    "transfers.download.auto_retry.peer_cooldown_seconds",
    "transfers.download.auto_retry.retry_delay_seconds",
    "transfers.groups.blacklisted.cidrs",
    "transfers.groups.blacklisted.members",
    "transfers.groups.blacklisted.patterns",
    "transfers.download.completed_path_template",
    "web.allow_remote_no_auth",
    "web.authentication.disabled",
    "web.authentication.jwt.key",
    "web.authentication.jwt.ttl",
    "web.authentication.password",
    "web.authentication.passthrough.allowed_cidrs",
    "web.authentication.username",
    "web.cors.allow_credentials",
    "web.cors.allowed_headers",
    "web.cors.allowed_methods",
    "web.cors.allowed_origins",
    "web.cors.enabled",
    "web.max_request_body_size",
    "web.rate_limiting.api_permit_limit",
    "web.rate_limiting.api_window_seconds",
    "web.rate_limiting.enabled",
    "web.rate_limiting.federation_permit_limit",
    "web.rate_limiting.federation_window_seconds",
    "web.rate_limiting.mesh_gateway_permit_limit",
    "web.rate_limiting.mesh_gateway_window_seconds",
    "web.ip_address",
    "web.port",
}
FULLY_PROVEN_CONFIG_PATHS.update(TRANSFER_UPLOAD_GROUP_CONFIG_PATHS)
FULLY_PROVEN_CONFIG_PATHS.update(TRANSFER_DOWNLOAD_CONFIG_PATHS)
FULLY_PROVEN_CONFIG_PATHS.update(SOULSEEK_CONNECTION_CONFIG_PATHS)
FULLY_PROVEN_CONFIG_PATHS.update(SOULSEEK_PROFILE_DISTRIBUTED_CONFIG_PATHS)
FULLY_PROVEN_CONFIG_PATHS.update(DAEMON_FOUNDATION_CONFIG_PATHS)
FULLY_PROVEN_CONFIG_PATHS.update(CORE_WORKFLOW_CONFIG_PATHS)
FULLY_PROVEN_CONFIG_PATHS.update(ADVANCED_NETWORKING_SECURITY_CONFIG_PATHS)
FULLY_PROVEN_CONFIG_PATHS.update(MEDIA_ADVANCED_SERVICE_CONFIG_PATHS)


def uncomment_yaml_line(raw: str) -> str:
    stripped = raw.lstrip()
    if not stripped.startswith("#"):
        return raw
    marker = len(raw) - len(stripped)
    line = raw[:marker] + raw[marker + 1 :]
    if len(line) > marker and line[marker] == " ":
        line = line[:marker] + line[marker + 1 :]
    return line


def documented_yaml_paths(root: Path) -> list[str]:
    example = root / "config/slskd.example.yml"
    stack: list[tuple[int, str]] = []
    nodes: set[str] = set()
    explicit_values: set[str] = set()
    for raw in example.read_text(encoding="utf-8").splitlines():
        line = uncomment_yaml_line(raw)
        if not line.strip() or line.lstrip().startswith(("#", "-")):
            continue
        match = KEY.match(line)
        if not match:
            continue
        indent = len(match.group(1).replace("\t", "  "))
        key = match.group(2)
        value = (match.group(3) or "").strip()
        while stack and stack[-1][0] >= indent:
            stack.pop()
        full = ".".join([part for _, part in stack] + [key])
        nodes.add(full)
        if value and value not in {"|", ">", "~"}:
            explicit_values.add(full)
        stack.append((indent, key))
    leaves = explicit_values | {
        node for node in nodes if not any(other.startswith(f"{node}.") for other in nodes)
    }
    return sorted(leaves)


def attributed_startup_options(root: Path) -> dict[str, list[dict[str, object]]]:
    source = (root / "src/slskd/Core/Options.cs").read_text(encoding="utf-8")
    output: dict[str, list[dict[str, object]]] = {
        "environmentVariables": [],
        "commandLineArguments": [],
        "requiresRestart": [],
        "requiresReconnect": [],
    }
    for match in PROPERTY.finditer(source):
        attributes, property_name = match.groups()
        ignored = "[YamlIgnore]" in attributes and "[JsonIgnore]" in attributes
        for variable in re.findall(r'\[EnvironmentVariable\("([^"]+)"\)\]', attributes):
            output["environmentVariables"].append(
                {"name": variable, "property": property_name, "ignoredInYaml": ignored}
            )
        for raw_argument in re.findall(r"\[Argument\(([^\]]+)\)\]", attributes):
            names = re.findall(r'"([^"]+)"', raw_argument)
            output["commandLineArguments"].append(
                {
                    "names": names,
                    "property": property_name,
                    "ignoredInYaml": ignored,
                }
            )
        if "[RequiresRestart]" in attributes:
            output["requiresRestart"].append(
                {"property": property_name, "ignoredInYaml": ignored}
            )
        if "[RequiresReconnect]" in attributes:
            output["requiresReconnect"].append(
                {"property": property_name, "ignoredInYaml": ignored}
            )
    for values in output.values():
        values.sort(key=lambda row: json.dumps(row, sort_keys=True))
    return output


def revision(root: Path) -> str:
    return subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
    ).strip()


def target_inventory(root: Path) -> dict[str, object]:
    paths = documented_yaml_paths(root)
    startup = attributed_startup_options(root)
    return {
        "revision": revision(root),
        "documentedYamlLeafPaths": paths,
        "documentedYamlLeafPathCount": len(paths),
        **startup,
    }


def slskr_inventory(root: Path) -> dict[str, object]:
    example = root / "docs/slskr.config.example.toml"
    section = ""
    paths: set[str] = set()
    for raw in example.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line.startswith("#"):
            line = line[1:].lstrip()
        table = TOML_TABLE.match(line)
        if table:
            section = table.group(1)
            continue
        key = TOML_KEY.match(line)
        if key:
            paths.add(f"{section}.{key.group(1)}" if section else key.group(1))

    source_root = root / "crates/slskr/src"
    source = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted(source_root.glob("*.rs"))
    )
    environment = sorted(
        set(re.findall(r'"((?:SLSKR|SLSKD|SLSK)_[A-Z0-9_]+)"', source))
    )
    cli_source = (source_root / "cli.rs").read_text(encoding="utf-8")
    cli = sorted(set(re.findall(r'"(--[a-z][a-z0-9-]*)"', source)))
    command_source = cli_source[: cli_source.find("async fn overlay_service_probe")]
    commands = sorted(set(re.findall(r'Some\("([a-z][a-z0-9-]*)"\)', command_source)))
    return {
        "documentedTomlLeafPaths": sorted(paths),
        "documentedTomlLeafPathCount": len(paths),
        "environmentVariables": environment,
        "commandLineArguments": cli,
        "commands": commands,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--slskd-root", type=Path, required=True)
    parser.add_argument("--slskdn-root", type=Path, required=True)
    parser.add_argument(
        "--slskr-root", type=Path, default=Path(__file__).resolve().parent.parent
    )
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--check-frozen", action="store_true")
    parser.add_argument(
        "--require-complete",
        action="store_true",
        help="fail unless every frozen config leaf has full YAML, environment, CLI, and runtime proof",
    )
    args = parser.parse_args()

    slskd = target_inventory(args.slskd_root.resolve())
    slskdn = target_inventory(args.slskdn_root.resolve())
    slskr = slskr_inventory(args.slskr_root.resolve())
    slskd_paths = set(slskd["documentedYamlLeafPaths"])
    slskdn_paths = set(slskdn["documentedYamlLeafPaths"])
    union_paths = slskd_paths | slskdn_paths
    leaf_status = []
    for path in sorted(union_paths):
        mapping = SLSKR_RUNTIME_CONFIG_MAPPINGS.get(path)
        fully_proven = path in FULLY_PROVEN_CONFIG_PATHS
        leaf_status.append(
            {
                "path": path,
                "targets": [
                    name
                    for name, paths in (("slskd", slskd_paths), ("slskdn", slskdn_paths))
                    if path in paths
                ],
                # Known core mappings are bound during startup. A mapping is
                # complete only after its full live/restart differential is in
                # FULLY_PROVEN_CONFIG_PATHS. All other keys are
                # persistence/projection only.
                "yaml": (
                    "implemented"
                    if fully_proven
                    else "startup-implemented"
                    if mapping
                    else "projection-only"
                ),
                "environment": (
                    "implemented"
                    if mapping and mapping["environment"]
                    else "not-exposed"
                    if mapping
                    else "missing"
                ),
                "environmentNames": mapping["environment"] if mapping else [],
                "commandLine": (
                    "implemented"
                    if mapping and mapping["commandLine"]
                    else "not-exposed"
                    if mapping
                    else "missing"
                ),
                "commandLineNames": mapping["commandLine"] if mapping else [],
                "runtime": "implemented" if mapping else "missing",
                "runtimeEvidence": mapping["runtime"] if mapping else None,
                "overall": (
                    "implemented"
                    if fully_proven
                    else "partial"
                    if mapping
                    else "missing"
                ),
            }
        )
    status_counts = {
        status: sum(1 for row in leaf_status if row["overall"] == status)
        for status in ("implemented", "partial", "missing")
    }
    report = {
        "slskd": slskd,
        "slskdn": slskdn,
        "slskr": slskr,
        "comparison": {
            "unionCount": len(slskd_paths | slskdn_paths),
            "commonCount": len(slskd_paths & slskdn_paths),
            "slskdOnly": sorted(slskd_paths - slskdn_paths),
            "slskdnOnly": sorted(slskdn_paths - slskd_paths),
            "leafStatus": leaf_status,
            "leafStatusCounts": status_counts,
        },
    }
    if args.check_frozen:
        failures = []
        for name, expected in EXPECTED.items():
            target = report[name]
            actual = (
                target["revision"],
                target["documentedYamlLeafPathCount"],
                len(target["environmentVariables"]),
                len(target["commandLineArguments"]),
                len(target["requiresRestart"]),
                len(target["requiresReconnect"]),
            )
            if actual != expected:
                failures.append(f"{name}: expected {expected!r}, got {actual!r}")
        unknown_mappings = sorted(set(SLSKR_RUNTIME_CONFIG_MAPPINGS) - union_paths)
        if unknown_mappings:
            failures.append(
                f"slskr runtime config mappings are not frozen leaves: {unknown_mappings!r}"
            )
        slskr_environment = set(slskr["environmentVariables"])
        slskr_cli = set(slskr["commandLineArguments"])
        for path, mapping in SLSKR_RUNTIME_CONFIG_MAPPINGS.items():
            missing_environment = sorted(set(mapping["environment"]) - slskr_environment)
            missing_cli = sorted(
                flag
                for flag in mapping["commandLine"]
                if flag.startswith("--") and flag not in slskr_cli
            )
            if missing_environment:
                failures.append(
                    f"{path}: mapped environment names absent from slskr source: {missing_environment!r}"
                )
            if missing_cli:
                failures.append(
                    f"{path}: mapped CLI names absent from slskr source: {missing_cli!r}"
                )
        if failures:
            print("frozen upstream config inventory check failed", file=sys.stderr)
            print("\n".join(failures), file=sys.stderr)
            raise SystemExit(1)
    if args.require_complete:
        incomplete = [row for row in leaf_status if row["overall"] != "implemented"]
        if incomplete:
            print(
                "literal config parity check failed: "
                f"{len(incomplete)} of {len(leaf_status)} frozen leaves remain incomplete "
                f"({status_counts})",
                file=sys.stderr,
            )
            for row in incomplete[:25]:
                print(
                    f"  {row['path']}: yaml={row['yaml']} env={row['environment']} "
                    f"cli={row['commandLine']} runtime={row['runtime']}",
                    file=sys.stderr,
                )
            raise SystemExit(1)
    if args.json:
        print(json.dumps(report, indent=2))
        return
    print(
        "upstream config inventory: "
        f"slskd={len(slskd_paths)} documented YAML leaves, "
        f"slskdN={len(slskdn_paths)}, "
        f"common={len(slskd_paths & slskdn_paths)}, "
        f"union={len(slskd_paths | slskdn_paths)}; "
        f"slskr={slskr['documentedTomlLeafPathCount']} documented TOML leaves"
    )
    print(
        "literal leaf status: "
        f"implemented={status_counts['implemented']} "
        f"partial={status_counts['partial']} missing={status_counts['missing']}"
    )
    print(
        "startup attributes: "
        f"slskd env={len(slskd['environmentVariables'])} cli={len(slskd['commandLineArguments'])} "
        f"restart={len(slskd['requiresRestart'])} reconnect={len(slskd['requiresReconnect'])}; "
        f"slskdN env={len(slskdn['environmentVariables'])} cli={len(slskdn['commandLineArguments'])} "
        f"restart={len(slskdn['requiresRestart'])} reconnect={len(slskdn['requiresReconnect'])}; "
        f"slskr env={len(slskr['environmentVariables'])} cli_flags={len(slskr['commandLineArguments'])} "
        f"commands={len(slskr['commands'])}"
    )


if __name__ == "__main__":
    main()
