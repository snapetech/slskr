use super::{
    query_bool, SLSKD_STORAGE_DIRECT_LIST_DEFAULT_ENTRIES,
    SLSKD_STORAGE_RECURSIVE_LIST_DEFAULT_ENTRIES,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct StorageDirectoryListOptions {
    pub(super) recursive: bool,
    pub(super) limit: usize,
}

impl StorageDirectoryListOptions {
    pub(super) fn from_query(query: Option<&str>) -> Self {
        let recursive = query_bool(query, "recursive").unwrap_or(false);
        let default_limit = if recursive {
            SLSKD_STORAGE_RECURSIVE_LIST_DEFAULT_ENTRIES
        } else {
            SLSKD_STORAGE_DIRECT_LIST_DEFAULT_ENTRIES
        };
        Self {
            recursive,
            limit: default_limit,
        }
    }
}

#[derive(Debug)]
pub(super) struct StorageDirectoryListState {
    pub(super) options: StorageDirectoryListOptions,
    pub(super) emitted: usize,
    pub(super) truncated: bool,
}

impl StorageDirectoryListState {
    pub(super) fn new(options: StorageDirectoryListOptions) -> Self {
        Self {
            options,
            emitted: 0,
            truncated: false,
        }
    }

    pub(super) fn reserve_entry(&mut self) -> bool {
        if self.emitted >= self.options.limit {
            self.truncated = true;
            return false;
        }
        self.emitted += 1;
        true
    }
}
