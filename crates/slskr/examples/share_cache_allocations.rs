//! Compare the production share-cache writer with its previous allocator-heavy
//! serialization pattern using deterministic in-memory file entries.

use std::{alloc::System, fs, path::PathBuf, time::Instant};

use slskr::rf_benchmark_write_share_cache;
use slskr_client::protocol::{peer::FileEntry, ProtocolTextEncoding};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

const ENTRY_COUNT: usize = 50_000;
const TRIALS: usize = 7;

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(Clone, Copy)]
struct Sample {
    elapsed_ns: u128,
    allocation_calls: u64,
    requested_bytes: u64,
}

fn file_entry(index: usize) -> FileEntry {
    let suffix = match index % 17 {
        0 => "line\nbreak",
        1 => "tab\tvalue",
        2 => "slash\\value",
        _ => "plain",
    };
    FileEntry {
        code: 1,
        filename: format!("artist-{index:06}/track-{index:06}-{suffix}.flac"),
        filename_encoding: ProtocolTextEncoding::Utf8,
        size: 4_194_304 + index as u64,
        extension: "flac".to_owned(),
        extension_encoding: ProtocolTextEncoding::Utf8,
        attributes: Vec::new(),
    }
}

fn entries() -> Vec<FileEntry> {
    (0..ENTRY_COUNT).map(file_entry).collect()
}

fn legacy_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn write_legacy_share_cache(path: &std::path::Path, entries: &[FileEntry]) -> Result<(), String> {
    let content = entries
        .iter()
        .map(|entry| {
            format!(
                "{}\t{}\t{}\t{}",
                legacy_escape(&entry.filename),
                entry.size,
                entry.code,
                legacy_escape(&entry.extension)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, content).map_err(|error| error.to_string())
}

fn measure(write: impl FnOnce() -> Result<(), String>) -> Sample {
    let region = Region::new(ALLOCATOR);
    let started = Instant::now();
    let result = write();
    let elapsed = started.elapsed();
    result.expect("write share cache");
    let stats = region.change();
    Sample {
        elapsed_ns: elapsed.as_nanos(),
        allocation_calls: (stats.allocations + stats.reallocations) as u64,
        requested_bytes: stats.bytes_allocated as u64,
    }
}

fn median(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn median_duration(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn output_path() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    match args.next() {
        Some(argument) if argument == "--output" => Some(PathBuf::from(
            args.next().expect("--output requires a path"),
        )),
        Some(argument) => panic!("unknown argument: {}", argument.to_string_lossy()),
        None => None,
    }
}

fn main() {
    let entries = entries();
    let nonce = SystemTimeNonce::now();
    let root = std::env::temp_dir().join(format!("slskr-rf038-{nonce}"));
    fs::create_dir(&root).expect("create temporary benchmark directory");
    let current_path = root.join("current.tsv");
    let legacy_path = root.join("legacy.tsv");

    // Warm both paths, then verify byte-for-byte compatibility outside timing.
    write_legacy_share_cache(&legacy_path, &entries).expect("write legacy fixture");
    rf_benchmark_write_share_cache(&current_path, &entries).expect("write current fixture");
    assert_eq!(
        fs::read(&current_path).expect("read current fixture"),
        fs::read(&legacy_path).expect("read legacy fixture")
    );

    let mut current_samples = Vec::with_capacity(TRIALS);
    let mut legacy_samples = Vec::with_capacity(TRIALS);
    for trial in 0..TRIALS {
        if trial % 2 == 0 {
            current_samples.push(measure(|| {
                rf_benchmark_write_share_cache(&current_path, &entries)
            }));
            legacy_samples.push(measure(|| write_legacy_share_cache(&legacy_path, &entries)));
        } else {
            legacy_samples.push(measure(|| write_legacy_share_cache(&legacy_path, &entries)));
            current_samples.push(measure(|| {
                rf_benchmark_write_share_cache(&current_path, &entries)
            }));
        }
    }

    let result = format!(
        concat!(
            "{{\n",
            "  \"schema\": 1,\n",
            "  \"workload\": \"share-cache serialization\",\n",
            "  \"entries\": {},\n",
            "  \"trials\": {},\n",
            "  \"output_bytes\": {},\n",
            "  \"current_median_ns\": {},\n",
            "  \"legacy_median_ns\": {},\n",
            "  \"current_median_allocation_calls\": {},\n",
            "  \"legacy_median_allocation_calls\": {},\n",
            "  \"current_median_requested_bytes\": {},\n",
            "  \"legacy_median_requested_bytes\": {},\n",
            "  \"notes\": \"Synthetic diagnostic; output bytes verified identical; no CI timing threshold.\"\n",
            "}}\n"
        ),
        ENTRY_COUNT,
        TRIALS,
        fs::metadata(&current_path).expect("stat current output").len(),
        median_duration(current_samples.iter().map(|sample| sample.elapsed_ns).collect()),
        median_duration(legacy_samples.iter().map(|sample| sample.elapsed_ns).collect()),
        median(current_samples.iter().map(|sample| sample.allocation_calls).collect()),
        median(legacy_samples.iter().map(|sample| sample.allocation_calls).collect()),
        median(current_samples.iter().map(|sample| sample.requested_bytes).collect()),
        median(legacy_samples.iter().map(|sample| sample.requested_bytes).collect()),
    );

    if let Some(path) = output_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create benchmark artifact directory");
        }
        fs::write(path, &result).expect("write benchmark artifact");
    }
    print!("{result}");
    fs::remove_dir_all(root).expect("remove temporary benchmark directory");
}

struct SystemTimeNonce;

impl SystemTimeNonce {
    fn now() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after Unix epoch")
            .as_nanos()
    }
}
