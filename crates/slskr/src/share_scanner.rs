use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use super::{
    compile_controller_regexes, share_cache_path, unix_timestamp, write_share_cache_if_enabled,
    AppConfig, ControllerRegex, FileAttribute, FileEntry, ShareDirectory, ShareExtensionSummary,
    ShareIndexSnapshot, ShareRoot, MAX_SHARE_SCAN_ENTRIES, SHARE_SCAN_CANCELLED_ERROR,
    SHARE_SCAN_ENTRY_LIMIT_ERROR,
};
#[cfg(unix)]
use super::{MAX_SHARE_SCAN_PENDING_DIRECTORIES, SHARE_SCAN_PENDING_DIRECTORY_LIMIT_ERROR};

pub(super) fn reserve_share_scan_entry(scanned_entries: &mut usize) -> bool {
    if *scanned_entries >= MAX_SHARE_SCAN_ENTRIES {
        return false;
    }
    *scanned_entries += 1;
    true
}

pub(super) fn virtual_share_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => name.to_str().map(sanitize_share_component),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn sanitize_share_component(value: &str) -> String {
    value.replace(['\\', '/'], "_").trim().to_owned()
}

fn is_hidden_share_path(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .map(|relative| {
            relative.components().any(|component| {
                matches!(component, std::path::Component::Normal(name) if name.to_string_lossy().starts_with('.'))
            })
        })
        .unwrap_or(false)
}

fn json_safe_share_label(label: &str) -> String {
    label.chars().take(80).collect()
}

pub(super) fn extension_for(filename: &str) -> String {
    filename
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default()
}

fn probe_media_attributes(path: &Path, extension: &str, file_size: u64) -> Vec<FileAttribute> {
    let Ok(mut file) = fs::File::open(path) else {
        return Vec::new();
    };
    probe_media_attributes_from_file(&mut file, extension, file_size)
}

fn probe_media_attributes_from_file(
    file: &mut fs::File,
    extension: &str,
    file_size: u64,
) -> Vec<FileAttribute> {
    if file.seek(SeekFrom::Start(0)).is_err() {
        return Vec::new();
    }
    let mut bytes = Vec::new();
    if file.take(256 * 1024).read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    match extension.to_ascii_lowercase().as_str() {
        "wav" => probe_wav_attributes(&bytes),
        "flac" => probe_flac_attributes(&bytes, file_size),
        "mp3" => probe_mp3_attributes(&bytes, file_size),
        _ => Vec::new(),
    }
}

fn media_attributes(
    bitrate: Option<u32>,
    duration: Option<u32>,
    variable_bitrate: Option<bool>,
    sample_rate: Option<u32>,
    bit_depth: Option<u32>,
) -> Vec<FileAttribute> {
    [
        (0, bitrate),
        (1, duration),
        (2, variable_bitrate.map(u32::from)),
        (4, sample_rate),
        (5, bit_depth),
    ]
    .into_iter()
    .filter_map(|(code, value)| value.map(|value| FileAttribute { code, value }))
    .collect()
}

fn saturating_media_u32(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn probe_wav_attributes(bytes: &[u8]) -> Vec<FileAttribute> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Vec::new();
    }
    let mut offset = 12_usize;
    let mut sample_rate = None;
    let mut byte_rate = None;
    let mut bit_depth = None;
    let mut data_bytes = None;
    while offset.saturating_add(8) <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let data_start = offset + 8;
        let data_end = data_start.saturating_add(size).min(bytes.len());
        if id == b"fmt " && data_end.saturating_sub(data_start) >= 16 {
            sample_rate = Some(u32::from_le_bytes(
                bytes[data_start + 4..data_start + 8].try_into().unwrap(),
            ));
            byte_rate = Some(u32::from_le_bytes(
                bytes[data_start + 8..data_start + 12].try_into().unwrap(),
            ));
            bit_depth = Some(u16::from_le_bytes(
                bytes[data_start + 14..data_start + 16].try_into().unwrap(),
            ) as u32);
        } else if id == b"data" {
            data_bytes = Some(size as u64);
        }
        offset = data_start.saturating_add(size).saturating_add(size % 2);
    }
    let duration = byte_rate
        .filter(|rate| *rate > 0)
        .zip(data_bytes)
        .map(|(rate, size)| saturating_media_u32((size / u64::from(rate)).max(1)));
    let bitrate = byte_rate.map(|rate| rate.saturating_mul(8) / 1_000);
    media_attributes(bitrate, duration, Some(false), sample_rate, bit_depth)
}

fn probe_flac_attributes(bytes: &[u8], file_size: u64) -> Vec<FileAttribute> {
    if bytes.len() < 42 || &bytes[..4] != b"fLaC" {
        return Vec::new();
    }
    let mut offset = 4_usize;
    while offset.saturating_add(4) <= bytes.len() {
        let kind = bytes[offset] & 0x7f;
        let last = bytes[offset] & 0x80 != 0;
        let length = ((bytes[offset + 1] as usize) << 16)
            | ((bytes[offset + 2] as usize) << 8)
            | bytes[offset + 3] as usize;
        let start = offset + 4;
        let end = start.saturating_add(length);
        if end > bytes.len() {
            return Vec::new();
        }
        if kind == 0 && length >= 34 {
            let packed = u64::from_be_bytes(bytes[start + 10..start + 18].try_into().unwrap());
            let sample_rate = ((packed >> 44) & 0x0f_ffff) as u32;
            let bit_depth = (((packed >> 36) & 0x1f) + 1) as u32;
            let total_samples = packed & 0x0f_ffff_ffff;
            let duration = (sample_rate > 0)
                .then(|| saturating_media_u32((total_samples / u64::from(sample_rate)).max(1)));
            let bitrate = duration.filter(|seconds| *seconds > 0).map(|seconds| {
                saturating_media_u32(file_size.saturating_mul(8) / u64::from(seconds) / 1_000)
            });
            return media_attributes(
                bitrate,
                duration,
                Some(false),
                Some(sample_rate),
                Some(bit_depth),
            );
        }
        if last {
            break;
        }
        offset = end;
    }
    Vec::new()
}

fn probe_mp3_attributes(bytes: &[u8], file_size: u64) -> Vec<FileAttribute> {
    const MPEG1_LAYER3_BITRATES: [u16; 16] = [
        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
    ];
    const MPEG2_LAYER3_BITRATES: [u16; 16] = [
        0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
    ];
    for frame in bytes.windows(4) {
        if frame[0] != 0xff || frame[1] & 0xe0 != 0xe0 {
            continue;
        }
        let version = (frame[1] >> 3) & 0x03;
        let layer = (frame[1] >> 1) & 0x03;
        if version == 1 || layer != 1 {
            continue;
        }
        let bitrate_index = (frame[2] >> 4) as usize;
        let sample_index = ((frame[2] >> 2) & 0x03) as usize;
        if sample_index == 3 {
            continue;
        }
        let bitrate = if version == 3 {
            MPEG1_LAYER3_BITRATES[bitrate_index]
        } else {
            MPEG2_LAYER3_BITRATES[bitrate_index]
        } as u32;
        if bitrate == 0 {
            continue;
        }
        let base_sample_rate = [44_100_u32, 48_000, 32_000][sample_index];
        let sample_rate = match version {
            3 => base_sample_rate,
            2 => base_sample_rate / 2,
            _ => base_sample_rate / 4,
        };
        let duration =
            saturating_media_u32((file_size.saturating_mul(8) / u64::from(bitrate) / 1_000).max(1));
        return media_attributes(
            Some(bitrate),
            Some(duration),
            Some(false),
            Some(sample_rate),
            None,
        );
    }
    Vec::new()
}

#[cfg(test)]
mod media_attribute_tests {
    use super::*;

    #[test]
    fn oversized_flac_duration_and_mp3_duration_do_not_wrap() {
        let mut flac = vec![0; 42];
        flac[..4].copy_from_slice(b"fLaC");
        flac[4] = 0x80;
        flac[7] = 34;
        let streaminfo = (1_u64 << 44) | (15_u64 << 36) | ((1_u64 << 36) - 1);
        flac[18..26].copy_from_slice(&streaminfo.to_be_bytes());
        let flac_attributes = probe_flac_attributes(&flac, u64::MAX);
        assert_eq!(
            flac_attributes
                .iter()
                .find(|attribute| attribute.code == 1)
                .unwrap()
                .value,
            u32::MAX
        );

        let mp3_attributes = probe_mp3_attributes(&[0xff, 0xfb, 0x90, 0x00], u64::MAX);
        assert_eq!(
            mp3_attributes
                .iter()
                .find(|attribute| attribute.code == 1)
                .unwrap()
                .value,
            u32::MAX
        );
        assert_eq!(saturating_media_u32(u64::MAX), u32::MAX);
    }
}

#[derive(Clone, Debug)]
pub(super) struct ShareScan {
    pub(super) entries: Vec<FileEntry>,
    pub(super) local_paths: BTreeMap<String, PathBuf>,
    pub(super) roots: Vec<ShareRoot>,
    pub(super) errors: Vec<String>,
    pub(super) cancelled: bool,
}

#[derive(Clone, Debug)]
struct ShareScanOptions {
    follow_symlinks: bool,
    include_hidden: bool,
    max_files: usize,
    probe_media_attributes: bool,
    cancellation: Arc<AtomicBool>,
}

pub(super) struct ShareScanRequest<'a> {
    pub(super) directories: &'a [ShareDirectory],
    pub(super) follow_symlinks: bool,
    pub(super) include_hidden: bool,
    pub(super) max_files: usize,
    pub(super) probe_media_attributes: bool,
    pub(super) workers: usize,
    pub(super) filters: &'a [ControllerRegex],
    pub(super) cancellation: Arc<AtomicBool>,
}

pub(super) fn build_share_index(config: &AppConfig) -> ShareIndexSnapshot {
    build_share_index_with_cancellation(config, Arc::new(AtomicBool::new(false)))
        .expect("share scan without cancellation cannot fail")
}

pub(super) fn build_share_index_with_cancellation(
    config: &AppConfig,
    cancellation: Arc<AtomicBool>,
) -> Result<ShareIndexSnapshot, String> {
    let filters = compile_controller_regexes(
        &config.share_settings.filters,
        config.controller_case_sensitive_regex,
        config.controller_profile,
    )
    .expect("validated share filters must compile");
    let mut scan = scan_share_dirs_with_cancellation(ShareScanRequest {
        directories: &config.share_settings.directories,
        follow_symlinks: config.share_settings.follow_symlinks,
        include_hidden: config.share_settings.include_hidden,
        max_files: config.share_settings.max_files,
        probe_media_attributes: config.share_settings.probe_media_attributes,
        workers: config.share_settings.cache_workers,
        filters: &filters,
        cancellation,
    });
    if scan.cancelled {
        return Err(SHARE_SCAN_CANCELLED_ERROR.to_owned());
    }
    let fixture_files = config.share_settings.fixture_entries.len();
    let mut entries = config.share_settings.fixture_entries.clone();
    entries.append(&mut scan.entries);
    let cache_path = share_cache_path(&config.state_dir);
    let (cache_written_at, cache_error) = write_share_cache_if_enabled(
        config.share_settings.cache_tsv_enabled,
        &cache_path,
        &entries,
    );

    Ok(ShareIndexSnapshot {
        entries,
        local_paths: scan.local_paths,
        roots: scan.roots,
        fixture_files,
        scan_errors: scan.errors,
        cache_path,
        cache_enabled: config.share_settings.cache_tsv_enabled,
        cache_written_at,
        cache_error,
        updated_at: unix_timestamp(),
    })
}

pub(super) fn scan_share_dirs(
    directories: &[ShareDirectory],
    follow_symlinks: bool,
    include_hidden: bool,
    max_files: usize,
    probe_media_attributes: bool,
    workers: usize,
    filters: &[ControllerRegex],
) -> ShareScan {
    scan_share_dirs_with_cancellation(ShareScanRequest {
        directories,
        follow_symlinks,
        include_hidden,
        max_files,
        probe_media_attributes,
        workers,
        filters,
        cancellation: Arc::new(AtomicBool::new(false)),
    })
}

pub(super) fn scan_share_dirs_with_cancellation(request: ShareScanRequest<'_>) -> ShareScan {
    let ShareScanRequest {
        directories,
        follow_symlinks,
        include_hidden,
        max_files,
        probe_media_attributes,
        workers,
        filters,
        cancellation,
    } = request;
    let options = ShareScanOptions {
        follow_symlinks,
        include_hidden,
        max_files,
        probe_media_attributes,
        cancellation,
    };
    let exclusions = directories
        .iter()
        .filter(|directory| directory.is_excluded)
        .map(|directory| directory.local_path.clone())
        .collect::<Vec<_>>();

    let mut ordered_directories = directories.to_vec();
    ordered_directories.sort_by(|left, right| {
        right
            .local_path
            .as_os_str()
            .len()
            .cmp(&left.local_path.as_os_str().len())
    });
    let active = ordered_directories
        .iter()
        .enumerate()
        .filter(|(_, directory)| !directory.is_excluded)
        .map(|(index, directory)| (index, directory.clone()))
        .collect::<Vec<_>>();
    let worker_count = workers.max(1).min(active.len().max(1));
    let mut groups = (0..worker_count).map(|_| Vec::new()).collect::<Vec<_>>();
    for (position, item) in active.into_iter().enumerate() {
        groups[position % worker_count].push(item);
    }
    let scanned = std::thread::scope(|scope| {
        groups
            .into_iter()
            .map(|group| {
                let exclusions = &exclusions;
                let options = options.clone();
                scope.spawn(move || {
                    group
                        .into_iter()
                        .map(|(index, directory)| {
                            let scan = scan_single_share_directory(
                                &directory,
                                exclusions,
                                filters,
                                options.clone(),
                            );
                            (index, scan)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .flat_map(|worker| worker.join().unwrap_or_default())
            .collect::<BTreeMap<_, _>>()
    });

    let mut entries = Vec::new();
    let mut local_paths = BTreeMap::new();
    let mut root_summaries = Vec::new();
    let mut errors = Vec::new();
    for (index, directory) in ordered_directories.iter().enumerate() {
        let mut scan = scanned.get(&index).cloned().unwrap_or(ShareScan {
            entries: Vec::new(),
            local_paths: BTreeMap::new(),
            roots: Vec::new(),
            errors: Vec::new(),
            cancelled: false,
        });
        if scan.cancelled {
            return ShareScan {
                entries,
                local_paths,
                roots: root_summaries,
                errors,
                cancelled: true,
            };
        }
        let remaining = max_files.saturating_sub(entries.len());
        scan.entries.truncate(remaining);
        let accepted = scan
            .entries
            .iter()
            .map(|entry| entry.filename.as_str())
            .collect::<HashSet<_>>();
        scan.local_paths
            .retain(|filename, _| accepted.contains(filename.as_str()));
        root_summaries.push(share_root_summary(directory, &scan.entries));
        entries.append(&mut scan.entries);
        local_paths.append(&mut scan.local_paths);
        errors.append(&mut scan.errors);
        if entries.len() >= max_files {
            break;
        }
    }

    let cancelled = options.cancellation.load(Ordering::Relaxed);
    ShareScan {
        entries,
        local_paths,
        roots: root_summaries,
        errors,
        cancelled,
    }
}

fn scan_single_share_directory(
    directory: &ShareDirectory,
    exclusions: &[PathBuf],
    filters: &[ControllerRegex],
    options: ShareScanOptions,
) -> ShareScan {
    let mut entries = Vec::new();
    let mut local_paths = BTreeMap::new();
    let mut errors = Vec::new();
    let mut scanned_entries = 0;
    let mut output = ShareScanOutput {
        entries: &mut entries,
        local_paths: &mut local_paths,
        errors: &mut errors,
        scanned_entries: &mut scanned_entries,
    };
    let cancellation = Arc::clone(&options.cancellation);
    scan_share_root(
        &directory.local_path,
        &directory.alias,
        exclusions,
        filters,
        options,
        &mut output,
    );
    ShareScan {
        entries,
        local_paths,
        roots: Vec::new(),
        errors,
        cancelled: cancellation.load(Ordering::Relaxed),
    }
}

fn share_root_summary(directory: &ShareDirectory, entries: &[FileEntry]) -> ShareRoot {
    ShareRoot {
        label: directory.alias.clone(),
        local_path: directory.local_path.clone(),
        raw: directory.raw.clone(),
        directories: entries
            .iter()
            .filter_map(|entry| entry.filename.split_once('/').map(|(_, path)| path))
            .filter_map(|path| path.rsplit_once('/').map(|(directory, _)| directory))
            .flat_map(|directory| {
                let parts = directory.split('/').collect::<Vec<_>>();
                (1..=parts.len())
                    .map(move |count| parts[..count].join("/"))
                    .collect::<Vec<_>>()
            })
            .collect::<HashSet<_>>()
            .len(),
        files: entries.len(),
        bytes: entries.iter().map(|entry| entry.size).sum(),
        extensions: summarize_extensions(entries),
        statistics_ready: true,
    }
}

pub(super) fn summarize_extensions(entries: &[FileEntry]) -> Vec<ShareExtensionSummary> {
    let mut summaries = BTreeMap::<String, (usize, u64)>::new();
    for entry in entries {
        let extension = if entry.extension.is_empty() {
            "(none)".to_owned()
        } else {
            entry.extension.to_ascii_lowercase()
        };
        let summary = summaries.entry(extension).or_default();
        summary.0 += 1;
        summary.1 += entry.size;
    }
    summaries
        .into_iter()
        .map(|(extension, (files, bytes))| ShareExtensionSummary {
            extension,
            files,
            bytes,
        })
        .collect()
}

struct ShareScanOutput<'a> {
    entries: &'a mut Vec<FileEntry>,
    local_paths: &'a mut BTreeMap<String, PathBuf>,
    errors: &'a mut Vec<String>,
    scanned_entries: &'a mut usize,
}

fn scan_share_root(
    root: &Path,
    label: &str,
    exclusions: &[PathBuf],
    filters: &[ControllerRegex],
    options: ShareScanOptions,
    output: &mut ShareScanOutput<'_>,
) {
    if options.cancellation.load(Ordering::Relaxed) {
        return;
    }
    if output.entries.len() >= options.max_files {
        return;
    }

    #[cfg(unix)]
    if !options.follow_symlinks {
        scan_share_root_unix(root, label, exclusions, filters, options, output);
        return;
    }

    let metadata = if options.follow_symlinks {
        fs::metadata(root)
    } else {
        fs::symlink_metadata(root)
    };
    let Ok(metadata) = metadata else {
        output.errors.push(format!(
            "{}: metadata unavailable",
            json_safe_share_label(label)
        ));
        return;
    };
    if !metadata.is_dir() {
        output
            .errors
            .push(format!("{}: not a directory", json_safe_share_label(label)));
        return;
    }
    let canonical_root = if options.follow_symlinks {
        match root.canonicalize() {
            Ok(path) => Some(path),
            Err(_) => {
                output.errors.push(format!(
                    "{}: canonical root unavailable",
                    json_safe_share_label(label)
                ));
                return;
            }
        }
    } else {
        None
    };

    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        if options.cancellation.load(Ordering::Relaxed) {
            return;
        }
        if output.entries.len() >= options.max_files {
            output
                .errors
                .push("share scan stopped at SLSKR_SHARE_SCAN_MAX_FILES".to_owned());
            return;
        }

        let read_dir = match fs::read_dir(&directory) {
            Ok(read_dir) => read_dir,
            Err(_) => {
                output.errors.push(format!(
                    "{}: directory unreadable",
                    json_safe_share_label(label)
                ));
                continue;
            }
        };

        for child in read_dir {
            if options.cancellation.load(Ordering::Relaxed) {
                return;
            }
            if output.entries.len() >= options.max_files {
                output
                    .errors
                    .push("share scan stopped at SLSKR_SHARE_SCAN_MAX_FILES".to_owned());
                return;
            }

            if !reserve_share_scan_entry(output.scanned_entries) {
                output.errors.push(SHARE_SCAN_ENTRY_LIMIT_ERROR.to_owned());
                return;
            }

            let Ok(child) = child else {
                output.errors.push(format!(
                    "{}: entry unreadable",
                    json_safe_share_label(label)
                ));
                continue;
            };
            let path = child.path();
            if exclusions.iter().any(|excluded| path.starts_with(excluded)) {
                continue;
            }
            let filter_path = path.to_string_lossy().replace('\\', "/");
            if filters.iter().any(|filter| filter.is_match(&filter_path)) {
                continue;
            }
            if !options.include_hidden && is_hidden_share_path(root, &path) {
                continue;
            }

            let metadata = if options.follow_symlinks {
                fs::metadata(&path)
            } else {
                fs::symlink_metadata(&path)
            };
            let Ok(metadata) = metadata else {
                output.errors.push(format!(
                    "{}: entry metadata unavailable",
                    json_safe_share_label(label)
                ));
                continue;
            };
            if metadata.file_type().is_symlink() && !options.follow_symlinks {
                continue;
            }
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            if let Some(canonical_root) = canonical_root.as_ref() {
                let Ok(canonical_path) = path.canonicalize() else {
                    output.errors.push(format!(
                        "{}: entry canonical path unavailable",
                        json_safe_share_label(label)
                    ));
                    continue;
                };
                if !canonical_path.starts_with(canonical_root) {
                    continue;
                }
            }

            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let filename = format!("{}/{}", label, virtual_share_path(relative));
            let extension = extension_for(&filename);
            let attributes = if options.probe_media_attributes {
                probe_media_attributes(&path, &extension, metadata.len())
            } else {
                Vec::new()
            };
            output.local_paths.insert(filename.clone(), path.clone());
            output.entries.push(FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: filename.clone(),
                size: metadata.len(),
                extension,
                attributes,
            });
        }
    }
}

#[cfg(unix)]
fn scan_share_root_unix(
    root: &Path,
    label: &str,
    exclusions: &[PathBuf],
    filters: &[ControllerRegex],
    options: ShareScanOptions,
    output: &mut ShareScanOutput<'_>,
) {
    use rustix::fs::{open, openat, Dir, Mode, OFlags};
    use std::os::unix::ffi::OsStrExt;

    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file_flags = OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let root_directory = match open(root, directory_flags, Mode::empty()) {
        Ok(directory) => directory,
        Err(_) => {
            output.errors.push(format!(
                "{}: directory unavailable",
                json_safe_share_label(label)
            ));
            return;
        }
    };
    let mut stack = vec![(PathBuf::new(), root_directory)];
    while let Some((relative_directory, directory)) = stack.pop() {
        if options.cancellation.load(Ordering::Relaxed) {
            return;
        }
        if output.entries.len() >= options.max_files {
            output
                .errors
                .push("share scan stopped at SLSKR_SHARE_SCAN_MAX_FILES".to_owned());
            return;
        }
        let mut reader = match Dir::read_from(&directory) {
            Ok(reader) => reader,
            Err(_) => {
                output.errors.push(format!(
                    "{}: directory unreadable",
                    json_safe_share_label(label)
                ));
                continue;
            }
        };
        while let Some(child) = reader.read() {
            if options.cancellation.load(Ordering::Relaxed) {
                return;
            }
            if output.entries.len() >= options.max_files {
                output
                    .errors
                    .push("share scan stopped at SLSKR_SHARE_SCAN_MAX_FILES".to_owned());
                return;
            }
            if !reserve_share_scan_entry(output.scanned_entries) {
                output.errors.push(SHARE_SCAN_ENTRY_LIMIT_ERROR.to_owned());
                return;
            }
            let Ok(child) = child else {
                output.errors.push(format!(
                    "{}: entry unreadable",
                    json_safe_share_label(label)
                ));
                continue;
            };
            let name = child.file_name();
            if matches!(name.to_bytes(), b"." | b"..") {
                continue;
            }
            let name_os = std::ffi::OsStr::from_bytes(name.to_bytes());
            if !options.include_hidden && name_os.to_string_lossy().starts_with('.') {
                continue;
            }
            let relative = relative_directory.join(name_os);
            if filters
                .iter()
                .any(|filter| filter.is_match(&root.join(&relative).to_string_lossy()))
            {
                continue;
            }
            if exclusions
                .iter()
                .any(|excluded| root.join(&relative).starts_with(excluded))
            {
                continue;
            }
            if let Ok(child_directory) = openat(&directory, name, directory_flags, Mode::empty()) {
                if stack.len() >= MAX_SHARE_SCAN_PENDING_DIRECTORIES {
                    output
                        .errors
                        .push(SHARE_SCAN_PENDING_DIRECTORY_LIMIT_ERROR.to_owned());
                    return;
                }
                stack.push((relative, child_directory));
                continue;
            }
            let Ok(file) = openat(&directory, name, file_flags, Mode::empty()) else {
                continue;
            };
            let mut file = fs::File::from(file);
            let Ok(metadata) = file.metadata() else {
                output.errors.push(format!(
                    "{}: entry metadata unavailable",
                    json_safe_share_label(label)
                ));
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            let filename = format!("{}/{}", label, virtual_share_path(&relative));
            let extension = extension_for(&filename);
            let attributes = if options.probe_media_attributes {
                probe_media_attributes_from_file(&mut file, &extension, metadata.len())
            } else {
                Vec::new()
            };
            output
                .local_paths
                .insert(filename.clone(), root.join(&relative));
            output.entries.push(FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: filename.clone(),
                size: metadata.len(),
                extension,
                attributes,
            });
        }
    }
}
