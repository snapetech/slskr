//! Controller full storage contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn files_api_lists_one_share_root_without_local_paths() {
    let (state, _receiver) = test_state();
    {
        let mut shares = state.shares.write().await;
        shares.roots.push(crate::ShareRoot {
            label: "Music".to_owned(),
            local_path: PathBuf::from("Music"),
            raw: "Music".to_owned(),
            directories: 0,
            files: 2,
            bytes: 142,
            extensions: vec![crate::ShareExtensionSummary {
                extension: "flac".to_owned(),
                files: 1,
                bytes: 42,
            }],
            statistics_ready: true,
        });
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Music/Other.mp3".to_owned(),
            size: 100,
            extension: "mp3".to_owned(),
            attributes: Vec::new(),
        });
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Music/Test.flac".to_owned(),
            size: 42,
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        });
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Library/Known/Release.jpg".to_owned(),
            size: 123,
            extension: "jpg".to_owned(),
            attributes: Vec::new(),
        });
        shares.local_paths.insert(
            "Music/Test.flac".to_owned(),
            PathBuf::from("/tmp/private/Test.flac"),
        );
    }

    let response = crate::route_http_request(
        "GET",
        "/api/v0/files/Music?extension=flac",
        None,
        "",
        &state,
    )
    .await
    .expect("files response");

    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"label\":\"Music\""));
    assert!(response.body.contains("\"path\":\"Test.flac\""));
    assert!(response
        .body
        .contains("\"virtual_path\":\"Music/Test.flac\""));
    assert!(response.body.contains("\"filtered_count\":1"));
    assert!(!response.body.contains("/tmp/private"));
    assert!(!response.body.contains("Other.mp3"));

    let missing = crate::route_http_request("GET", "/api/v0/files/Missing", None, "", &state)
        .await
        .expect("missing files response");
    assert_eq!(missing.status, "404 Not Found");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn files_api_projects_folder_views_and_directory_summaries() {
    let (state, _receiver) = test_state();
    {
        let mut shares = state.shares.write().await;
        shares.roots.push(crate::ShareRoot {
            label: "Music".to_owned(),
            local_path: PathBuf::from("Music"),
            raw: "Music".to_owned(),
            directories: 2,
            files: 4,
            bytes: 410,
            extensions: vec![
                crate::ShareExtensionSummary {
                    extension: "flac".to_owned(),
                    files: 3,
                    bytes: 310,
                },
                crate::ShareExtensionSummary {
                    extension: "mp3".to_owned(),
                    files: 1,
                    bytes: 100,
                },
            ],
            statistics_ready: true,
        });
        for (filename, size, extension) in [
            ("Music/Artist/Album/Track.flac", 100, "flac"),
            ("Music/Artist/Album/Second.flac", 110, "flac"),
            ("Music/Artist/Live/Bootleg.flac", 100, "flac"),
            ("Music/Artist/Loose.mp3", 100, "mp3"),
        ] {
            shares.entries.push(FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: filename.to_owned(),
                size,
                extension: extension.to_owned(),
                attributes: Vec::new(),
            });
        }
    }

    let root = crate::route_http_request("GET", "/api/v0/files/Music", None, "", &state)
        .await
        .expect("root files response");
    assert_eq!(root.status, "200 OK");
    let root_json = serde_json::from_str::<serde_json::Value>(&root.body).unwrap();
    assert_eq!(root_json["count"], 4);
    assert_eq!(root_json["filtered_count"], 4);
    assert_eq!(root_json["directory_count"], 1);
    assert_eq!(root_json["directories"][0]["path"], "Artist");
    assert_eq!(root_json["directories"][0]["file_count"], 4);
    assert_eq!(root_json["entries"].as_array().unwrap().len(), 4);

    let folder = crate::route_http_request(
        "GET",
        "/api/v0/files/Music?folder=Artist&extension=mp3",
        None,
        "",
        &state,
    )
    .await
    .expect("folder files response");
    assert_eq!(folder.status, "200 OK");
    let folder_json = serde_json::from_str::<serde_json::Value>(&folder.body).unwrap();
    assert_eq!(folder_json["folder"], "Artist");
    assert_eq!(folder_json["recursive"], false);
    assert_eq!(folder_json["filtered_count"], 1);
    assert_eq!(folder_json["directory_count"], 0);
    assert_eq!(folder_json["entries"][0]["path"], "Loose.mp3");
    assert_eq!(
        folder_json["entries"][0]["virtual_path"],
        "Music/Artist/Loose.mp3"
    );

    let nested = crate::route_http_request(
        "GET",
        "/api/v0/files/Music?folder=Artist&recursive=true&extension=flac&q=album",
        None,
        "",
        &state,
    )
    .await
    .expect("recursive folder files response");
    assert_eq!(nested.status, "200 OK");
    let nested_json = serde_json::from_str::<serde_json::Value>(&nested.body).unwrap();
    assert_eq!(nested_json["recursive"], true);
    assert_eq!(nested_json["filtered_count"], 2);
    assert_eq!(nested_json["directory_count"], 1);
    assert_eq!(nested_json["directories"][0]["path"], "Album");
    assert_eq!(nested_json["entries"][0]["path"], "Album/Track.flac");
    assert!(!nested.body.contains("Bootleg.flac"));
    assert!(!nested.body.contains("/tmp/private"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn file_storage_errors_redact_internal_details_and_preserve_client_errors() {
    let internal = crate::file_storage_error_response(
        "storage directory read failed: permission denied: /srv/private/downloads",
    );
    assert_eq!(internal.status, "503 Service Unavailable");
    assert_eq!(internal.body, "{\"error\":\"file storage unavailable\"}");
    assert!(!internal.body.contains("permission denied"));
    assert!(!internal.body.contains("/srv/private"));

    let client = crate::file_storage_error_response(
        "path must be relative and stay within the storage root",
    );
    assert_eq!(client.status, "400 Bad Request");
    assert!(client.body.contains("path must be relative"));

    let oversized = crate::file_storage_error_response(crate::STORAGE_DIRECTORY_ENTRY_LIMIT_ERROR);
    assert_eq!(oversized.status, "413 Payload Too Large");
    assert_eq!(
        oversized.body,
        "{\"error\":\"storage directory is too large to list\"}"
    );

    let too_deep = crate::file_storage_error_response(crate::STORAGE_DIRECTORY_DELETE_DEPTH_ERROR);
    assert_eq!(too_deep.status, "413 Payload Too Large");
    assert_eq!(
        too_deep.body,
        "{\"error\":\"storage directory tree is too deep to delete\"}"
    );

    let too_wide =
        crate::file_storage_error_response(crate::STORAGE_DIRECTORY_DELETE_ENTRY_LIMIT_ERROR);
    assert_eq!(too_wide.status, "413 Payload Too Large");
    assert_eq!(
        too_wide.body,
        "{\"error\":\"storage directory is too large to delete\"}"
    );
    let aggregate =
        crate::file_storage_error_response(crate::STORAGE_DIRECTORY_DELETE_TOTAL_LIMIT_ERROR);
    assert_eq!(aggregate.status, "413 Payload Too Large");
    assert_eq!(aggregate.body, too_wide.body);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn scoped_storage_listing_rejects_symlinked_parent() {
    use std::os::unix::fs::symlink;

    let (state, _receiver) = test_state();
    let root = state.config.state_dir.join("confined-listing");
    let outside = state.config.state_dir.join("outside-listing");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), b"secret").unwrap();
    symlink(&outside, root.join("linked")).unwrap();

    let error = crate::controller_storage_directory_json_unix(
        &root,
        std::path::Path::new("linked"),
        crate::StorageDirectoryListOptions {
            recursive: true,
            limit: 100,
        },
    )
    .expect_err("symlinked directory must be rejected");
    assert!(error.contains("confined open failed"));

    let listing = crate::controller_storage_directory_json(
        &root,
        None,
        crate::StorageDirectoryListOptions {
            recursive: true,
            limit: 100,
        },
    )
    .unwrap();
    assert!(!listing.contains("linked"));
    assert!(!listing.contains("secret.txt"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn destinations_bound_deduplicate_and_select_one_default() {
    let mut persisted = (0..crate::MAX_DESTINATIONS + 2)
        .map(|index| crate::persistence::DestinationRecord {
            id: format!("destination-{index}"),
            name: format!("Destination {index}"),
            path: format!("/downloads/{index}"),
            is_default: index < 2,
            created_at: 1,
            updated_at: 1,
        })
        .collect::<Vec<_>>();
    persisted.push(crate::persistence::DestinationRecord {
        id: "destination-0".to_owned(),
        name: "Duplicate".to_owned(),
        path: "/duplicate".to_owned(),
        is_default: true,
        created_at: 2,
        updated_at: 2,
    });
    let destinations = crate::DestinationStore::from_persisted(persisted);
    assert_eq!(destinations.records.len(), crate::MAX_DESTINATIONS);
    assert_eq!(
        destinations
            .records
            .iter()
            .filter(|record| record.is_default)
            .count(),
        1
    );
    assert_eq!(
        destinations
            .records
            .iter()
            .filter(|record| record.id == "destination-0")
            .count(),
        1
    );
    assert!(destinations.records[0].is_default);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn destination_validation_allows_children_but_rejects_escape_paths() {
    let root = std::env::temp_dir().join(format!(
        "slskr-destination-validation-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let child = root.join("nested");
    std::fs::create_dir_all(&child).unwrap();
    let destinations = crate::DestinationStore::from_config(&root, &[]);

    assert_eq!(
        destinations.normalize_explicit_path(&child.display().to_string()),
        Some(child.clone())
    );
    assert!(destinations
        .normalize_explicit_path(&root.join("../outside").display().to_string())
        .is_none());
    assert!(destinations
        .normalize_explicit_path("relative/nested")
        .is_none());

    let missing = root.join("new").join("nested");
    assert_eq!(
        destinations.normalize_explicit_path(&missing.display().to_string()),
        Some(missing)
    );

    #[cfg(unix)]
    {
        let outside = std::env::temp_dir().join(format!(
            "slskr-destination-validation-outside-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&outside).unwrap();
        let link = root.join("escape");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let escaped_missing = link.join("not-yet-created");
        assert!(destinations
            .normalize_explicit_path(&escaped_missing.display().to_string())
            .is_none());
        std::fs::remove_dir_all(outside).unwrap();
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn folder_contents_payload_filters_to_requested_virtual_folder() {
    let entries = crate::config::parse_share_entries(
        "Remote/Album/Song.flac=321;Remote/Other/Skip.flac=9;Loose.mp3=7",
    )
    .expect("entries");
    let payload =
        crate::build_folder_contents_payload(&entries, 321, "Remote/Album", Default::default())
            .expect("folder payload");
    let decoded = crate::decompress_zlib_payload(&payload).expect("decoded folder payload");
    let mut reader = crate::Reader::new(&decoded);

    assert_eq!(reader.read_u32_le().unwrap(), 321);
    assert_eq!(reader.read_string().unwrap(), "Remote/Album");
    assert_eq!(reader.read_u32_le().unwrap(), 1);
    assert_eq!(reader.read_string().unwrap(), "Remote/Album");
    assert_eq!(reader.read_u32_le().unwrap(), 1);
    assert_eq!(reader.read_u8().unwrap(), 1);
    assert_eq!(reader.read_string().unwrap(), "Song.flac");
    assert_eq!(reader.read_u64_le().unwrap(), 321);
    assert_eq!(reader.read_string().unwrap(), "flac");
    assert_eq!(reader.read_u32_le().unwrap(), 0);
    assert!(reader.is_empty());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn folder_browse_parsers_reject_excessive_wire_records() {
    let file_count = crate::MAX_BROWSE_WIRE_FILES_PER_RESPONSE + 1;

    let mut folder_contents = crate::Writer::new();
    folder_contents.write_u32_le(1);
    folder_contents.write_string("root").unwrap();
    folder_contents.write_u32_le(1);
    folder_contents.write_string("folder").unwrap();
    folder_contents.write_u32_le(u32::try_from(file_count).unwrap());
    for _ in 0..file_count {
        folder_contents.write_u8(0);
        folder_contents.write_string("").unwrap();
        folder_contents.write_u64_le(0);
        folder_contents.write_string("").unwrap();
        folder_contents.write_u32_le(0);
    }
    let folder_contents = crate::compress_zlib_payload(&folder_contents.into_inner()).unwrap();
    let error = crate::parse_folder_contents_response_payload(
        &folder_contents,
        "folder",
        crate::ProtocolTextEncoding::Utf8,
    )
    .unwrap_err();
    assert!(error.contains("folder files"), "{error}");

    let mut folder_files = crate::Writer::new();
    folder_files.write_u32_le(u32::try_from(file_count).unwrap());
    for _ in 0..file_count {
        folder_files.write_u8(0);
        folder_files.write_string("").unwrap();
        folder_files.write_u64_le(0);
        folder_files.write_string("").unwrap();
        folder_files.write_u32_le(0);
    }
    let folder_files = crate::compress_zlib_payload(&folder_files.into_inner()).unwrap();
    let error = crate::parse_folder_file_list_payload(
        &folder_files,
        "folder",
        crate::ProtocolTextEncoding::Utf8,
    )
    .unwrap_err();
    assert!(error.contains("folder files"), "{error}");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn path_segments_preserve_literal_plus_signs() {
    assert_eq!(crate::decoded_path_segment("a+b"), "a+b");
    assert_eq!(crate::decoded_path_segment("a%2Bb"), "a+b");
    assert_eq!(crate::decoded_path_segment("a%20b"), "a b");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn scoped_file_storage_delete_rejects_traversal_and_deletes_only_under_root() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-file-delete-test-{}-{unique}",
        std::process::id()
    ));
    let nested = root.join("artist");
    std::fs::create_dir_all(&nested).expect("create nested dir");
    let file = nested.join("track.flac");
    std::fs::write(&file, b"fixture").expect("write file");

    let encoded_file = crate::STANDARD.encode("artist/track.flac");
    assert_eq!(
        crate::delete_scoped_file_storage_path(&root, &encoded_file, false),
        Ok(true)
    );
    assert!(!file.exists());

    let traversal = crate::STANDARD.encode("../outside.flac");
    let error = crate::delete_scoped_file_storage_path(&root, &traversal, false)
        .expect_err("traversal must fail");
    assert!(error.contains("relative"));

    let album = root.join("album/disc");
    std::fs::create_dir_all(&album).expect("create recursive directory");
    std::fs::write(album.join("song.flac"), b"fixture").expect("write recursive file");
    let encoded_album = crate::STANDARD.encode("album");
    assert_eq!(
        crate::delete_scoped_file_storage_path(&root, &encoded_album, true),
        Ok(true)
    );
    assert!(!root.join("album").exists());

    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn scoped_storage_confined_delete_rejects_symlinked_parent() {
    use std::os::unix::fs::symlink;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-confined-delete-test-{}-{unique}",
        std::process::id()
    ));
    let outside = std::env::temp_dir().join(format!(
        "slskr-confined-delete-outside-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create root");
    std::fs::create_dir_all(&outside).expect("create outside");
    let victim = outside.join("victim.flac");
    std::fs::write(&victim, b"keep").expect("write victim");
    symlink(&outside, root.join("linked")).expect("symlinked parent");

    assert!(crate::controller_storage::delete_scoped_storage_path_unix(
        &root,
        &root.join("linked/victim.flac"),
        false
    )
    .is_err());
    assert_eq!(std::fs::read(&victim).unwrap(), b"keep");

    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn scoped_file_storage_delete_removes_dangling_symlink_without_following_it() {
    use std::os::unix::fs::symlink;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-dangling-delete-test-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create root");
    let link = root.join("dangling.flac");
    let missing_target = root.join("missing-target.flac");
    symlink(&missing_target, &link).expect("create dangling symlink");
    assert!(!link.exists(), "fixture must be dangling");
    assert!(link.symlink_metadata().is_ok(), "symlink must exist");

    let encoded = crate::STANDARD.encode("dangling.flac");
    assert_eq!(
        crate::delete_scoped_file_storage_path(&root, &encoded, false),
        Ok(true)
    );
    assert!(link.symlink_metadata().is_err());
    assert!(!missing_target.exists());

    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn scoped_storage_delete_bounds_directory_depth() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-deep-delete-test-{}-{unique}",
        std::process::id()
    ));
    let tree = root.join("tree");
    let mut directory = tree.clone();
    for depth in 0..=crate::controller_storage::SLSKD_STORAGE_MAX_DELETE_DEPTH {
        directory.push(format!("d{depth:02}"));
    }
    std::fs::create_dir_all(&directory).expect("create deep directory tree");

    let encoded = crate::STANDARD.encode("tree");
    assert_eq!(
        crate::delete_scoped_file_storage_path(&root, &encoded, true).unwrap_err(),
        crate::STORAGE_DIRECTORY_DELETE_DEPTH_ERROR
    );
    assert!(tree.exists());
    assert!(directory.exists());

    let _ = std::fs::remove_dir_all(root);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn scoped_storage_delete_bounds_directory_width() {
    let mut scanned = crate::SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES - 1;
    let mut total = 0;
    crate::controller_storage::reserve_storage_delete_entry(&mut scanned, &mut total)
        .expect("last delete scan slot");
    assert_eq!(scanned, crate::SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES);
    assert_eq!(
        crate::controller_storage::reserve_storage_delete_entry(&mut scanned, &mut total)
            .unwrap_err(),
        crate::STORAGE_DIRECTORY_DELETE_ENTRY_LIMIT_ERROR
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn scoped_storage_delete_bounds_aggregate_entries() {
    let mut scanned = 0;
    let mut total = crate::SLSKD_STORAGE_MAX_DELETE_TOTAL_ENTRIES - 1;
    crate::controller_storage::reserve_storage_delete_entry(&mut scanned, &mut total)
        .expect("last aggregate delete slot");
    assert_eq!(total, crate::SLSKD_STORAGE_MAX_DELETE_TOTAL_ENTRIES);
    assert_eq!(
        crate::controller_storage::reserve_storage_delete_entry(&mut scanned, &mut total)
            .unwrap_err(),
        crate::STORAGE_DIRECTORY_DELETE_TOTAL_LIMIT_ERROR
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn atomic_state_writer_replaces_existing_file() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-state-replace-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let destination = state_dir.join("state.json");

    crate::write_file_atomic(&destination, b"first").expect("first state write");
    crate::write_file_atomic(&destination, b"second").expect("replacement state write");

    assert_eq!(std::fs::read(&destination).expect("read state"), b"second");
    assert_eq!(
        std::fs::read_dir(&state_dir)
            .expect("list state dir")
            .count(),
        1
    );
    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn backfill_hash_matches_frozen_flac_header_policy() {
    use sha2::{Digest, Sha256};

    let mut header = vec![0_u8; 42];
    header[..4].copy_from_slice(b"fLaC");
    header[4] = 0;
    header[7] = 34;
    for (index, byte) in header[8..].iter_mut().enumerate() {
        *byte = u8::try_from(index).unwrap();
    }
    assert_eq!(
        crate::parse_flac_backfill_hash(&header).expect("valid FLAC header"),
        hex::encode(Sha256::digest(&header))
    );
    header[0] = b'X';
    assert_eq!(
        crate::parse_flac_backfill_hash(&header).unwrap_err(),
        "failed to parse FLAC header"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn backfill_daily_peer_limits_survive_restart() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-backfill-state-test-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&state_dir).unwrap();
    let now = crate::unix_timestamp();
    let mut state = crate::BackfillState::load(&state_dir).unwrap();
    state.record_peer_success("Peer-A", now);
    state.record_peer_success("peer-a", now);
    state.persist().unwrap();

    let mut reloaded = crate::BackfillState::load(&state_dir).unwrap();
    assert_eq!(reloaded.peer_count_today("PEER-A", now), 2);
    let _ = std::fs::remove_dir_all(state_dir);
}
