//! Controller full persistence fixtures ownership.

use super::*;

pub(super) async fn seed_kindless_library_item(
    state: &Arc<crate::AppState>,
    title: &str,
) -> String {
    let mut library = state.library.write().await;
    let record = library
        .create(
            "Differential Artist".to_owned(),
            title.to_owned(),
            String::new(),
        )
        .expect("kindless library fixture capacity");
    format!("{}-missing-kind", record.id)
}

pub(super) async fn conversation_request(
    state: &Arc<crate::AppState>,
    method: &str,
    path: &str,
    body: &str,
) -> crate::routing::HttpResponse {
    crate::route_http_request(method, path, None, body, state.as_ref())
        .await
        .unwrap_or_else(|error| panic!("{method} {path}: {error}"))
}

pub(super) async fn conversation_connect(state: &Arc<crate::AppState>) {
    state.session.write().await.state = "connected";
}

pub(super) async fn conversation_add(
    state: &Arc<crate::AppState>,
    username: &str,
    body: &str,
) -> u64 {
    state
        .messages
        .write()
        .await
        .add(username.to_owned(), "inbound", body.to_owned())
        .id
}

pub(super) async fn conversation_runtime_state(
    env: MapEnv,
    username: Option<&str>,
) -> Arc<crate::AppState> {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("conversation runtime database");
    let (state, _receiver) = test_state_with_env_parts(
        env.with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    if let Some(username) = username {
        conversation_add(&state, username, "runtime").await;
    }
    db.close_for_test().await;
    state
}

pub(super) async fn download_request_seed(state: &Arc<crate::AppState>) -> String {
    let mut transfers = state.transfers.write().await;
    transfers
        .create(
            0,
            Some("download-peer".to_owned()),
            "Remote/Download.flac".to_owned(),
            None,
            Some(100),
        )
        .request_id
        .expect("download request id")
}

pub(super) async fn download_request_add_attempt(
    state: &Arc<crate::AppState>,
    request_id: &str,
) -> u64 {
    let mut transfers = state.transfers.write().await;
    let entry = transfers.create(
        0,
        Some("download-peer".to_owned()),
        "Remote/Download.flac".to_owned(),
        None,
        Some(100),
    );
    let id = entry.id;
    transfers
        .entries
        .iter_mut()
        .find(|entry| entry.id == id)
        .expect("download attempt")
        .request_id = Some(request_id.to_owned());
    id
}

pub(super) async fn download_runtime_state(env: MapEnv) -> Arc<crate::AppState> {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("download runtime database");
    let (state, _receiver) = test_state_with_env_parts(
        env.with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    download_request_seed(&state).await;
    db.close_for_test().await;
    state
}

pub(super) fn file_path_env(target: &str, remote_management: bool) -> MapEnv {
    MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with(
            "SLSKD_REMOTE_FILE_MANAGEMENT",
            if remote_management { "true" } else { "false" },
        )
}

pub(super) fn file_path_segment(value: &str) -> String {
    crate::STANDARD_NO_PAD.encode(value.as_bytes())
}

pub(super) fn file_storage_root(state: &Arc<crate::AppState>, storage: &str) -> PathBuf {
    if storage == "downloads" {
        crate::effective_downloads_dir(state)
    } else {
        crate::effective_incomplete_dir(state)
    }
}

pub(super) fn replace_file_storage_root(
    state: &Arc<crate::AppState>,
    storage: &str,
    path: PathBuf,
) {
    if storage == "downloads" {
        *state
            .downloads_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = path;
    } else {
        *state
            .incomplete_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = path;
    }
}

pub(super) async fn file_runtime_state(storage: &str) -> Arc<crate::AppState> {
    let (state, _receiver) = test_state_with_env(file_path_env("slskdn", true));
    let root = file_storage_root(&state, storage);
    let bad_root = root.with_extension("not-a-directory");
    fs::write(&bad_root, b"not a directory").expect("create file storage failure fixture");
    replace_file_storage_root(&state, storage, bad_root);
    state
}
