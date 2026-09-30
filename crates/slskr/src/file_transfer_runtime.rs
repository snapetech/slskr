use super::transfer_queue::AudioTechnicalMetadata;
use super::*;
use std::io::Read;
#[path = "file_transfer_runtime_owners/audio_metadata.rs"]
mod audio_metadata;
pub(super) use self::audio_metadata::enrich_completed_audio_metadata;
#[path = "file_transfer_runtime_owners/completed_permissions.rs"]
mod completed_permissions;
pub(super) use self::completed_permissions::apply_completed_download_permissions;
#[path = "file_transfer_runtime_owners/download_connection.rs"]
mod download_connection;
pub(super) use self::download_connection::download_file_transfer_with_connection;
use self::download_connection::download_file_transfer_with_retry;
#[path = "file_transfer_runtime_owners/download_content_safety.rs"]
mod download_content_safety;
pub(super) use self::download_content_safety::enforce_completed_download_content_safety;
use self::download_content_safety::validate_configured_path_policy;
#[path = "file_transfer_runtime_owners/download_paths.rs"]
mod download_paths;
pub(super) use self::download_paths::{
    configured_download_destination_path, ensure_scoped_download_path, open_download_file,
    open_download_file_for_read, prepare_transfer_local_path,
    render_configured_completed_download_path, safe_download_path,
};
#[path = "file_transfer_runtime_owners/download_progress.rs"]
mod download_progress;
use self::download_progress::download_file_with_progress;
pub(super) use self::download_progress::{transfer_is_cancelled, update_transfer_progress};
#[path = "file_transfer_runtime_owners/inbound_transfer.rs"]
mod inbound_transfer;
pub(super) use self::inbound_transfer::handle_inbound_file_transfer;
#[path = "file_transfer_runtime_owners/indirect_transfer.rs"]
mod indirect_transfer;
use self::indirect_transfer::execute_indirect_file_transfer;
pub(super) use self::indirect_transfer::{fail_indirect_browse, fail_indirect_transfer};
#[path = "file_transfer_runtime_owners/peer_negotiation.rs"]
mod peer_negotiation;
pub(super) use self::peer_negotiation::{
    execute_accepted_file_transfer, probe_peer_capability, project_indirect_transfer_response,
    project_peer_transfer_response, schedule_queued_downloads,
};
#[path = "file_transfer_runtime_owners/transfer_policy.rs"]
mod transfer_policy;
pub(super) use self::transfer_policy::{
    download_capacity_available, effective_transfer_group, effective_transfer_group_from,
    inbound_upload_policy, transfer_capacity_available, transfer_group_upload_settings,
};
#[path = "file_transfer_runtime_owners/upload_streaming.rs"]
mod upload_streaming;
use self::upload_streaming::upload_file_transfer;
pub(super) use self::upload_streaming::upload_file_transfer_with_connection;

// Preserve parent-scoped bindings used by feature-selected contracts.
#[allow(unused_imports)]
pub(super) use self::audio_metadata::{mp3_technical_metadata, wav_technical_metadata};

// Preserve parent-scoped bindings used by feature-selected contracts.
#[allow(unused_imports)]
pub(super) use self::download_connection::{
    download_retry_delay, prepare_incomplete_download_file,
};

// Preserve parent-scoped bindings used by feature-selected contracts.
#[allow(unused_imports)]
pub(super) use self::download_paths::render_completed_download_path;

// Preserve parent-scoped bindings used by feature-selected contracts.
#[allow(unused_imports)]
pub(super) use self::download_progress::{
    effective_download_pacing_limit, write_download_chunk_if_active,
};

// Preserve parent-scoped bindings used by feature-selected contracts.
#[allow(unused_imports)]
pub(super) use self::upload_streaming::effective_upload_speed_limit;

// Preserve the original test/differential-only path helper binding.
#[cfg(any(test, feature = "bounded-differential"))]
#[allow(unused_imports)]
pub(super) use self::download_paths::download_root;
