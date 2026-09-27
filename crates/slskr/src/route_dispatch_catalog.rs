use super::*;

pub(super) fn extended_controller_get_route(path: &str) -> bool {
    matches!(
        path,
        "/api/bridge/rooms"
            | "/api/hashdb/backfill/candidates"
            | "/api/source-feed-imports/history"
            | "/api/hashdb/metadata-processing"
            | "/api/hashdb/inventory/unhashed"
            | "/api/hashdb/key"
            | "/api/hashdb/optimize/analyze"
            | "/api/hashdb/optimize/slow-queries"
            | "/api/hashdb/peers"
            | "/api/hashdb/schema"
            | "/api/mesh/delta"
            | "/api/mesh/hello"
            | "/api/multisource/search"
            | "/api/multisource/users"
            | "/api/opinions"
            | "/api/opinions/summary"
            | "/api/overlay/blocklist"
            | "/api/overlay/connections"
            | "/api/overlay/stats"
            | "/api/podcore/backfill/stats"
            | "/api/podcore/content/metadata"
            | "/api/podcore/dht/stats"
            | "/api/podcore/discovery/all"
            | "/api/podcore/discovery/stats"
            | "/api/podcore/membership/stats"
            | "/api/podcore/messages/stats"
            | "/api/podcore/routing/stats"
            | "/api/podcore/signing/stats"
            | "/api/podcore/verification/stats"
            | "/api/quarantine-jury/audit"
            | "/api/quarantine-jury/requests"
            | "/api/signals/config"
            | "/api/signals/status"
            | "/api/songid/capabilities"
            | "/api/telemetry/prometheus"
            | "/api/telemetry/prometheus/kpis"
            | "/api/virtualsoulfind/disaster-mode/status"
    )
}

pub(super) fn decoded_segments_after(path: &str, prefix: &str) -> Option<Vec<String>> {
    let rest = path.strip_prefix(prefix)?;
    if rest.is_empty() {
        return None;
    }
    rest.split('/')
        .map(|segment| {
            let decoded = decoded_path_segment(segment).trim().to_owned();
            (!decoded.is_empty() && !decoded.contains('/')).then_some(decoded)
        })
        .collect()
}

fn activitypub_actor_mutation_path(path: &str) -> bool {
    decoded_segments_after(path, "/actors/").is_some_and(|segments| {
        segments.len() == 2 && matches!(segments[1].as_str(), "inbox" | "outbox")
    })
}

pub(super) fn extended_controller_dynamic_get_route(path: &str) -> bool {
    let exact_tail = |prefix: &str, count: usize| {
        decoded_segments_after(path, prefix).is_some_and(|segments| segments.len() == count)
    };
    exact_tail("/api/audio/canonical/", 1)
        || exact_tail("/api/audio/variants/dedupe/", 1)
        || exact_tail("/api/bridge/transfer/", 2)
        || (path.starts_with("/api/compatibility/users/") && path.ends_with("/browse"))
        || exact_tail("/api/downloads/", 1)
        || exact_tail("/api/source-feed-imports/history/", 1)
        || exact_tail("/api/capabilities/peers/", 1)
        || path
            .strip_prefix("/api/capabilities/peers/")
            .is_some_and(|segment| !segment.is_empty() && !segment.contains('/'))
        || exact_tail("/api/hashdb/inventory/by-size/", 1)
        || exact_tail("/api/hashdb/sync/since/", 1)
        || exact_tail("/api/listening-party/", 2)
        || exact_tail("/api/listening-party/radio/", 2)
        || exact_tail("/api/mesh/lookup/", 1)
        || (path.starts_with("/api/multisource/users/") && path.ends_with("/files"))
        || (path.starts_with("/api/musicbrainz/artist/") && path.ends_with("/release-graph"))
        || (path.starts_with("/api/musicbrainz/overlays/artist/")
            && path.ends_with("/release-graph"))
        || (path.starts_with("/api/musicbrainz/overlays/edits/")
            && (path.ends_with("/export-review") || path.ends_with("/routes")))
        || (path.starts_with("/api/musicbrainz/release-radar/notifications/")
            && path.ends_with("/routes"))
        || (path.starts_with("/api/playback/") && path.ends_with("/diagnostics"))
        || (path.starts_with("/api/podcore/") && dynamic_podcore_get_route(path))
        || exact_tail("/api/portforwarding/status/", 1)
        || (path.starts_with("/api/quarantine-jury/requests/")
            && decoded_segments_after(path, "/api/quarantine-jury/requests/")
                .is_some_and(|segments| matches!(segments.len(), 1 | 2)))
        || exact_tail("/api/ranking/history/", 1)
        || path
            .strip_prefix("/api/ranking/history/")
            .is_some_and(|segment| !segment.is_empty() && !segment.contains('/'))
        || (path.starts_with("/api/realm-subject-indexes/")
            && decoded_segments_after(path, "/api/realm-subject-indexes/")
                .is_some_and(|segments| matches!(segments.len(), 1..=3)))
        || exact_tail("/api/relay/streams/", 1)
        || (path.starts_with("/api/traces/") && path.ends_with("/summary"))
        || exact_tail("/api/virtualsoulfind/canonical/", 1)
}

pub(super) fn audio_blank_recording_id_path(path: &str) -> bool {
    ["/api/audio/canonical/", "/api/audio/variants/dedupe/"]
        .into_iter()
        .any(|prefix| {
            let Some(rest) = path.strip_prefix(prefix) else {
                return false;
            };
            !rest.is_empty() && !rest.contains('/') && decoded_path_segment(rest).trim().is_empty()
        })
}

pub(super) fn extended_controller_mutation_route(method: &str, path: &str) -> bool {
    match method {
        "GET" => matches!(
            path,
            "/api/bridge/admin/clients"
                | "/api/bridge/admin/config"
                | "/api/bridge/admin/dashboard"
                | "/api/bridge/admin/stats"
                | "/api/bridge/status"
        ),
        "DELETE" => {
            (path.starts_with("/api/rooms/") && path.matches('/').count() == 3)
                || (path.starts_with("/api/collections/") && path.contains("/items/"))
                || path.starts_with("/api/mediacore/publish/descriptor/")
                || path.starts_with("/api/opinions/")
                || path.starts_with("/api/overlay/blocklist/")
                || path.starts_with("/api/podcore/")
                || path.starts_with("/api/security/circuits/")
                || path == "/api/session"
                || path == "/api/soulseek/mesh-rendezvous/interest"
        }
        "POST" => {
            activitypub_actor_mutation_path(path)
                || path.starts_with("/mesh/http/")
                || matches!(
                    path,
                    "/api/audio/analyzers/migrate"
                        | "/api/bridge/download"
                        | "/api/bridge/search"
                        | "/api/bridge/start"
                        | "/api/bridge/stop"
                        | "/api/downloads"
                        | "/api/transfers/downloads"
                        | "/api/jobs/label-crate"
                        | "/api/library/scan"
                        | "/api/rooms"
                        | "/api/search"
                        | "/api/slskdn/library/remediate"
                        | "/api/slskdn/warm-cache/hints"
                        | "/api/application/dump"
                        | "/api/application/loopback"
                        | "/api/capabilities/parse"
                        | "/api/dht/announce"
                        | "/api/dht/discover"
                        | "/api/events"
                        | "/api/hashdb/backfill/from-history"
                        | "/api/hashdb/optimize/indexes"
                        | "/api/hashdb/optimize/profile"
                        | "/api/hashdb/optimize/vacuum"
                        | "/api/mediacore/contentid/register"
                        | "/api/mediacore/fuzzymatch/perceptual"
                        | "/api/mediacore/fuzzymatch/text"
                        | "/api/mediacore/perceptualhash/audio"
                        | "/api/mediacore/perceptualhash/image"
                        | "/api/mediacore/perceptualhash/similarity"
                        | "/api/mediacore/portability/analyze"
                        | "/api/mediacore/portability/export"
                        | "/api/mediacore/portability/import"
                        | "/api/mediacore/publish/batch"
                        | "/api/mediacore/publish/descriptor"
                        | "/api/mediacore/publish/republish"
                        | "/api/mediacore/retrieve/batch"
                        | "/api/mediacore/retrieve/cache/clear"
                        | "/api/mediacore/retrieve/verify"
                        | "/api/mediacore/stats/reset"
                        | "/api/mesh/merge"
                        | "/api/mesh/message"
                        | "/api/mesh/nat/detect"
                        | "/api/mesh/publish"
                        | "/api/multisource/download-file"
                        | "/api/multisource/file-sources"
                        | "/api/multisource/test"
                        | "/api/multisource/verify"
                        | "/api/musicbrainz/library-bloom/diffs"
                        | "/api/musicbrainz/library-bloom/snapshots/preview"
                        | "/api/musicbrainz/library-bloom/wishlist"
                        | "/api/musicbrainz/overlays/edits"
                        | "/api/musicbrainz/release-radar/observations"
                        | "/api/nowplaying/webhook"
                        | "/api/opinions"
                        | "/api/options"
                        | "/api/overlay/blocklist/ip"
                        | "/api/overlay/blocklist/username"
                        | "/api/overlay/connect"
                        | "/api/playback/feedback"
                        | "/api/podcore/backfill/sync-all"
                        | "/api/podcore/content/create-pod"
                        | "/api/podcore/content/validate"
                        | "/api/podcore/dht/publish"
                        | "/api/podcore/dht/update"
                        | "/api/podcore/discovery/refresh"
                        | "/api/podcore/discovery/register"
                        | "/api/podcore/discovery/update"
                        | "/api/podcore/membership/cleanup"
                        | "/api/podcore/messages/rebuild-index"
                        | "/api/podcore/messages/vacuum"
                        | "/api/podcore/routing/cleanup"
                        | "/api/podcore/routing/route"
                        | "/api/podcore/routing/route-to-peers"
                        | "/api/podcore/signing/generate-keypair"
                        | "/api/podcore/signing/sign"
                        | "/api/podcore/signing/verify"
                        | "/api/podcore/verification/message"
                        | "/api/quarantine-jury/requests"
                        | "/api/quarantine-jury/verdicts"
                        | "/api/ranking/history"
                        | "/api/ranking/rank"
                        | "/api/searches/cleanup"
                        | "/api/security/circuits"
                        | "/api/security/entropy/check"
                        | "/api/security/tor/test"
                        | "/api/security/transports/test"
                        | "/api/share-grants/announce"
                        | "/api/solid/resolve-webid"
                        | "/api/soulseek/mesh-rendezvous/interest"
                )
                || path.starts_with("/api/hashdb/optimize/")
                || path.starts_with("/api/collections/") && path.ends_with("/items/reorder")
                || path.starts_with("/api/listening-party/")
                || path.starts_with("/api/mediacore/fuzzymatch/find/")
                || path.starts_with("/api/mediacore/ipld/links/")
                || path.starts_with("/api/musicbrainz/artist/")
                || path.starts_with("/api/musicbrainz/overlays/edits/")
                || path.starts_with("/api/musicbrainz/release-radar/notifications/")
                || path.starts_with("/api/podcore/")
                || path.starts_with("/api/portforwarding/stop/")
                || path.starts_with("/api/quarantine-jury/requests/")
                || path.starts_with("/api/realm-subject-indexes/")
                || path.starts_with("/api/searches/")
                || path.starts_with("/api/streams/") && path.ends_with("/ticket")
        }
        "PUT" => {
            (path.starts_with("/api/collections/") && path.contains("/items/"))
                || path.starts_with("/api/conversations/")
                || path.starts_with("/api/mediacore/publish/descriptor/")
                || path.starts_with("/api/overlay/pins/")
                || path.starts_with("/api/podcore/")
                || path == "/api/bridge/admin/config"
                || path == "/api/security/adversarial"
                || path.starts_with("/api/security/disclosure/")
                || path.starts_with("/api/security/reputation/")
        }
        _ => false,
    }
}
