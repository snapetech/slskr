use super::*;

#[derive(Debug, Default)]
pub(super) struct RecordListFilter {
    pub(super) q: Option<String>,
    pub(super) status: Option<String>,
    pub(super) target: Option<String>,
    pub(super) direction: Option<String>,
    pub(super) username: Option<String>,
    pub(super) joined: Option<bool>,
    pub(super) kind: Option<String>,
    pub(super) topic: Option<String>,
    pub(super) limit: Option<usize>,
    pub(super) offset: usize,
}

impl RecordListFilter {
    pub(super) fn from_query(query: Option<&str>) -> Self {
        let mut filter = Self {
            limit: Some(DEFAULT_LIST_LIMIT),
            ..Self::default()
        };
        for (name, value) in query_params(query.unwrap_or_default()) {
            match name.as_str() {
                "q" => filter.q = non_empty(value.to_ascii_lowercase()),
                "status" => filter.status = non_empty(value),
                "target" => filter.target = non_empty(value),
                "direction" => filter.direction = non_empty(value),
                "username" => filter.username = non_empty(value),
                "joined" => filter.joined = parse_bool_value(&value),
                "kind" => filter.kind = non_empty(value),
                "topic" => filter.topic = non_empty(value),
                "limit" => filter.limit = Some(parse_list_limit(&value)),
                "offset" => filter.offset = value.parse::<usize>().unwrap_or(0),
                _ => {}
            }
        }
        filter
    }
}

pub(super) fn parse_list_limit(value: &str) -> usize {
    value
        .parse::<usize>()
        .ok()
        .filter(|limit| *limit > 0)
        .map(|limit| limit.min(DEFAULT_LIST_LIMIT))
        .unwrap_or(DEFAULT_LIST_LIMIT)
}
