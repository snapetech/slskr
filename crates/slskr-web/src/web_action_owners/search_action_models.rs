#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchCandidateRank {
    pub reasons: Vec<String>,
    pub score: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchDuplicateGroup {
    pub candidate_count: usize,
    pub folded_count: usize,
    pub key: String,
    pub providers: Vec<String>,
    pub usernames: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchActionPreview {
    pub candidate_score: Option<u32>,
    pub file_count: usize,
    pub filenames: Vec<String>,
    pub locked_count: usize,
    pub provider_labels: Vec<String>,
    pub route: String,
    pub total_size_bytes: u64,
    pub username: String,
    pub warnings: Vec<String>,
}
