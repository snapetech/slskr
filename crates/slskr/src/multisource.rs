use std::{
    collections::{BTreeMap, HashSet},
    fs,
    future::Future,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

use futures_util::{stream::FuturesUnordered, StreamExt};
use reqwest::{header, redirect::Policy, Client, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{
    io::AsyncWriteExt,
    sync::{RwLock, Semaphore},
    time::timeout,
};

use crate::utils::{is_blocked_outbound_ipv4, is_non_global_special_use_ipv6, nat64_embedded_ipv4};

pub const DEFAULT_CHUNK_SIZE: u64 = 512 * 1024;
const MIN_CHUNK_SIZE: u64 = 64 * 1024;
const MAX_CHUNK_SIZE: u64 = 8 * 1024 * 1024;
const MAX_SOURCES: usize = 16;
const MAX_CHUNKS: u64 = 65_536;
const MAX_FILE_SIZE: u64 = 16 * 1024 * 1024 * 1024;
const MAX_RETAINED_JOBS: usize = 256;
const MAX_CONCURRENT_EXECUTIONS: usize = 4;
const SOURCE_TIMEOUT: Duration = Duration::from_secs(30);
static EXECUTION_PERMITS: Semaphore = Semaphore::const_new(MAX_CONCURRENT_EXECUTIONS);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmRequest {
    pub filename: String,
    #[serde(alias = "size")]
    pub file_size: u64,
    pub expected_hash: Option<String>,
    pub output_path: Option<String>,
    #[serde(
        default = "default_chunk_size",
        deserialize_with = "deserialize_chunk_size"
    )]
    pub chunk_size: u64,
    #[serde(default)]
    pub sources: Vec<RangeSource>,
}

fn default_chunk_size() -> u64 {
    DEFAULT_CHUNK_SIZE
}

fn deserialize_chunk_size<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = i64::deserialize(deserializer)?;
    Ok(if value <= 0 {
        DEFAULT_CHUNK_SIZE
    } else {
        value as u64
    })
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeSource {
    pub username: String,
    #[serde(alias = "endpoint")]
    pub url: String,
    pub authorization: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkResult {
    pub index: u64,
    pub username: String,
    pub start_offset: u64,
    pub end_offset: u64,
    pub bytes_downloaded: u64,
    pub time_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmResult {
    pub id: String,
    pub success: bool,
    pub filename: String,
    pub output_path: String,
    pub bytes_downloaded: u64,
    pub total_time_ms: u64,
    pub sources_used: usize,
    pub final_hash: String,
    pub chunks: Vec<ChunkResult>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmJob {
    pub id: String,
    pub status: String,
    pub filename: String,
    pub output_path: String,
    pub file_size: u64,
    pub chunk_size: u64,
    pub sources: Vec<String>,
    pub completed_chunks: u64,
    pub total_chunks: u64,
    pub bytes_downloaded: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub result: Option<SwarmResult>,
}

#[derive(Debug, Default)]
pub struct SwarmStore {
    records: BTreeMap<String, SwarmJob>,
}

impl SwarmStore {
    pub fn insert(&mut self, job: SwarmJob) {
        while self.records.len() >= MAX_RETAINED_JOBS {
            let Some(oldest) = self
                .records
                .values()
                .min_by_key(|record| (record.created_at, record.id.as_str()))
                .map(|record| record.id.clone())
            else {
                break;
            };
            self.records.remove(&oldest);
        }
        self.records.insert(job.id.clone(), job);
    }

    pub fn get(&self, id: &str) -> Option<&SwarmJob> {
        self.records.get(id)
    }

    pub fn list(&self) -> Vec<&SwarmJob> {
        let mut records = self.records.values().collect::<Vec<_>>();
        records.sort_by_key(|record| std::cmp::Reverse((record.created_at, record.id.as_str())));
        records
    }

    pub fn invalidate_completed(&mut self, id: &str, error: &str, now: u64) {
        if let Some(job) = self.records.get_mut(id) {
            job.status = "failed".to_owned();
            job.updated_at = now;
            if let Some(result) = job.result.as_mut() {
                result.success = false;
                result.error = Some(error.to_owned());
            }
        }
    }

    fn update_progress(&mut self, id: &str, completed: u64, bytes: u64, now: u64) {
        if let Some(job) = self.records.get_mut(id) {
            job.completed_chunks = completed;
            job.bytes_downloaded = bytes;
            job.updated_at = now;
        }
    }

    fn finish(&mut self, id: &str, result: SwarmResult, now: u64) {
        if let Some(job) = self.records.get_mut(id) {
            job.status = if result.success {
                "completed"
            } else {
                "failed"
            }
            .to_owned();
            job.bytes_downloaded = result.bytes_downloaded;
            job.updated_at = now;
            job.result = Some(result);
        }
    }
}

pub fn validate_request(request: &mut SwarmRequest) -> Result<String, String> {
    request.filename = request.filename.trim().to_owned();
    if request.filename.is_empty() {
        return Err("filename is required".to_owned());
    }
    if request.file_size == 0 {
        return Err("size is required (exact file size in bytes)".to_owned());
    }
    if request.file_size > MAX_FILE_SIZE {
        return Err(format!("size exceeds the {MAX_FILE_SIZE} byte limit"));
    }
    if request.chunk_size == 0 {
        request.chunk_size = DEFAULT_CHUNK_SIZE;
    }
    if !(MIN_CHUNK_SIZE..=MAX_CHUNK_SIZE).contains(&request.chunk_size) {
        return Err(format!(
            "chunkSize must be between {MIN_CHUNK_SIZE} and {MAX_CHUNK_SIZE} bytes"
        ));
    }
    let chunk_count = request.file_size.div_ceil(request.chunk_size);
    if chunk_count > MAX_CHUNKS {
        return Err(format!("download exceeds the {MAX_CHUNKS} chunk limit"));
    }
    if request.sources.len() < 2 {
        return Err("at least two range sources are required".to_owned());
    }
    if request.sources.len() > MAX_SOURCES {
        return Err(format!(
            "source count exceeds the {MAX_SOURCES} source limit"
        ));
    }
    let mut usernames = HashSet::new();
    for source in &mut request.sources {
        source.username = source.username.trim().to_owned();
        if source.username.is_empty() {
            return Err("every source requires a username".to_owned());
        }
        if !usernames.insert(source.username.to_ascii_lowercase()) {
            return Err("source usernames must be unique".to_owned());
        }
        if source
            .authorization
            .as_ref()
            .is_some_and(|value| value.contains(['\r', '\n']) || value.len() > 8 * 1024)
        {
            return Err("source authorization is invalid".to_owned());
        }
    }
    let expected = request
        .expected_hash
        .as_deref()
        .map(str::trim)
        .filter(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| "expectedHash must be a 64-character SHA-256 digest".to_owned())?
        .to_ascii_lowercase();
    request.expected_hash = Some(expected.clone());
    Ok(expected)
}

pub fn new_job(
    id: String,
    request: &SwarmRequest,
    public_output_path: String,
    now: u64,
) -> SwarmJob {
    SwarmJob {
        id,
        status: "queued".to_owned(),
        filename: request.filename.clone(),
        output_path: public_output_path,
        file_size: request.file_size,
        chunk_size: request.chunk_size,
        sources: request
            .sources
            .iter()
            .map(|source| source.username.clone())
            .collect(),
        completed_chunks: 0,
        total_chunks: request.file_size.div_ceil(request.chunk_size),
        bytes_downloaded: 0,
        created_at: now,
        updated_at: now,
        result: None,
    }
}

#[derive(Clone)]
struct PreparedSource {
    username: String,
    url: Url,
    authorization: Option<String>,
    client: Client,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChunkState {
    Pending,
    InFlight,
    Complete,
}

struct ChunkWork {
    index: u64,
    start: u64,
    end: u64,
    state: ChunkState,
    attempted: HashSet<usize>,
}

type FetchFuture = Pin<
    Box<dyn std::future::Future<Output = (usize, usize, Instant, Result<Vec<u8>, String>)> + Send>,
>;

pub async fn execute(
    id: String,
    request: SwarmRequest,
    output_path: PathBuf,
    public_output_path: String,
    store: Arc<RwLock<SwarmStore>>,
) -> SwarmResult {
    let started = Instant::now();
    let result = match EXECUTION_PERMITS.try_acquire() {
        Ok(_permit) => {
            execute_inner(
                &id,
                &request,
                &output_path,
                &public_output_path,
                &store,
                started,
            )
            .await
        }
        Err(_) => Err("multisource execution capacity is full".to_owned()),
    };
    let result = match result {
        Ok(result) => result,
        Err(error) => SwarmResult {
            id: id.clone(),
            success: false,
            filename: request.filename,
            output_path: public_output_path,
            bytes_downloaded: 0,
            total_time_ms: millis(started.elapsed()),
            sources_used: 0,
            final_hash: String::new(),
            chunks: Vec::new(),
            error: Some(error),
        },
    };
    store
        .write()
        .await
        .finish(&id, result.clone(), unix_timestamp());
    result
}

pub async fn fetch_single_verified_source(
    source: RangeSource,
    file_size: u64,
    expected_hash: &str,
    output_path: &Path,
) -> Result<String, String> {
    if file_size == 0 || file_size > MAX_FILE_SIZE {
        return Err("mesh preview size is outside the supported range".to_owned());
    }
    let expected_hash = expected_hash.trim().to_ascii_lowercase();
    if expected_hash.len() != 64 || !expected_hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("mesh preview expected hash is invalid".to_owned());
    }
    if output_path.exists() {
        return Err("mesh preview staging path already exists".to_owned());
    }
    let prepared = prepare_source(&source).await?;
    fetch_range(&prepared, 0, 0, file_size).await?;
    let output = open_private_file(output_path)
        .map_err(|_| "mesh preview staging file could not be created".to_owned())?;
    let mut output = tokio::fs::File::from_std(output);
    let operation = async {
        let mut hasher = Sha256::new();
        let mut offset = 0_u64;
        while offset < file_size {
            let end = offset.saturating_add(DEFAULT_CHUNK_SIZE).min(file_size) - 1;
            let bytes = fetch_range(&prepared, offset, end, file_size).await?;
            output
                .write_all(&bytes)
                .await
                .map_err(|_| "mesh preview staging write failed".to_owned())?;
            hasher.update(&bytes);
            offset = end + 1;
        }
        output
            .sync_all()
            .await
            .map_err(|_| "mesh preview staging sync failed".to_owned())?;
        let actual_hash = hex::encode(hasher.finalize());
        if actual_hash != expected_hash {
            return Err("mesh preview failed SHA-256 verification".to_owned());
        }
        Ok(actual_hash)
    }
    .await;
    drop(output);
    if operation.is_err() {
        let _ = fs::remove_file(output_path);
    }
    operation
}

async fn execute_inner(
    id: &str,
    request: &SwarmRequest,
    output_path: &Path,
    public_output_path: &str,
    store: &Arc<RwLock<SwarmStore>>,
    started: Instant,
) -> Result<SwarmResult, String> {
    if output_path.exists() {
        return Err("output file already exists".to_owned());
    }
    let parent = output_path
        .parent()
        .ok_or_else(|| "output path has no parent directory".to_owned())?;
    fs::create_dir_all(parent).map_err(|_| "output directory could not be created".to_owned())?;
    let temp_dir = parent.join(format!(".slskr-swarm-{}", uuid::Uuid::new_v4()));
    create_private_directory(&temp_dir)
        .map_err(|_| "swarm workspace could not be created".to_owned())?;

    let operation = async {
        let mut sources = Vec::with_capacity(request.sources.len());
        for source in &request.sources {
            sources.push(prepare_source(source).await?);
        }
        let file_size = request.file_size;
        let probes = sources.iter().map(|source| {
            let source = source.clone();
            async move { fetch_range(&source, 0, 0, file_size).await.map(|_| source) }
        });
        let probe_results = futures_util::future::join_all(probes).await;
        let sources = probe_results
            .into_iter()
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        if sources.len() < 2 {
            return Err("fewer than two sources passed range preflight".to_owned());
        }

        if let Some(job) = store.write().await.records.get_mut(id) {
            job.status = "in_progress".to_owned();
            job.sources = sources
                .iter()
                .map(|source| source.username.clone())
                .collect();
            job.updated_at = unix_timestamp();
        }

        let mut chunks = (0..request.file_size.div_ceil(request.chunk_size))
            .map(|index| {
                let start = index * request.chunk_size;
                ChunkWork {
                    index,
                    start,
                    end: start
                        .saturating_add(request.chunk_size)
                        .min(request.file_size)
                        - 1,
                    state: ChunkState::Pending,
                    attempted: HashSet::new(),
                }
            })
            .collect::<Vec<_>>();
        let mut source_busy = vec![false; sources.len()];
        let mut in_flight = FuturesUnordered::<FetchFuture>::new();
        let mut completed = 0_u64;
        let mut completed_bytes = 0_u64;
        let mut results = Vec::with_capacity(chunks.len());
        let mut source_cursor = 0_usize;

        while completed < chunks.len() as u64 {
            while let Some(chunk_index) = chunks
                .iter()
                .position(|chunk| chunk.state == ChunkState::Pending)
            {
                let source_index = (0..sources.len())
                    .map(|offset| (source_cursor + offset) % sources.len())
                    .find(|index| {
                        !source_busy[*index] && !chunks[chunk_index].attempted.contains(index)
                    });
                let Some(source_index) = source_index else {
                    break;
                };
                source_cursor = (source_index + 1) % sources.len();
                source_busy[source_index] = true;
                chunks[chunk_index].state = ChunkState::InFlight;
                chunks[chunk_index].attempted.insert(source_index);
                let source = sources[source_index].clone();
                let start = chunks[chunk_index].start;
                let end = chunks[chunk_index].end;
                let total = request.file_size;
                in_flight.push(Box::pin(async move {
                    let attempt_started = Instant::now();
                    let result = fetch_range(&source, start, end, total).await;
                    (chunk_index, source_index, attempt_started, result)
                }));
            }

            let Some((chunk_index, source_index, attempt_started, fetched)) =
                in_flight.next().await
            else {
                return Err("all range sources failed before the download completed".to_owned());
            };
            source_busy[source_index] = false;
            match fetched {
                Ok(bytes) => {
                    let path = temp_dir.join(format!("chunk-{chunk_index:08}.part"));
                    fs::write(&path, &bytes)
                        .map_err(|_| "downloaded chunk could not be stored".to_owned())?;
                    chunks[chunk_index].state = ChunkState::Complete;
                    completed += 1;
                    completed_bytes = completed_bytes.saturating_add(bytes.len() as u64);
                    results.push(ChunkResult {
                        index: chunks[chunk_index].index,
                        username: sources[source_index].username.clone(),
                        start_offset: chunks[chunk_index].start,
                        end_offset: chunks[chunk_index].end,
                        bytes_downloaded: bytes.len() as u64,
                        time_ms: millis(attempt_started.elapsed()),
                    });
                    store.write().await.update_progress(
                        id,
                        completed,
                        completed_bytes,
                        unix_timestamp(),
                    );
                }
                Err(_) if chunks[chunk_index].attempted.len() < sources.len() => {
                    chunks[chunk_index].state = ChunkState::Pending;
                }
                Err(_) => return Err("a chunk failed on every verified range source".to_owned()),
            }
        }

        results.sort_by_key(|result| result.index);
        let assembly_path = parent.join(format!(".slskr-swarm-{}.part", uuid::Uuid::new_v4()));
        let assembly_result = assemble_and_publish(
            &temp_dir,
            &assembly_path,
            output_path,
            chunks.len(),
            request.file_size,
            request.expected_hash.as_deref().unwrap_or_default(),
        );
        let _ = fs::remove_file(&assembly_path);
        let final_hash = assembly_result?;
        let sources_used = results
            .iter()
            .map(|result| result.username.to_ascii_lowercase())
            .collect::<HashSet<_>>()
            .len();
        Ok(SwarmResult {
            id: id.to_owned(),
            success: true,
            filename: request.filename.clone(),
            output_path: public_output_path.to_owned(),
            bytes_downloaded: request.file_size,
            total_time_ms: millis(started.elapsed()),
            sources_used,
            final_hash,
            chunks: results,
            error: None,
        })
    }
    .await;
    let _ = fs::remove_dir_all(&temp_dir);
    operation
}

async fn prepare_source(source: &RangeSource) -> Result<PreparedSource, String> {
    let url = Url::parse(&source.url).map_err(|_| "source URL is invalid".to_owned())?;
    if !matches!(url.scheme(), "http" | "https") || url.username() != "" || url.password().is_some()
    {
        return Err("source URL must be an HTTP(S) URL without credentials".to_owned());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "source URL has no host".to_owned())?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| "source URL has no usable port".to_owned())?;
    let addresses = resolve_source_addrs(
        async move {
            tokio::net::lookup_host((host, port))
                .await
                .map(|addresses| addresses.collect())
        },
        SOURCE_TIMEOUT,
    )
    .await?;
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| blocked_source_ip(address.ip()))
    {
        return Err("source host resolves to a blocked network".to_owned());
    }
    let mut builder = Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .timeout(SOURCE_TIMEOUT);
    for address in addresses {
        builder = builder.resolve(host, SocketAddr::new(address.ip(), port));
    }
    let client = builder
        .build()
        .map_err(|_| "source HTTP client could not be created".to_owned())?;
    Ok(PreparedSource {
        username: source.username.clone(),
        url,
        authorization: source.authorization.clone(),
        client,
    })
}

async fn resolve_source_addrs<F>(
    resolution: F,
    deadline: Duration,
) -> Result<Vec<SocketAddr>, String>
where
    F: Future<Output = std::io::Result<Vec<SocketAddr>>>,
{
    timeout(deadline, resolution)
        .await
        .map_err(|_| "source host resolution timed out".to_owned())?
        .map_err(|_| "source host could not be resolved".to_owned())
}

async fn fetch_range(
    source: &PreparedSource,
    start: u64,
    end: u64,
    total: u64,
) -> Result<Vec<u8>, String> {
    let mut request = source
        .client
        .get(source.url.clone())
        .header(header::RANGE, format!("bytes={start}-{end}"));
    if let Some(authorization) = source.authorization.as_deref() {
        request = request.header(header::AUTHORIZATION, authorization);
    }
    let response = request
        .send()
        .await
        .map_err(|_| "range source request failed".to_owned())?;
    if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err("range source did not return partial content".to_owned());
    }
    let content_range = response
        .headers()
        .get(header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "range source omitted Content-Range".to_owned())?;
    let expected_range = format!("bytes {start}-{end}/{total}");
    if content_range != expected_range {
        return Err("range source returned an unexpected Content-Range".to_owned());
    }
    let expected_length = end - start + 1;
    if response.content_length() != Some(expected_length) {
        return Err("range source returned an unexpected Content-Length".to_owned());
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| "range source body failed".to_owned())?;
    if bytes.len() as u64 != expected_length {
        return Err("range source body length did not match the requested chunk".to_owned());
    }
    Ok(bytes.to_vec())
}

fn assemble_and_publish(
    temp_dir: &Path,
    assembly_path: &Path,
    output_path: &Path,
    chunk_count: usize,
    file_size: u64,
    expected_hash: &str,
) -> Result<String, String> {
    let mut output = open_private_file(assembly_path)
        .map_err(|_| "assembly file could not be created".to_owned())?;
    let mut hasher = Sha256::new();
    let mut written = 0_u64;
    let mut buffer = vec![0_u8; 64 * 1024];
    for index in 0..chunk_count {
        let mut chunk = fs::File::open(temp_dir.join(format!("chunk-{index:08}.part")))
            .map_err(|_| "completed chunk could not be reopened".to_owned())?;
        loop {
            let count = chunk
                .read(&mut buffer)
                .map_err(|_| "completed chunk could not be read".to_owned())?;
            if count == 0 {
                break;
            }
            output
                .write_all(&buffer[..count])
                .map_err(|_| "assembled download could not be written".to_owned())?;
            hasher.update(&buffer[..count]);
            written = written.saturating_add(count as u64);
        }
    }
    output
        .sync_all()
        .map_err(|_| "assembled download could not be synchronized".to_owned())?;
    drop(output);
    if written != file_size {
        return Err("assembled download size did not match the request".to_owned());
    }
    let final_hash = hex::encode(hasher.finalize());
    if final_hash != expected_hash {
        return Err("assembled download failed SHA-256 verification".to_owned());
    }
    fs::hard_link(assembly_path, output_path).map_err(|_| {
        "verified download could not be published without overwriting a file".to_owned()
    })?;
    fs::remove_file(assembly_path)
        .map_err(|_| "published download staging file could not be removed".to_owned())?;
    if let Ok(parent) = fs::File::open(output_path.parent().unwrap_or_else(|| Path::new("."))) {
        let _ = parent.sync_all();
    }
    Ok(final_hash)
}

fn create_private_directory(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

fn open_private_file(path: &Path) -> std::io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

fn blocked_source_ip(ip: IpAddr) -> bool {
    if cfg!(test) && ip.is_loopback() {
        return false;
    }
    match ip {
        IpAddr::V4(ip) => blocked_source_ipv4(ip),
        IpAddr::V6(ip) => blocked_source_ipv6(ip),
    }
}

fn blocked_source_ipv4(ip: Ipv4Addr) -> bool {
    is_blocked_outbound_ipv4(ip)
}

fn blocked_source_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(ipv4) = ip.to_ipv4_mapped().or_else(|| ip.to_ipv4()) {
        return blocked_source_ipv4(ipv4);
    }
    if let Some(ipv4) = nat64_embedded_ipv4(ip) {
        return blocked_source_ipv4(ipv4);
    }
    let segments = ip.segments();
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || segments[0] == 0x2002
        || (segments[0] == 0x2001 && segments[1] == 0)
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] & 0xffc0) == 0xfec0
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || is_non_global_special_use_ipv6(ip)
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn source_dns_resolution_is_bounded() {
        let error = resolve_source_addrs(
            std::future::pending::<std::io::Result<Vec<SocketAddr>>>(),
            Duration::ZERO,
        )
        .await
        .unwrap_err();
        assert_eq!(error, "source host resolution timed out");
    }

    #[test]
    fn source_filter_blocks_ipv4_transition_and_special_use_ipv6() {
        for address in [
            "::ffff:127.0.0.1",
            "2002:c0a8:0101::1",
            "2001:0000:4136:e378::1",
            "64:ff9b::7f00:1",
            "64:ff9b:1::1",
            "100::1",
            "2001:2::1",
            "2001:10::1",
            "2001:20::1",
        ] {
            assert!(blocked_source_ip(address.parse().unwrap()), "{address}");
        }
    }
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn spawn_range_source(
        content: Arc<Vec<u8>>,
    ) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind range source");
        let address = listener.local_addr().expect("range source address");
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let content = Arc::clone(&content);
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    loop {
                        let count = stream.read(&mut buffer).await.expect("read range request");
                        if count == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..count]);
                        if request.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let request = String::from_utf8(request).expect("range request UTF-8");
                    let range = request
                        .lines()
                        .filter_map(|line| line.split_once(':'))
                        .find(|(name, _)| name.eq_ignore_ascii_case("range"))
                        .and_then(|(_, value)| value.trim().strip_prefix("bytes="))
                        .expect("range header");
                    let (start, end) = range.split_once('-').expect("range bounds");
                    let start = start.parse::<usize>().expect("range start");
                    let end = end.parse::<usize>().expect("range end");
                    let body = &content[start..=end];
                    let response = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{}\r\nConnection: close\r\n\r\n",
                        body.len(),
                        content.len()
                    );
                    stream
                        .write_all(response.as_bytes())
                        .await
                        .expect("write range headers");
                    stream.write_all(body).await.expect("write range body");
                });
            }
        });
        (address, task)
    }

    #[test]
    fn request_validation_applies_default_and_hard_bounds() {
        let parsed = serde_json::from_str::<SwarmRequest>(
            r#"{"filename":"x","fileSize":1,"chunkSize":-1,"sources":[]}"#,
        )
        .expect("negative chunk size request");
        assert_eq!(parsed.chunk_size, DEFAULT_CHUNK_SIZE);

        let mut request = SwarmRequest {
            filename: "Album/Track.flac".to_owned(),
            file_size: 128 * 1024,
            expected_hash: Some("ab".repeat(32)),
            output_path: None,
            chunk_size: 0,
            sources: vec![
                RangeSource {
                    username: "one".to_owned(),
                    url: "https://one.example/file".to_owned(),
                    authorization: None,
                },
                RangeSource {
                    username: "two".to_owned(),
                    url: "https://two.example/file".to_owned(),
                    authorization: None,
                },
            ],
        };
        assert_eq!(validate_request(&mut request).unwrap(), "ab".repeat(32));
        assert_eq!(request.chunk_size, DEFAULT_CHUNK_SIZE);

        request.file_size = MAX_FILE_SIZE + 1;
        assert_eq!(
            validate_request(&mut request).unwrap_err(),
            format!("size exceeds the {MAX_FILE_SIZE} byte limit")
        );
        request.file_size = 128 * 1024;
        request.sources.clear();
        assert_eq!(
            validate_request(&mut request).unwrap_err(),
            "at least two range sources are required"
        );
    }

    #[test]
    fn swarm_store_bounds_history_and_orders_by_creation_time() {
        let mut store = SwarmStore::default();
        for index in 0..=MAX_RETAINED_JOBS {
            let id = format!("job-{index:04}");
            store.insert(SwarmJob {
                id: id.clone(),
                status: "completed".to_owned(),
                filename: "file.flac".to_owned(),
                output_path: "downloads/file.flac".to_owned(),
                file_size: 1,
                chunk_size: MIN_CHUNK_SIZE,
                sources: Vec::new(),
                completed_chunks: 1,
                total_chunks: 1,
                bytes_downloaded: 1,
                created_at: index as u64,
                updated_at: index as u64,
                result: None,
            });
        }

        assert_eq!(store.records.len(), MAX_RETAINED_JOBS);
        assert!(store.get("job-0000").is_none());
        assert_eq!(store.list()[0].id, format!("job-{MAX_RETAINED_JOBS:04}"));
    }

    #[cfg(unix)]
    #[test]
    fn temporary_swarm_artifacts_have_private_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!(
            "slskr-swarm-permissions-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&root).expect("create permissions test root");
        let workspace = root.join("workspace");
        super::create_private_directory(&workspace).expect("create private workspace");
        let staging = root.join("staging.part");
        drop(super::open_private_file(&staging).expect("create private staging file"));

        assert_eq!(
            fs::metadata(&workspace)
                .expect("workspace metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&staging)
                .expect("staging metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        fs::remove_dir_all(root).expect("remove permissions test root");
    }

    #[tokio::test]
    async fn concurrent_range_sources_assemble_and_verify_exact_file() {
        let content = Arc::new(
            (0..(192 * 1024 + 17))
                .map(|index| (index % 251) as u8)
                .collect::<Vec<_>>(),
        );
        let expected_hash = hex::encode(Sha256::digest(content.as_slice()));
        let (first_address, first_server) = spawn_range_source(Arc::clone(&content)).await;
        let (second_address, second_server) = spawn_range_source(Arc::clone(&content)).await;
        let root = std::env::temp_dir().join(format!("slskr-swarm-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).expect("create swarm test root");
        let output_path = root.join("assembled.flac");
        let mut request = SwarmRequest {
            filename: "Remote/assembled.flac".to_owned(),
            file_size: content.len() as u64,
            expected_hash: Some(expected_hash.clone()),
            output_path: None,
            chunk_size: MIN_CHUNK_SIZE,
            sources: vec![
                RangeSource {
                    username: "first".to_owned(),
                    url: format!("http://{first_address}/file"),
                    authorization: None,
                },
                RangeSource {
                    username: "second".to_owned(),
                    url: format!("http://{second_address}/file"),
                    authorization: None,
                },
            ],
        };
        validate_request(&mut request).expect("valid swarm request");
        let id = uuid::Uuid::new_v4().to_string();
        let store = Arc::new(RwLock::new(SwarmStore::default()));
        store.write().await.insert(new_job(
            id.clone(),
            &request,
            "multisource/assembled.flac".to_owned(),
            unix_timestamp(),
        ));

        let result = execute(
            id.clone(),
            request,
            output_path.clone(),
            "multisource/assembled.flac".to_owned(),
            Arc::clone(&store),
        )
        .await;
        first_server.abort();
        second_server.abort();

        assert!(result.success, "swarm failed: {:?}", result.error);
        assert_eq!(result.output_path, "multisource/assembled.flac");
        assert_eq!(result.final_hash, expected_hash);
        assert_eq!(result.sources_used, 2);
        assert_eq!(result.chunks.len(), 4);
        assert_eq!(
            fs::read(&output_path).expect("read assembled file"),
            *content
        );
        let store = store.read().await;
        let job = store.get(&id).expect("completed swarm job");
        assert_eq!(job.status, "completed");
        assert_eq!(job.output_path, "multisource/assembled.flac");
        assert_eq!(job.completed_chunks, 4);
        assert_eq!(job.bytes_downloaded, content.len() as u64);
        drop(store);
        fs::remove_dir_all(root).expect("remove swarm test root");
    }

    #[tokio::test]
    async fn single_mesh_source_hash_failure_removes_staging_file() {
        let content = Arc::new(b"untrusted-mesh-bytes".to_vec());
        let (address, server) = spawn_range_source(content).await;
        let root =
            std::env::temp_dir().join(format!("slskr-mesh-preview-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).expect("create mesh preview test root");
        let output = root.join("preview.part");
        let result = fetch_single_verified_source(
            RangeSource {
                username: "mesh-peer".to_owned(),
                url: format!("http://{address}/content"),
                authorization: None,
            },
            20,
            &"00".repeat(32),
            &output,
        )
        .await;
        server.abort();

        assert_eq!(
            result.unwrap_err(),
            "mesh preview failed SHA-256 verification"
        );
        assert!(!output.exists());
        fs::remove_dir_all(root).expect("remove mesh preview test root");
    }
}
