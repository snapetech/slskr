//! Compare incremental search-result persistence with full projection rewrites.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use slskr::{DatabaseManager, RfBenchmarkSearchRecord as SearchRecord, SearchResultRecord};

const SEARCH_ID: &str = "rf045-benchmark-search";
const EXTERNAL_ID: &str = "rf045-benchmark-external";
const RESULT_COUNT: usize = 10_000;
const BATCH_SIZE: usize = 200;
const BATCH_COUNT: usize = RESULT_COUNT / BATCH_SIZE;
const TRIALS: usize = 3;
const LEGACY_REINSERTED_ROWS: usize = BATCH_SIZE * BATCH_COUNT * (BATCH_COUNT + 1) / 2;

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn kill_and_wait(&mut self) -> ExitStatus {
        let child = self.0.as_mut().expect("child process is present");
        child.kill().expect("kill crash-proof child");
        let status = child.wait().expect("wait for crash-proof child");
        self.0.take();
        status
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn make_results() -> Vec<SearchResultRecord> {
    (0..RESULT_COUNT)
        .map(|index| SearchResultRecord {
            id: 0,
            search_id: SEARCH_ID.to_owned(),
            peer_username: Some(format!("peer-{:04}", index % 64)),
            filename: format!("artist/{index:05}-track.flac"),
            size: 4_000_000 + index as i64,
            extension: "flac".to_owned(),
            bit_rate: Some(320),
            sample_rate: Some(44_100),
            bit_depth: Some(16),
            length_seconds: Some(240),
            locked: false,
            slot_free: Some(true),
            average_speed: Some(1_000_000),
            queue_length: Some(0),
            created_at: index as i64,
        })
        .collect()
}

fn search_record(result_count: usize) -> SearchRecord {
    SearchRecord {
        id: SEARCH_ID.to_owned(),
        query: "rf045 synthetic result burst".to_owned(),
        status: "active".to_owned(),
        result_count: result_count as i64,
        created_at: 1,
        completed_at: None,
        room: None,
        target: Some("global".to_owned()),
        fallback_attempts: 0,
    }
}

async fn persist_batches(
    db: &DatabaseManager,
    results: &[SearchResultRecord],
    rewrite_full_projection: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    for (batch_index, batch) in results.chunks(BATCH_SIZE).enumerate() {
        let end = (batch_index + 1) * BATCH_SIZE;
        let record = search_record(end);
        if rewrite_full_projection {
            db.persist_search(&record, EXTERNAL_ID, &results[..end])
                .await?;
        } else {
            db.append_search_results(&record, EXTERNAL_ID, batch)
                .await?;
        }
    }
    Ok(())
}

async fn verify_projection(db: &DatabaseManager, results: &[SearchResultRecord]) {
    let stored = db
        .list_search_results(Some(SEARCH_ID), RESULT_COUNT as i32, 0)
        .await
        .expect("read persisted search result projection");
    assert_eq!(stored.len(), RESULT_COUNT);
    assert_eq!(
        stored.first().unwrap().filename,
        results.first().unwrap().filename
    );
    assert_eq!(
        stored.last().unwrap().filename,
        results.last().unwrap().filename
    );
    assert_eq!(
        db.get_search(SEARCH_ID)
            .await
            .expect("read persisted search metadata")
            .expect("search projection exists")
            .result_count,
        RESULT_COUNT as i64
    );
}

async fn run_trial(results: &[SearchResultRecord], rewrite_full_projection: bool) -> Duration {
    let db = DatabaseManager::in_memory()
        .await
        .expect("create in-memory database");
    let started = Instant::now();
    persist_batches(&db, results, rewrite_full_projection)
        .await
        .expect("persist search-result batches");
    let elapsed = started.elapsed();
    verify_projection(&db, results).await;
    elapsed
}

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("slskr-rf045-{nonce}"))
}

async fn crash_writer(path: &Path) {
    let db = DatabaseManager::new(path.to_str().expect("UTF-8 temporary database path"))
        .await
        .expect("create crash-proof database");
    let results = make_results();
    persist_batches(&db, &results, false)
        .await
        .expect("commit crash-proof result projection");
    println!("READY");
    std::io::stdout().flush().expect("flush readiness marker");
    std::thread::sleep(Duration::from_secs(15));
}

async fn crash_restart_proof(root: &Path, results: &[SearchResultRecord]) -> usize {
    let db_path = root.join("crash-restart.sqlite");
    let mut child = ChildGuard(Some(
        Command::new(std::env::current_exe().expect("current example path"))
            .arg("--crash-writer")
            .arg(&db_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("start crash-proof child"),
    ));
    let stdout = child
        .0
        .as_mut()
        .expect("child process is present")
        .stdout
        .take()
        .expect("capture crash-proof readiness marker");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("read crash-proof readiness marker");
    assert_eq!(line.trim(), "READY");
    drop(reader);

    let status = child.kill_and_wait();
    assert!(
        !status.success(),
        "crash-proof child must be terminated abruptly"
    );

    let db = DatabaseManager::new(db_path.to_str().expect("UTF-8 temporary database path"))
        .await
        .expect("reopen search database after child termination");
    verify_projection(&db, results).await;
    RESULT_COUNT
}

fn median(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args_os().skip(1);
    if let Some(argument) = args.next() {
        if argument == "--crash-writer" {
            let path = PathBuf::from(args.next().expect("--crash-writer requires a path"));
            assert!(args.next().is_none(), "unexpected crash-writer argument");
            crash_writer(&path).await;
            return;
        }
        assert_eq!(argument, "--output", "unknown argument");
        let path = PathBuf::from(args.next().expect("--output requires a path"));
        assert!(args.next().is_none(), "unexpected benchmark argument");
        run(Some(path)).await;
    } else {
        run(None).await;
    }
}

async fn run(output: Option<PathBuf>) {
    let results = make_results();
    let mut current = Vec::with_capacity(TRIALS);
    let mut legacy = Vec::with_capacity(TRIALS);
    for trial in 0..TRIALS {
        if trial % 2 == 0 {
            current.push(run_trial(&results, false).await.as_nanos());
            legacy.push(run_trial(&results, true).await.as_nanos());
        } else {
            legacy.push(run_trial(&results, true).await.as_nanos());
            current.push(run_trial(&results, false).await.as_nanos());
        }
    }

    let root = temp_root();
    fs::create_dir(&root).expect("create temporary crash-proof directory");
    let crash_restart_rows = crash_restart_proof(&root, &results).await;
    let result = format!(
        concat!(
            "{{\n",
            "  \"schema\": 1,\n",
            "  \"workload\": \"10,000-result search projection\",\n",
            "  \"results\": {},\n",
            "  \"batch_size\": {},\n",
            "  \"batches\": {},\n",
            "  \"trials\": {},\n",
            "  \"current_delta_median_ns\": {},\n",
            "  \"legacy_full_rewrite_median_ns\": {},\n",
            "  \"current_result_rows_inserted\": {},\n",
            "  \"legacy_result_rows_reinserted\": {},\n",
            "  \"crash_restart_rows_verified\": {},\n",
            "  \"notes\": \"Synthetic in-memory SQLite diagnostic; timings have no CI threshold.\"\n",
            "}}\n"
        ),
        RESULT_COUNT,
        BATCH_SIZE,
        BATCH_COUNT,
        TRIALS,
        median(current),
        median(legacy),
        RESULT_COUNT,
        LEGACY_REINSERTED_ROWS,
        crash_restart_rows,
    );

    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create benchmark artifact directory");
        }
        fs::write(path, &result).expect("write benchmark artifact");
    }
    println!("{result}");
    fs::remove_dir_all(root).expect("remove temporary crash-proof directory");
}
