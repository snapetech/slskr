//! A temporary stream file retains cleanup ownership until its last consumer.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

pub(super) struct LocalStreamFile {
    // Rust drops fields in declaration order: close the file before cleanup.
    pub(super) file: fs::File,
    pub(super) length: u64,
    pub(super) content_type: String,
    pub(super) cleanup_path: Option<StreamCleanupPath>,
}

#[derive(Clone)]
pub(super) struct StreamCleanupPath(Arc<CleanupOwner>);

struct CleanupOwner(PathBuf);

impl From<PathBuf> for StreamCleanupPath {
    fn from(path: PathBuf) -> Self {
        Self(Arc::new(CleanupOwner(path)))
    }
}

impl AsRef<Path> for StreamCleanupPath {
    fn as_ref(&self) -> &Path {
        &self.0 .0
    }
}

impl Drop for CleanupOwner {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_file(&self.0) {
            if error.kind() != std::io::ErrorKind::NotFound {
                eprintln!("temporary stream cleanup failed: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_stream() -> (PathBuf, LocalStreamFile) {
        let path = std::env::temp_dir().join(format!("slskr-stream-{}", uuid::Uuid::new_v4()));
        fs::write(&path, b"stream").unwrap();
        let stream = LocalStreamFile {
            file: fs::File::open(&path).unwrap(),
            length: 6,
            content_type: "application/octet-stream".to_owned(),
            cleanup_path: Some(path.clone().into()),
        };
        (path, stream)
    }

    #[test]
    fn unconsumed_stream_closes_file_and_removes_owned_temporary_output() {
        let (path, stream) = temporary_stream();
        drop(stream);
        assert!(!path.exists());
    }

    #[test]
    fn last_consumer_owns_cleanup_and_explicit_removal_remains_idempotent() {
        for explicitly_remove in [false, true] {
            let (path, stream) = temporary_stream();
            let cleanup = stream.cleanup_path.clone().unwrap();
            drop(stream);
            assert!(path.exists());
            if explicitly_remove {
                fs::remove_file(&cleanup).unwrap();
            }
            drop(cleanup);
            assert!(!path.exists());
        }
    }
}
