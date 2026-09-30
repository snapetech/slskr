use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
    time::UNIX_EPOCH,
};

async fn sha256_local_file(path: &Path) -> Option<String> {
    use tokio::io::AsyncReadExt as _;

    let mut file = tokio::fs::File::open(path).await.ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 32 * 1024];
    loop {
        let read = file.read(&mut buffer).await.ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Some(hex::encode(hasher.finalize()))
}

const MAX_LOCAL_SHA256_CACHE_ENTRIES: usize = 4_096;

#[derive(Clone, Debug)]
struct LocalSha256CacheEntry {
    size: u64,
    modified_nanos: Option<u128>,
    digest: String,
}

static LOCAL_SHA256_CACHE: LazyLock<Mutex<BTreeMap<PathBuf, LocalSha256CacheEntry>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

/// Reuse local content hashes while still invalidating them when a file's
/// size or modification timestamp changes. Browser pages and stream/ticket
/// requests commonly resolve the same file back-to-back; hashing it again for
/// each route adds avoidable synchronous-looking I/O to the request path.
pub(super) async fn sha256_local_file_cached(path: &Path) -> Option<String> {
    let metadata = tokio::fs::metadata(path).await.ok()?;
    let size = metadata.len();
    let modified_nanos = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos());
    if let Ok(cache) = LOCAL_SHA256_CACHE.lock() {
        if let Some(cached) = cache
            .get(path)
            .filter(|cached| cached.size == size && cached.modified_nanos == modified_nanos)
        {
            return Some(cached.digest.clone());
        }
    }

    let digest = sha256_local_file(path).await?;
    if let Ok(mut cache) = LOCAL_SHA256_CACHE.lock() {
        if cache.len() >= MAX_LOCAL_SHA256_CACHE_ENTRIES {
            let _ = cache.pop_first();
        }
        cache.insert(
            path.to_path_buf(),
            LocalSha256CacheEntry {
                size,
                modified_nanos,
                digest: digest.clone(),
            },
        );
    }
    Some(digest)
}
