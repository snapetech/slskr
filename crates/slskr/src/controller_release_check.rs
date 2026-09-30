use super::*;

pub(super) fn normalize_controller_release_version(version_or_tag: &str) -> String {
    let mut normalized = version_or_tag.trim();
    let refs_prefix = "refs/tags/";
    if normalized
        .get(..refs_prefix.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(refs_prefix))
    {
        normalized = &normalized[refs_prefix.len()..];
    }
    for prefix in ["build-main-", "build-dev-", "release-v", "v"] {
        if normalized
            .get(..prefix.len())
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
        {
            normalized = &normalized[prefix.len()..];
            break;
        }
    }
    normalized
        .split_once('+')
        .map_or(normalized, |(base, _)| base)
        .to_owned()
}

fn parse_native_release(version: &str) -> Option<(u64, u32)> {
    let lower = version.to_ascii_lowercase();
    let (release_line, remainder) = lower.split_once("-slskdn.")?;
    let release_line = if (8..=10).contains(&release_line.len())
        && release_line.bytes().all(|byte| byte.is_ascii_digit())
    {
        release_line.parse().ok()?
    } else {
        let components = release_line.split('.').collect::<Vec<_>>();
        if components.len() != 3
            || components.iter().any(|component| {
                component.is_empty() || !component.bytes().all(|byte| byte.is_ascii_digit())
            })
        {
            return None;
        }
        components.concat().parse().ok()?
    };
    let sequence = remainder
        .split(|character: char| !character.is_ascii_digit())
        .next()
        .filter(|value| !value.is_empty())?
        .parse()
        .ok()?;
    Some((release_line, sequence))
}

fn parse_dotnet_version(version: &str) -> Option<[i64; 4]> {
    let components = version.split('.').collect::<Vec<_>>();
    if !(2..=4).contains(&components.len()) {
        return None;
    }
    let mut parsed = [-1; 4];
    for (index, component) in components.iter().enumerate() {
        parsed[index] = component
            .parse::<i32>()
            .ok()
            .filter(|component| *component >= 0)
            .map(i64::from)?;
    }
    Some(parsed)
}

pub(super) fn is_newer_controller_release_available(current: &str, latest: &str) -> bool {
    let current = normalize_controller_release_version(current);
    let latest = normalize_controller_release_version(latest);
    if latest.is_empty() || current.eq_ignore_ascii_case(&latest) {
        return false;
    }
    if let (Some(current), Some(latest)) = (
        parse_native_release(&current),
        parse_native_release(&latest),
    ) {
        return latest > current;
    }
    if let (Some(current), Some(latest)) = (
        parse_dotnet_version(&current),
        parse_dotnet_version(&latest),
    ) {
        return latest > current;
    }
    !current.is_empty()
}

pub(super) async fn start_controller_version_check(state: Arc<AppState>) {
    if state.config.controller_no_version_check {
        return;
    }
    if state.config.controller_profile == ControllerProfile::Legacy {
        record_daemon_log(
            &state,
            logging::LogLevel::Info,
            "version",
            "Skipping version check for Development build",
        )
        .await;
        return;
    }
    record_daemon_log(
        &state,
        logging::LogLevel::Info,
        "version",
        "Checking GitHub Releases for latest version",
    )
    .await;
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let _ = refresh_controller_version_check(
            &task_state,
            controller_releases_url(task_state.config.controller_profile),
        )
        .await;
    });
}

pub(super) fn controller_releases_url(target: ControllerProfile) -> &'static str {
    match target {
        ControllerProfile::Legacy => "https://api.github.com/repos/slskd/slskd/releases/latest",
        ControllerProfile::Native => "https://api.github.com/repos/snapetech/slskr/releases/latest",
    }
}

pub(super) async fn controller_version_latest_response(
    state: &AppState,
    force_check: bool,
    releases_url: &str,
) -> HttpResponse {
    if force_check {
        let check = refresh_controller_version_check(state, releases_url).await;
        if check.is_err() && state.config.controller_profile == ControllerProfile::Legacy {
            // slskd's ApplicationController awaits CheckVersionAsync without
            // catching its network failure; ASP.NET therefore emits a 500.
            // native profile catches the same failure and returns its current state.
            return routing::internal_server_error_response("failed to check application version");
        }
    }
    routing::ok_response(controller_version_json(state).to_string())
}

/// Performs one real GitHub Releases lookup and updates `controller_version`
/// -- matches the oracle's `Application.CheckVersionAsync`. Shared by the
/// startup check (fire-and-forget via `start_controller_version_check`) and
/// `GET .../version/latest?forceCheck=true`, which awaits it directly so
/// the response reflects a freshly fetched latest release. The returned
/// error is retained for slskd's controller-specific propagation contract;
/// native profile's controller ignores it after recording the failed check.
pub(super) async fn refresh_controller_version_check(
    state: &AppState,
    releases_url: &str,
) -> Result<(), String> {
    let result = async {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(100))
            .build()
            .map_err(|error| error.to_string())?;
        let response = client
            .get(releases_url)
            .header(
                reqwest::header::USER_AGENT,
                format!("slskR v{APP_VERSION} ({APP_VERSION})"),
            )
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?;
        let release = read_bounded_integration_json(response, "GitHub release").await?;
        let latest_tag = release
            .get("tag_name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "GitHub release omitted tag_name".to_owned())?
            .to_owned();
        let latest = normalize_controller_release_version(&latest_tag);
        let latest_lower = latest.to_ascii_lowercase();
        if latest_lower.contains("-dev-") || latest_lower.contains("-canary-") {
            return Ok(None);
        }
        let latest_url = release
            .get("html_url")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        Ok::<_, String>(Some((latest, latest_tag, latest_url)))
    }
    .await;
    match result {
        Ok(Some((latest, latest_tag, latest_url))) => {
            let is_update_available = is_newer_controller_release_available(APP_VERSION, &latest);
            {
                let mut version = state
                    .controller_version
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                version.latest = Some(latest.clone());
                version.latest_tag = Some(latest_tag);
                version.latest_url = Some(latest_url);
                version.checked_at = Some(chrono::Utc::now().to_rfc3339());
                version.is_update_available = Some(is_update_available);
            }
            let message = if is_update_available {
                format!("A new version is available! {APP_VERSION} -> {latest}")
            } else {
                format!("Version {APP_VERSION} is up to date.")
            };
            record_daemon_log(state, logging::LogLevel::Info, "version", message).await;
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(error) => {
            {
                let mut version = state
                    .controller_version
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                version.checked_at = Some(chrono::Utc::now().to_rfc3339());
                version.is_update_available = None;
            }
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "version",
                format!("Failed to check version: {error}"),
            )
            .await;
            Err(error)
        }
    }
}
