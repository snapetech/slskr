use super::*;

pub(crate) async fn enrich_completed_audio_metadata(
    state: &AppState,
    transfer: TransferEntry,
) -> TransferEntry {
    if transfer.direction != 0 || !is_successful_transfer_status(&transfer.status) {
        return transfer;
    }
    let Some(local_path) = transfer.local_path.as_deref() else {
        return transfer;
    };
    let root = effective_downloads_dir(state);
    let Ok(path) = ensure_scoped_download_path(&root, local_path) else {
        return transfer;
    };

    // Matches native profile's completed-download HashDb pipeline: hash the first
    // 32 KiB of supported audio files, store the real byte hash under the
    // shared FLAC key, and expose each stage through the metadata activity
    // endpoint. Unsupported and undersized files are recorded as skipped,
    // not silently presented as an empty pipeline.
    let filename = transfer.filename.clone();
    let display_filename = Path::new(&filename)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(filename.as_str())
        .to_owned();
    let hash_stage = {
        let discovery = state.content_discovery.read().await;
        discovery.begin_metadata_stage(&display_filename, "hash")
    };
    if !is_audio_hash_candidate(&filename) {
        let discovery = state.content_discovery.read().await;
        discovery.finish_metadata_stage(
            hash_stage,
            "skipped",
            Some("Not a supported audio file".to_owned()),
        );
        return transfer;
    }
    let Ok(hash_file) = open_download_file_for_read(&root, &path) else {
        let discovery = state.content_discovery.read().await;
        discovery.finish_metadata_stage(
            hash_stage,
            "failed",
            Some("Completed file could not be opened".to_owned()),
        );
        return transfer;
    };
    let file_size = hash_file.metadata().map(|metadata| metadata.len()).ok();
    let Some(file_size) = file_size else {
        let discovery = state.content_discovery.read().await;
        discovery.finish_metadata_stage(
            hash_stage,
            "failed",
            Some("Completed file metadata could not be read".to_owned()),
        );
        return transfer;
    };
    if file_size < METADATA_HASH_CHUNK_SIZE as u64 {
        let discovery = state.content_discovery.read().await;
        discovery.finish_metadata_stage(
            hash_stage,
            "skipped",
            Some("File is too small for hashing".to_owned()),
        );
        return transfer;
    }
    let byte_hash = tokio::task::spawn_blocking(move || read_file_prefix_hash(hash_file))
        .await
        .ok()
        .flatten();
    let Some(byte_hash) = byte_hash else {
        let discovery = state.content_discovery.read().await;
        discovery.finish_metadata_stage(
            hash_stage,
            "failed",
            Some("Hash computation failed".to_owned()),
        );
        return transfer;
    };
    let flac_key = content_discovery::generate_flac_key(&filename, file_size);
    let persistence_turn = hash_db_persistence_turn().await;
    let (merge_result, previous_entries, previous_latest_seq, mutated_entries, mutated_latest_seq) = {
        let mut discovery = state.content_discovery.write().await;
        let previous_entries = discovery.hash_entries().to_vec();
        let previous_latest_seq = discovery.latest_seq();
        let merge_result = discovery.merge_hash_entries(vec![content_discovery::HashDbEntry {
            flac_key,
            byte_hash,
            size: file_size,
            ..content_discovery::HashDbEntry::default()
        }]);
        let mutated_entries = discovery.hash_entries().to_vec();
        let mutated_latest_seq = discovery.latest_seq();
        (
            merge_result,
            previous_entries,
            previous_latest_seq,
            mutated_entries,
            mutated_latest_seq,
        )
    };
    let merge_error = match merge_result {
        Ok(_) => match persist_current_hash_db_snapshot(state, &persistence_turn).await {
            Ok(()) => None,
            Err(error) => {
                rollback_hash_db_entries_if_unchanged(
                    state,
                    previous_entries,
                    previous_latest_seq,
                    &mutated_entries,
                    mutated_latest_seq,
                )
                .await;
                Some(error)
            }
        },
        Err(error) => Some(error),
    };
    drop(persistence_turn);
    let discovery = state.content_discovery.read().await;
    if let Some(error) = merge_error {
        discovery.finish_metadata_stage(hash_stage, "failed", Some(error));
        return transfer;
    }
    discovery.finish_metadata_stage(hash_stage, "complete", Some("Hash stored".to_owned()));
    let chromaprint_stage = discovery.begin_metadata_stage(&display_filename, "chromaprint");
    discovery.finish_metadata_stage(
        chromaprint_stage,
        "skipped",
        Some("Chromaprint is not configured".to_owned()),
    );
    drop(discovery);

    let Ok(file) = open_download_file_for_read(&root, &path) else {
        return transfer;
    };
    let metadata =
        tokio::task::spawn_blocking(move || read_audio_technical_metadata(file, &filename))
            .await
            .ok()
            .flatten();
    let Some(metadata) = metadata else {
        return transfer;
    };
    let mut transfers = state.transfers.write().await;
    transfers
        .update_audio_metadata(transfer.id, metadata)
        .unwrap_or(transfer)
}

const METADATA_HASH_CHUNK_SIZE: usize = 32 * 1024;

pub(super) fn is_audio_hash_candidate(filename: &str) -> bool {
    matches!(
        Path::new(filename)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "aac" | "flac" | "m4a" | "mp3" | "ogg" | "opus" | "wav" | "wave"
    )
}

pub(super) fn read_file_prefix_hash(mut file: fs::File) -> Option<String> {
    use sha2::Digest as _;
    use std::io::Read;

    let mut prefix = vec![0_u8; METADATA_HASH_CHUNK_SIZE];
    let bytes_read = file.read(&mut prefix).ok()?;
    (bytes_read > 0).then(|| hex::encode(Sha256::digest(&prefix[..bytes_read])))
}

pub(super) fn read_audio_technical_metadata(
    mut file: fs::File,
    filename: &str,
) -> Option<AudioTechnicalMetadata> {
    use std::io::Read;

    const MAX_AUDIO_HEADER_BYTES: u64 = 1024 * 1024;
    let file_size = file.metadata().ok()?.len();
    let mut header = Vec::new();
    file.by_ref()
        .take(MAX_AUDIO_HEADER_BYTES)
        .read_to_end(&mut header)
        .ok()?;
    let extension = Path::new(filename)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "flac" => flac_technical_metadata(&header, file_size),
        "mp3" => mp3_technical_metadata(&header, file_size),
        "wav" | "wave" => wav_technical_metadata(&header),
        _ => None,
    }
}

pub(super) fn flac_technical_metadata(
    header: &[u8],
    file_size: u64,
) -> Option<AudioTechnicalMetadata> {
    if header.get(..4)? != b"fLaC" {
        return None;
    }
    let block_header = header.get(4..8)?;
    if block_header[0] & 0x7f != 0 {
        return None;
    }
    let length = usize::from(block_header[1]) << 16
        | usize::from(block_header[2]) << 8
        | usize::from(block_header[3]);
    if length < 34 {
        return None;
    }
    let stream_info = header.get(8..8 + length)?;
    let packed = u64::from_be_bytes(stream_info.get(10..18)?.try_into().ok()?);
    let sample_rate = u32::try_from((packed >> 44) & 0x000f_ffff).ok()?;
    let bit_depth = u32::try_from(((packed >> 36) & 0x1f) + 1).ok()?;
    let total_samples = packed & 0x0000_000f_ffff_ffff;
    if sample_rate == 0 || total_samples == 0 {
        return None;
    }
    let length_seconds = u32::try_from(total_samples / u64::from(sample_rate)).ok()?;
    let bit_rate = (length_seconds > 0)
        .then(|| file_size.saturating_mul(8) / u64::from(length_seconds) / 1000)
        .and_then(|value| u32::try_from(value).ok());
    Some(AudioTechnicalMetadata {
        bit_rate,
        sample_rate: Some(sample_rate),
        bit_depth: Some(bit_depth),
        length_seconds: Some(length_seconds),
    })
}

pub(crate) fn mp3_technical_metadata(
    header: &[u8],
    file_size: u64,
) -> Option<AudioTechnicalMetadata> {
    const MPEG1_LAYER3: [u32; 16] = [
        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
    ];
    const MPEG2_LAYER3: [u32; 16] = [
        0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
    ];
    let frame = header.windows(4).find(|frame| {
        frame[0] == 0xff
            && frame[1] & 0xe0 == 0xe0
            && (frame[1] >> 1) & 0x03 == 0x01
            && frame[2] >> 4 != 0
            && frame[2] >> 4 != 0x0f
            && (frame[2] >> 2) & 0x03 != 0x03
    })?;
    let version = (frame[1] >> 3) & 0x03;
    if version == 1 {
        return None;
    }
    let bitrate_index = usize::from(frame[2] >> 4);
    let bit_rate = if version == 3 {
        MPEG1_LAYER3[bitrate_index]
    } else {
        MPEG2_LAYER3[bitrate_index]
    };
    let sample_index = usize::from((frame[2] >> 2) & 0x03);
    let base_sample_rate = [44_100_u32, 48_000, 32_000][sample_index];
    let sample_rate = match version {
        3 => base_sample_rate,
        2 => base_sample_rate / 2,
        0 => base_sample_rate / 4,
        _ => return None,
    };
    if bit_rate == 0 || sample_rate == 0 {
        return None;
    }
    let length_seconds = u32::try_from(file_size.saturating_mul(8) / (u64::from(bit_rate) * 1000))
        .ok()
        .filter(|value| *value > 0);
    Some(AudioTechnicalMetadata {
        bit_rate: Some(bit_rate),
        sample_rate: Some(sample_rate),
        bit_depth: None,
        length_seconds,
    })
}

pub(crate) fn wav_technical_metadata(header: &[u8]) -> Option<AudioTechnicalMetadata> {
    if header.get(..4)? != b"RIFF" || header.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut offset = 12_usize;
    let mut sample_rate = None;
    let mut bit_depth = None;
    let mut byte_rate = None;
    let mut data_length = None;
    while offset.saturating_add(8) <= header.len() {
        let kind = header.get(offset..offset + 4)?;
        let length = u32::from_le_bytes(header.get(offset + 4..offset + 8)?.try_into().ok()?);
        let length_usize = usize::try_from(length).ok()?;
        if kind == b"data" {
            data_length = Some(length);
        } else if kind == b"fmt " {
            let payload = header.get(offset + 8..offset + 8 + length_usize)?;
            if payload.len() < 16 {
                return None;
            }
            sample_rate = Some(u32::from_le_bytes(payload.get(4..8)?.try_into().ok()?));
            byte_rate = Some(u32::from_le_bytes(payload.get(8..12)?.try_into().ok()?));
            bit_depth = Some(u32::from(u16::from_le_bytes(
                payload.get(14..16)?.try_into().ok()?,
            )));
        }
        if sample_rate.is_some() && data_length.is_some() {
            break;
        }
        offset = offset.saturating_add(8 + length_usize + (length_usize % 2));
    }
    let byte_rate = byte_rate.filter(|value| *value > 0)?;
    let length_seconds = data_length
        .map(|length| length / byte_rate)
        .filter(|value| *value > 0);
    Some(AudioTechnicalMetadata {
        bit_rate: Some(byte_rate.saturating_mul(8) / 1000),
        sample_rate,
        bit_depth,
        length_seconds,
    })
}
