use super::*;

pub(super) fn health_response(config: &AppConfig) -> HttpResponse {
    let warnings = if auth_disabled_on_non_loopback(config) {
        serde_json::json!(["auth_disabled_non_loopback"])
    } else {
        serde_json::json!([])
    };
    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: serde_json::json!({
            "status": "ok",
            "service": "slskr",
            "warnings": warnings,
        })
        .to_string(),
    }
}

/// Matches the oracle's `MapHealthChecks("/health/mesh", ...)`, filtered to
/// checks tagged "mesh": reports whether the mesh subsystem is enabled.
pub(super) fn mesh_health_response(config: &AppConfig) -> HttpResponse {
    let enabled = config.advanced_networking.mesh.enabled;
    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: serde_json::json!({
            "status": if enabled { "ok" } else { "disabled" },
            "service": "slskr-mesh",
        })
        .to_string(),
    }
}

pub(super) fn federation_diagnostics_response(config: &AppConfig) -> HttpResponse {
    let federation = &config.social_federation;
    let publishing = &config.federation_publishing;
    let mode = federation.mode.as_str();
    let is_hermit = mode.eq_ignore_ascii_case("Hermit");
    let is_friends_only = mode.eq_ignore_ascii_case("FriendsOnly");
    let is_public = mode.eq_ignore_ascii_case("Public");
    let exposure = if !federation.enabled || is_hermit {
        "Hermit"
    } else if is_friends_only {
        "FriendsOnly"
    } else if is_public {
        "Public"
    } else {
        "Custom"
    };
    let mut warnings = Vec::new();
    if federation.enabled && federation.base_url.is_none() {
        warnings.push("Federation is enabled but federation.baseUrl is not configured.");
    }
    if federation.enabled && is_public && !federation.verify_signatures {
        warnings
            .push("Public federation is enabled while HTTP signature verification is disabled.");
    }
    if publishing.enabled && !federation.enabled {
        warnings.push("Federation publishing is enabled while social federation is disabled.");
    }
    if publishing.enabled
        && !publishing.require_moderation_approval
        && publishing.default_visibility.eq_ignore_ascii_case("public")
    {
        warnings.push("Federation publishing defaults to public without moderation approval.");
    }
    if config.advanced_networking.pod_join_signature_mode == PodSignatureMode::Off {
        warnings.push("Pod join signatures are not enforced.");
    }
    if config.advanced_networking.pod_security_signature_mode == PodSignatureMode::Off {
        warnings.push("Pod message signatures are not enforced.");
    }
    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: serde_json::json!({
            "federation": {
                "enabled": federation.enabled,
                "mode": federation.mode,
                "domainConfigured": federation.domain.is_some(),
                "baseUrlConfigured": federation.base_url.is_some(),
                "approvedPeerCount": federation.approved_peers.len(),
                "verifySignatures": federation.verify_signatures,
                "httpTimeoutSeconds": federation.http_timeout_seconds,
                "pageSize": federation.page_size,
                "exposure": exposure,
            },
            "publishing": {
                "enabled": publishing.enabled,
                "publishableDomains": publishing.publishable_domains,
                "defaultVisibility": publishing.default_visibility,
                "approvedCircleCount": publishing.approved_circles.len(),
                "requireModerationApproval": publishing.require_moderation_approval,
                "includeExternalLinks": publishing.include_external_links,
                "maxMetadataSizeKb": publishing.max_metadata_size_kb,
            },
            "pods": {
                "joinSignatureMode": pod_signature_mode_target_name(
                    config.advanced_networking.pod_join_signature_mode,
                ),
                "messageSignatureMode": pod_signature_mode_target_name(
                    config.advanced_networking.pod_security_signature_mode,
                ),
            },
            "mesh": {
                "selfPeerIdConfigured": true,
                "soulseekRendezvousEnabled": config
                    .advanced_networking
                    .mesh
                    .enable_soulseek_rendezvous,
            },
            "warnings": warnings,
        })
        .to_string(),
    }
}

pub(super) fn pod_signature_mode_target_name(mode: PodSignatureMode) -> &'static str {
    match mode {
        PodSignatureMode::Off => "Off",
        PodSignatureMode::Warn => "Warn",
        PodSignatureMode::Enforce => "Enforce",
    }
}

pub(super) fn auth_disabled_on_non_loopback(config: &AppConfig) -> bool {
    !config.auth_required
        && config
            .http_binds
            .iter()
            .any(|address| !address.ip().is_loopback())
}

pub(super) fn version_response() -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: format!(
            "{{\"name\":\"{}\",\"version\":\"{}\",\"protocol\":{{\"client_name\":\"{}\",\"major\":{},\"minor\":{}}}}}",
            CLIENT_NAME, APP_VERSION, CLIENT_NAME, CLIENT_MAJOR_VERSION, CLIENT_MINOR_VERSION
        ),
    }
}

pub(super) fn controller_swagger_index_response(config: &AppConfig) -> HttpResponse {
    if config.controller_swagger {
        HttpResponse {
            status: "200 OK",
            content_type: "text/html;charset=utf-8",
            body: openapi::frozen_swagger_ui_html(),
        }
    } else {
        controller_swagger_not_found_response()
    }
}

pub(super) fn controller_swagger_not_found_response() -> HttpResponse {
    HttpResponse {
        status: "404 Not Found",
        content_type: "",
        body: String::new(),
    }
}

pub(super) fn controller_metrics_path(config: &AppConfig) -> String {
    if config.controller_metrics_url.starts_with('/') {
        config.controller_metrics_url.clone()
    } else {
        format!("/{}", config.controller_metrics_url)
    }
}

pub(super) fn controller_metrics_unauthorized_response() -> HttpResponse {
    HttpResponse {
        status: "401 Unauthorized",
        content_type: "",
        body: String::new(),
    }
}

pub(super) fn controller_metrics_auth_failure(
    config: &AppConfig,
    authorization: Option<&str>,
) -> Option<HttpResponse> {
    if config.controller_metrics_auth_disabled {
        return None;
    }
    if config.controller_profile == ControllerProfile::Native
        && config.controller_metrics_password.trim().is_empty()
    {
        return Some(HttpResponse {
            status: "503 Service Unavailable",
            content_type: "",
            body: "Metrics endpoint unavailable: authentication password is not configured."
                .to_owned(),
        });
    }
    let Some(authorization) = authorization else {
        return Some(controller_metrics_unauthorized_response());
    };
    let valid = match config.controller_profile {
        ControllerProfile::Legacy => {
            let provided = authorization.split(' ').next_back().unwrap_or_default();
            let expected = STANDARD.encode(format!(
                "{}:{}",
                config.controller_metrics_username, config.controller_metrics_password
            ));
            authorization
                .get(..5)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("Basic"))
                && provided.eq_ignore_ascii_case(&expected)
        }
        ControllerProfile::Native => authorization
            .get(..6)
            .filter(|prefix| prefix.eq_ignore_ascii_case("Basic "))
            .and_then(|_| STANDARD.decode(authorization[6..].trim()).ok())
            .is_some_and(|provided| {
                super::constant_time_bytes_equal(
                    &provided,
                    format!(
                        "{}:{}",
                        config.controller_metrics_username, config.controller_metrics_password
                    )
                    .as_bytes(),
                )
            }),
    };
    (!valid).then(controller_metrics_unauthorized_response)
}
