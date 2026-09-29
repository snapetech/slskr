//! Compare the production fingerprinted search deduplicator with its former
//! linear `Vec::contains` scan under a duplicate-heavy peer response workload.

use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use slskr_client::search::SearchResults;
use slskr_protocol::{
    peer::{FileAttribute, FileEntry, FileSearchResponse, PeerMessage},
    ProtocolTextEncoding,
};

const TOKEN: u32 = 77;
const SEED_RESPONSES: usize = 900;
const FILES_PER_RESPONSE: usize = 11;
const DUPLICATE_ATTEMPTS: usize = 900;
const TRIALS: usize = 7;

fn entry(filename: String) -> FileEntry {
    FileEntry {
        code: 1,
        filename,
        filename_encoding: ProtocolTextEncoding::Utf8,
        size: 4_194_304,
        extension: "flac".to_owned(),
        extension_encoding: ProtocolTextEncoding::Utf8,
        attributes: vec![FileAttribute {
            code: 0,
            value: 44_100,
        }],
    }
}

fn responses() -> Vec<FileSearchResponse> {
    let common: Vec<_> = (0..FILES_PER_RESPONSE - 1)
        .map(|index| entry(format!("shared/track-{index:02}-common-name.flac")))
        .collect();

    (0..SEED_RESPONSES)
        .map(|index| {
            let mut results = common.clone();
            results.push(entry(format!("shared/track-unique-{index:04}.flac")));
            FileSearchResponse {
                username: "benchmark-peer".to_owned(),
                token: TOKEN,
                results,
                slot_free: true,
                average_speed: 1_000_000,
                queue_length: 0,
                unknown: 0,
                private_results: Vec::new(),
            }
        })
        .collect()
}

fn seed_current(seed: &[FileSearchResponse]) -> SearchResults {
    let mut results = SearchResults::new();
    for response in seed {
        results
            .accept_peer_message(PeerMessage::FileSearchResponse(response.clone()))
            .expect("valid benchmark response");
    }
    assert_eq!(results.responses_for(TOKEN).len(), SEED_RESPONSES);
    results
}

fn seed_linear(seed: &[FileSearchResponse]) -> Vec<FileSearchResponse> {
    seed.to_vec()
}

fn measure_current(seed: &[FileSearchResponse], duplicate: &FileSearchResponse) -> Duration {
    let mut results = seed_current(seed);
    let started = Instant::now();
    for _ in 0..DUPLICATE_ATTEMPTS {
        results
            .accept_peer_message(PeerMessage::FileSearchResponse(duplicate.clone()))
            .expect("valid duplicate response");
    }
    let elapsed = started.elapsed();
    assert_eq!(results.responses_for(TOKEN).len(), SEED_RESPONSES);
    elapsed
}

fn measure_linear(seed: &[FileSearchResponse], duplicate: &FileSearchResponse) -> Duration {
    let mut results = seed_linear(seed);
    let started = Instant::now();
    for _ in 0..DUPLICATE_ATTEMPTS {
        if !results.contains(duplicate) {
            results.push(duplicate.clone());
        }
    }
    let elapsed = started.elapsed();
    assert_eq!(results.len(), SEED_RESPONSES);
    elapsed
}

fn median(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn output_path() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--output" {
            return Some(PathBuf::from(
                args.next().expect("--output requires a path"),
            ));
        }
        panic!("unknown argument: {}", argument.to_string_lossy());
    }
    None
}

fn main() {
    let seed = responses();
    let duplicate = seed.last().expect("seed responses").clone();

    // Warm the allocator and instruction/data caches before collecting samples.
    let _ = measure_current(&seed, &duplicate);
    let _ = measure_linear(&seed, &duplicate);

    let mut current = Vec::with_capacity(TRIALS);
    let mut linear = Vec::with_capacity(TRIALS);
    for _ in 0..TRIALS {
        current.push(measure_current(&seed, &duplicate).as_nanos());
        linear.push(measure_linear(&seed, &duplicate).as_nanos());
    }
    let current_median = median(current);
    let linear_median = median(linear);
    let speedup = linear_median as f64 / current_median as f64;
    let result = format!(
        concat!(
            "{{\n",
            "  \"schema\": 1,\n",
            "  \"workload\": \"duplicate-heavy search response acceptance\",\n",
            "  \"seed_responses\": {},\n",
            "  \"files_per_response\": {},\n",
            "  \"shared_files_before_unique_tail\": {},\n",
            "  \"duplicate_attempts\": {},\n",
            "  \"trials\": {},\n",
            "  \"current_fingerprint_median_ns\": {},\n",
            "  \"former_linear_scan_median_ns\": {},\n",
            "  \"linear_over_current_speedup\": {:.3},\n",
            "  \"current_file_entries_hashed\": {},\n",
            "  \"linear_response_comparisons_upper_bound\": {},\n",
            "  \"linear_file_entry_comparisons_upper_bound\": {},\n",
            "  \"notes\": \"Synthetic diagnostic; no CI timing threshold or production-peer claim.\"\n",
            "}}\n"
        ),
        SEED_RESPONSES,
        FILES_PER_RESPONSE,
        FILES_PER_RESPONSE - 1,
        DUPLICATE_ATTEMPTS,
        TRIALS,
        current_median,
        linear_median,
        speedup,
        DUPLICATE_ATTEMPTS * FILES_PER_RESPONSE,
        SEED_RESPONSES * DUPLICATE_ATTEMPTS,
        SEED_RESPONSES * DUPLICATE_ATTEMPTS * FILES_PER_RESPONSE,
    );

    if let Some(path) = output_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create benchmark artifact directory");
        }
        fs::write(&path, &result).expect("write benchmark artifact");
    }
    print!("{result}");
}
