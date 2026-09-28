use super::*;

pub(crate) async fn transfer_capacity_available(
    state: &AppState,
    excluding_id: Option<u64>,
) -> bool {
    if state.config.transfer_max_active == 0 {
        return false;
    }
    let transfers = state.transfers.read().await;
    transfers.active_count_excluding(excluding_id) < state.config.transfer_max_active
}

pub(crate) async fn download_capacity_available(
    state: &AppState,
    excluding_id: Option<u64>,
) -> bool {
    if !transfer_capacity_available(state, excluding_id).await {
        return false;
    }
    let slots = state.transfer_download_settings.read().await.slots;
    let transfers = state.transfers.read().await;
    let active = transfers
        .entries
        .iter()
        .filter(|entry| entry.id != excluding_id.unwrap_or(u64::MAX))
        .filter(|entry| entry.direction == 0 && is_active_transfer_status(&entry.status))
        .count();
    active < usize::try_from(slots).unwrap_or(usize::MAX)
}

pub(crate) fn effective_transfer_group_from(
    settings: &crate::config::TransferGroupsSettings,
    users: &UserStore,
    username: &str,
) -> String {
    // Matches the oracle's real UserService.GetGroup: a blacklist decision is
    // only made for a username already present in the user cache. Unknown
    // usernames return the default group, even when the configured blacklist
    // would match them; once cached, the blacklist wins over every other
    // classification, including privileged status.
    if users
        .records
        .iter()
        .any(|record| record.username == username)
        && settings
            .blacklisted
            .members
            .iter()
            .any(|member| member.eq_ignore_ascii_case(username))
    {
        return "blacklisted".to_owned();
    }
    if users
        .records
        .iter()
        .find(|record| record.username == username)
        .is_some_and(|record| record.privileged)
    {
        return "privileged".to_owned();
    }
    if let Some((name, _)) = settings
        .user_defined
        .iter()
        .filter(|(_, group)| group.members.iter().any(|member| member == username))
        .min_by(|(left_name, left), (right_name, right)| {
            left.upload
                .priority
                .cmp(&right.upload.priority)
                .then_with(|| left_name.cmp(right_name))
        })
    {
        return name.clone();
    }
    if users
        .records
        .iter()
        .find(|record| record.username == username)
        .is_some_and(|record| {
            record
                .file_count
                .is_some_and(|count| count < settings.leechers.threshold_files)
                || record
                    .directory_count
                    .is_some_and(|count| count < settings.leechers.threshold_directories)
        })
    {
        "leechers".to_owned()
    } else {
        "default".to_owned()
    }
}

pub(crate) async fn effective_transfer_group(state: &AppState, username: &str) -> String {
    let settings = state.transfer_groups_settings.read().await;
    let users = state.users.read().await;
    effective_transfer_group_from(&settings, &users, username)
}

pub(crate) fn transfer_group_upload_settings<'a>(
    settings: &'a crate::config::TransferGroupsSettings,
    group: &str,
) -> Option<&'a crate::config::TransferGroupUploadSettings> {
    match group {
        "privileged" => None,
        "default" => Some(&settings.default.upload),
        "leechers" => Some(&settings.leechers.upload),
        name => settings.user_defined.get(name).map(|group| &group.upload),
    }
}

#[derive(Default)]
pub(super) struct UserUploadLimitStatistics {
    pub(super) queued_files: u64,
    pub(super) queued_bytes: u64,
    pub(super) weekly_failed_files: u64,
    pub(super) weekly_succeeded_files: u64,
    pub(super) weekly_succeeded_bytes: u64,
    pub(super) daily_failed_files: u64,
    pub(super) daily_succeeded_files: u64,
    pub(super) daily_succeeded_bytes: u64,
}

pub(super) fn user_upload_limit_statistics(
    transfers: &TransferQueue,
    username: &str,
    now: u64,
) -> UserUploadLimitStatistics {
    let mut stats = UserUploadLimitStatistics::default();
    let daily_cutoff = now.saturating_sub(24 * 60 * 60);
    let weekly_cutoff = now.saturating_sub(7 * 24 * 60 * 60);
    for entry in transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1 && entry.peer_username.as_deref() == Some(username))
    {
        if !is_terminal_transfer_status(&entry.status) {
            stats.queued_files = stats.queued_files.saturating_add(1);
            stats.queued_bytes = stats.queued_bytes.saturating_add(entry.size.unwrap_or(0));
        }
        let Some(started_at) = entry.started_at else {
            continue;
        };
        let failed = is_failed_transfer_status(&entry.status) || entry.status == "cancelled";
        let succeeded = is_successful_transfer_status(&entry.status);
        if started_at >= weekly_cutoff {
            if failed {
                stats.weekly_failed_files = stats.weekly_failed_files.saturating_add(1);
            } else if succeeded {
                stats.weekly_succeeded_files = stats.weekly_succeeded_files.saturating_add(1);
                stats.weekly_succeeded_bytes = stats
                    .weekly_succeeded_bytes
                    .saturating_add(entry.size.unwrap_or(0));
            }
        }
        if started_at >= daily_cutoff {
            if failed {
                stats.daily_failed_files = stats.daily_failed_files.saturating_add(1);
            } else if succeeded {
                stats.daily_succeeded_files = stats.daily_succeeded_files.saturating_add(1);
                stats.daily_succeeded_bytes = stats
                    .daily_succeeded_bytes
                    .saturating_add(entry.size.unwrap_or(0));
            }
        }
    }
    stats
}

pub(super) fn effective_limit_value(
    group: Option<&crate::config::TransferLimitSettings>,
    global: Option<&crate::config::TransferLimitSettings>,
    field: fn(&crate::config::TransferLimitSettings) -> Option<u32>,
) -> Option<u32> {
    group.and_then(field).or_else(|| global.and_then(field))
}

pub(super) fn transfer_window_limit_error(
    group: Option<&crate::config::TransferLimitSettings>,
    global: Option<&crate::config::TransferLimitSettings>,
    files: u64,
    bytes: u64,
    requested_size: u64,
    suffix: &str,
    megabytes_first: bool,
) -> Option<String> {
    let file_over = effective_limit_value(group, global, |limit| limit.files)
        .is_some_and(|limit| files.saturating_add(1) > u64::from(limit));
    let megabytes_over =
        effective_limit_value(group, global, |limit| limit.megabytes).is_some_and(|limit| {
            bytes.saturating_add(requested_size) > u64::from(limit).saturating_mul(1_000_000)
        });
    if !file_over && !megabytes_over {
        return None;
    }
    let unit = if (megabytes_first && megabytes_over) || (!megabytes_first && !file_over) {
        "megabytes"
    } else {
        "files"
    };
    Some(format!("Too many {unit}{suffix}"))
}

pub(crate) async fn inbound_upload_policy(
    state: &AppState,
    username: &str,
    filename: &str,
    requested_size: u64,
) -> Result<String, String> {
    if state
        .failed_upload_peer_cooldowns
        .write()
        .await
        .remaining(username, unix_timestamp())
        .is_some()
    {
        return Err("Recent transfer failed; retry later.".to_owned());
    }
    if !state.config.transfer_allow_inbound {
        return Err("inbound transfers are disabled".to_owned());
    }
    if !transfer_capacity_available(state, None).await {
        return Err("transfer limit reached".to_owned());
    }
    let group_name = effective_transfer_group(state, username).await;
    let privileged = group_name == "privileged";
    let groups = state.transfer_groups_settings.read().await;
    let upload = state.transfer_upload_settings.read().await;
    let group =
        transfer_group_upload_settings(&groups, &group_name).unwrap_or(&groups.default.upload);
    if !privileged
        && state.config.controller_profile == ControllerProfile::Native
        && !group.allowed_file_types.is_empty()
    {
        let extension = Path::new(filename)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!(".{extension}"))
            .unwrap_or_default();
        if !group
            .allowed_file_types
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(&extension))
        {
            return Err(format!("File type {extension} is not permitted."));
        }
    }
    let transfers = state.transfers.read().await;
    let stats = user_upload_limit_statistics(&transfers, username, unix_timestamp());
    if !privileged {
        if let Some(error) = transfer_window_limit_error(
            group.limits.queued.as_ref(),
            upload.limits.queued.as_ref(),
            stats.queued_files,
            stats.queued_bytes,
            requested_size,
            "",
            true,
        ) {
            return Err(error);
        }
        if effective_limit_value(
            group.limits.weekly.as_ref(),
            upload.limits.weekly.as_ref(),
            |limit| limit.failures,
        )
        .is_some_and(|limit| stats.weekly_failed_files >= u64::from(limit))
        {
            return Err("Too many failed transfers this week".to_owned());
        }
        if let Some(error) = transfer_window_limit_error(
            group.limits.weekly.as_ref(),
            upload.limits.weekly.as_ref(),
            stats.weekly_succeeded_files,
            stats.weekly_succeeded_bytes,
            requested_size,
            " this week",
            false,
        ) {
            return Err(error);
        }
        if effective_limit_value(
            group.limits.daily.as_ref(),
            upload.limits.daily.as_ref(),
            |limit| limit.failures,
        )
        .is_some_and(|limit| stats.daily_failed_files >= u64::from(limit))
        {
            return Err("Too many failed transfers today".to_owned());
        }
        if let Some(error) = transfer_window_limit_error(
            group.limits.daily.as_ref(),
            upload.limits.daily.as_ref(),
            stats.daily_succeeded_files,
            stats.daily_succeeded_bytes,
            requested_size,
            " today",
            false,
        ) {
            return Err(error);
        }
    }
    let active_uploads = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1 && is_active_transfer_status(&entry.status))
        .count();
    if active_uploads >= usize::try_from(upload.slots).unwrap_or(usize::MAX) {
        return Err("Queued".to_owned());
    }
    let users = state.users.read().await;
    let group_active = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1 && is_active_transfer_status(&entry.status))
        .filter(|entry| {
            entry.peer_username.as_deref().is_some_and(|peer| {
                effective_transfer_group_from(&groups, &users, peer) == group_name
            })
        })
        .count();
    let group_slots = if privileged {
        upload.slots
    } else {
        group.slots.min(upload.slots)
    };
    if group_active >= usize::try_from(group_slots).unwrap_or(usize::MAX) {
        return Err("Queued".to_owned());
    }
    Ok(group_name)
}
