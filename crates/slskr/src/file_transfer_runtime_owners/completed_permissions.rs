use super::*;

pub(crate) async fn apply_completed_download_permissions(
    state: &AppState,
    transfer: TransferEntry,
) -> TransferEntry {
    if transfer.direction != 0 || !is_successful_transfer_status(&transfer.status) {
        return transfer;
    }
    let mode = state
        .transfer_download_settings
        .read()
        .await
        .destination
        .permissions_mode
        .clone()
        .or_else(|| state.config.permissions_file_mode.clone());
    let (Some(mode), Some(local_path)) = (mode, transfer.local_path.as_deref()) else {
        return transfer;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let Ok(mode) = u32::from_str_radix(&mode, 8) else {
            return transfer;
        };
        let root = effective_downloads_dir(state);
        let Ok(path) = ensure_scoped_download_path(&root, local_path) else {
            return transfer;
        };
        if let Err(error) = fs::set_permissions(&path, fs::Permissions::from_mode(mode)) {
            let failed = state.transfers.write().await.update_status(
                transfer.id,
                "failed",
                None,
                Some(format!("download destination permissions failed: {error}")),
            );
            return failed.unwrap_or(transfer);
        }
        if let Some(parent) = path.parent().filter(|parent| *parent != root) {
            let directory_mode =
                mode | ((mode & 0o400) >> 2) | ((mode & 0o040) >> 2) | ((mode & 0o004) >> 2);
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(directory_mode));
        }
    }
    transfer
}
