use super::*;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FiltersFileConfig {
    pub(in crate::config) search: SearchFiltersFileConfig,
    pub(in crate::config) search_retention: SearchRetentionFileConfig,
    pub(in crate::config) download: DownloadFiltersFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchFiltersFileConfig {
    pub(in crate::config) request: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchRetentionFileConfig {
    pub(in crate::config) max_age_days: Option<u64>,
    pub(in crate::config) max_count: Option<usize>,
    pub(in crate::config) cleanup_interval_seconds: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DownloadFiltersFileConfig {
    pub(in crate::config) exclude: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShareFileConfig {
    pub(in crate::config) dirs: Vec<String>,
    pub(in crate::config) fixture: Option<String>,
    pub(in crate::config) follow_symlinks: Option<bool>,
    pub(in crate::config) include_hidden: Option<bool>,
    pub(in crate::config) scan_max_files: Option<usize>,
    pub(in crate::config) cache_tsv_enabled: Option<bool>,
    pub(in crate::config) cache: ShareCacheFileConfig,
    pub(in crate::config) probe_media_attributes: Option<bool>,
    pub(in crate::config) filters: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShareCacheFileConfig {
    pub(in crate::config) storage_mode: Option<String>,
    pub(in crate::config) workers: Option<usize>,
    pub(in crate::config) retention: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AutoReplaceFileConfig {
    pub(in crate::config) interval_seconds: Option<u64>,
    pub(in crate::config) size_threshold_percent: Option<f64>,
    pub(in crate::config) max_retries: Option<usize>,
}
