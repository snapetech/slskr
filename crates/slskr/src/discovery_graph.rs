//! Discovery Graph backend used by the versioned controller route.
//!
//! The oracle builds this response from the SongID run store and the
//! MusicBrainz artist release-graph service.  Keep the response builder close
//! to those two slskR stores instead of manufacturing a graph from request
//! strings alone.

use std::cmp::Ordering;

use serde_json::{json, Map, Value};

use super::{routing, AppState};

const MINIMUM_TRACK_IDENTITY_FOR_WEAK_RUN: f64 = 0.70;
const MINIMUM_TRACK_IDENTITY_FOR_CATALOG_EXPANSION: f64 = 0.85;
const MINIMUM_SEGMENT_CONFIDENCE_FOR_GRAPH: f64 = 0.65;
const MINIMUM_SEGMENT_CANDIDATE_IDENTITY_FOR_GRAPH: f64 = 0.63;
const MINIMUM_MIX_IDENTITY_FOR_GRAPH: f64 = 0.60;

pub(crate) async fn build_response(body: &str, state: &AppState) -> routing::HttpResponse {
    let mut request = match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(request)) => Value::Object(request),
        Ok(_) => return routing::bad_request_response("Discovery Graph request is required."),
        Err(_) => return routing::bad_request_response("invalid JSON body"),
    };

    let scope = normalized_string(&request, "scope").unwrap_or_else(|| "songid_run".to_owned());
    request["scope"] = json!(scope);
    let requested_run_id = normalized_string(&request, "songIdRunId");
    let run = if let Some(run_id) = requested_run_id.as_deref() {
        let runtime = state.runtime.read().await;
        runtime.songid_run(run_id)
    } else {
        None
    };

    let mut graph = Graph::new(request.clone());
    match scope.to_ascii_lowercase().as_str() {
        "artist" => build_artist_graph(&mut graph, &request, run.as_ref(), state).await,
        "album" => build_album_graph(&mut graph, &request, run.as_ref()),
        "track" => build_track_graph(&mut graph, &request, run.as_ref()),
        _ => build_run_graph(
            &mut graph,
            &request,
            run.as_ref(),
            requested_run_id.as_deref(),
        ),
    }
    graph.finalize();
    add_comparison_overlay(&mut graph, &request, run.as_ref());
    graph.finalize();
    routing::ok_response(graph.to_value().to_string())
}

struct Graph {
    request: Value,
    title: String,
    summary: String,
    seed_node_id: String,
    nodes: Vec<Value>,
    edges: Vec<Value>,
}

impl Graph {
    fn new(request: Value) -> Self {
        Self {
            request,
            title: String::new(),
            summary: String::new(),
            seed_node_id: String::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    fn add_node(
        &mut self,
        node_id: impl Into<String>,
        label: impl Into<String>,
        node_type: &str,
        weight: f64,
        depth: i64,
        accent: &str,
        reason: &str,
    ) {
        let node_id = node_id.into();
        if self.nodes.iter().any(|node| {
            node["nodeId"]
                .as_str()
                .is_some_and(|existing| existing.eq_ignore_ascii_case(&node_id))
        }) {
            return;
        }
        let label = label.into();
        self.nodes.push(json!({
            "nodeId": node_id,
            "label": label,
            "subtitle": node_type.replace('_', " "),
            "nodeType": node_type,
            "accent": accent,
            "reason": reason,
            "weight": clamp_weight(weight),
            "depth": depth,
        }));
    }

    fn add_edge(
        &mut self,
        source_node_id: &str,
        target_node_id: &str,
        edge_type: &str,
        weight: f64,
        reason: &str,
        provenance: &str,
        score_components: &[(&str, f64)],
        evidence: &[String],
    ) {
        if self.edges.iter().any(|edge| {
            edge["sourceNodeId"]
                .as_str()
                .is_some_and(|source| source.eq_ignore_ascii_case(source_node_id))
                && edge["targetNodeId"]
                    .as_str()
                    .is_some_and(|target| target.eq_ignore_ascii_case(target_node_id))
                && edge["edgeType"]
                    .as_str()
                    .is_some_and(|kind| kind.eq_ignore_ascii_case(edge_type))
        }) {
            return;
        }

        let mut components = Map::new();
        for (lane, score) in score_components {
            components.insert((*lane).to_owned(), json!(score));
        }
        let evidence_lanes = edge_evidence_lanes(provenance, score_components, evidence);
        self.edges.push(json!({
            "sourceNodeId": source_node_id,
            "targetNodeId": target_node_id,
            "edgeType": edge_type,
            "weight": clamp_weight(weight),
            "reason": reason,
            "provenance": provenance,
            "scoreComponents": Value::Object(components),
            "evidence": evidence,
            "evidenceLanes": evidence_lanes,
        }));
    }

    fn finalize(&mut self) {
        self.seed_node_id = self
            .nodes
            .iter()
            .find(|node| node["depth"].as_i64() == Some(0))
            .and_then(|node| node["nodeId"].as_str())
            .unwrap_or_default()
            .to_owned();
    }

    fn to_value(&self) -> Value {
        json!({
            "title": self.title,
            "summary": self.summary,
            "seedNodeId": self.seed_node_id,
            "request": self.request,
            "nodes": self.nodes,
            "edges": self.edges,
            "evidenceSummary": graph_evidence_summary(&self.edges),
        })
    }
}

async fn build_artist_graph(
    graph: &mut Graph,
    request: &Value,
    run: Option<&Value>,
    state: &AppState,
) {
    let can_expand_catalog = run.is_none_or(can_expand_catalog_context);
    let artist_id = normalized_string(request, "artistId")
        .or_else(|| first_candidate_string(run, "artists", "artistId"))
        .or_else(|| first_candidate_string(run, "tracks", "musicBrainzArtistId"));
    let artist_name = normalized_string(request, "artist")
        .or_else(|| artist_name_for_id(run, artist_id.as_deref()))
        .or_else(|| nested_string(run, &["metadata", "artist"]))
        .unwrap_or_else(|| "Unknown artist".to_owned());
    let center_node_id = format!(
        "artist:{}",
        normalize_id(artist_id.as_deref().unwrap_or(artist_name.as_str()))
    );
    graph.add_node(
        center_node_id.clone(),
        artist_name.clone(),
        "artist",
        1.0,
        0,
        "center",
        "Seed artist for discovery topology.",
    );

    if let Some(run) = run {
        for track in graph_track_candidates(run)
            .into_iter()
            .filter(|track| same_artist_candidate(track, artist_id.as_deref(), Some(&artist_name)))
            .take(4)
        {
            let Some(recording_id) = string_field(&track, "recordingId") else {
                continue;
            };
            let title = string_field(&track, "title").unwrap_or_else(|| "Unknown track".to_owned());
            let identity = number_field(&track, "identityScore");
            let action = number_field(&track, "actionScore");
            let node_id = format!("track:{recording_id}");
            graph.add_node(
                node_id.clone(),
                title,
                "track",
                action,
                1,
                "recording",
                "Track candidate attached to the selected artist.",
            );
            graph.add_edge(
                &center_node_id,
                &node_id,
                "performed_by",
                identity,
                "Track candidate resolves to this artist.",
                "songid",
                &[("identity", identity), ("action", action)],
                &candidate_evidence(&track, "Track candidate"),
            );
        }

        for artist in graph_artist_candidates(run)
            .into_iter()
            .filter(|artist| {
                artist_id.as_deref().is_none_or(|id| {
                    string_field(artist, "artistId")
                        .is_none_or(|candidate| !candidate.eq_ignore_ascii_case(id))
                })
            })
            .take(4)
        {
            let Some(candidate_id) = string_field(&artist, "artistId") else {
                continue;
            };
            let name = string_field(&artist, "name").unwrap_or_else(|| "Unknown artist".to_owned());
            let identity = number_field(&artist, "identityScore");
            let byzantine = number_field(&artist, "byzantineScore");
            let action = number_field(&artist, "actionScore");
            let node_id = format!("artist:{candidate_id}");
            graph.add_node(
                node_id.clone(),
                name,
                "artist",
                action,
                1,
                "neighbor",
                "Nearby artist candidate surfaced in the same SongID context.",
            );
            graph.add_edge(
                &center_node_id,
                &node_id,
                "candidate_neighbor",
                byzantine,
                "Artist candidate co-occurred in the same identification neighborhood.",
                "songid",
                &[
                    ("identity", identity),
                    ("byzantine", byzantine),
                    ("action", action),
                ],
                &candidate_evidence(&artist, "SongID artist candidate"),
            );
        }
    }

    if can_expand_catalog {
        if let Some(artist_id) = artist_id.as_deref() {
            let releases = local_musicbrainz_release_graph(state, artist_id, &artist_name).await;
            for release in releases.into_iter().take(6) {
                let Some(release_id) = string_field(&release, "id") else {
                    continue;
                };
                let title =
                    string_field(&release, "title").unwrap_or_else(|| "Unknown release".to_owned());
                let node_id = format!("release-group:{release_id}");
                graph.add_node(
                    node_id.clone(),
                    title,
                    "album",
                    0.58,
                    1,
                    "release",
                    "Release group from MusicBrainz artist graph.",
                );
                graph.add_edge(
                    &center_node_id,
                    &node_id,
                    "release_group",
                    0.62,
                    "MusicBrainz release-graph expansion.",
                    "musicbrainz_release_graph",
                    &[("metadata", 0.62)],
                    &["Release-group expansion from MusicBrainz".to_owned()],
                );
            }
        }
    }

    graph.title = artist_name;
    graph.summary =
        "Artist neighborhood with SongID candidates and release-graph context.".to_owned();
}

fn build_album_graph(graph: &mut Graph, request: &Value, run: Option<&Value>) {
    let album = run.and_then(|run| {
        let requested = normalized_string(request, "releaseId");
        let albums = graph_album_candidates(run);
        requested
            .as_deref()
            .and_then(|release_id| {
                albums.iter().find(|album| {
                    string_field(album, "releaseId")
                        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(release_id))
                })
            })
            .cloned()
            .or_else(|| albums.into_iter().next())
    });
    let title = normalized_string(request, "album")
        .or_else(|| {
            album
                .as_ref()
                .and_then(|value| string_field(value, "title"))
        })
        .unwrap_or_else(|| "Unknown album".to_owned());
    let center_id = normalized_string(request, "releaseId")
        .or_else(|| {
            album
                .as_ref()
                .and_then(|value| string_field(value, "releaseId"))
        })
        .unwrap_or_else(|| normalize_id(&title));
    let center_node_id = format!("album:{center_id}");
    graph.add_node(
        center_node_id.clone(),
        title.clone(),
        "album",
        1.0,
        0,
        "center",
        "Seed release for discovery topology.",
    );

    if let Some(album) = album.as_ref() {
        add_artist_neighbor(
            graph,
            &center_node_id,
            string_field(album, "musicBrainzArtistId"),
            string_field(album, "artist"),
            number_field(album, "identityScore"),
            "Album candidate resolves to this artist.",
        );
    }

    if let Some(run) = run {
        let artist = album
            .as_ref()
            .and_then(|value| string_field(value, "artist"))
            .or_else(|| normalized_string(request, "artist"));
        for track in graph_track_candidates(run)
            .into_iter()
            .filter(|track| {
                artist
                    .as_deref()
                    .is_some_and(|artist| string_field(track, "artist").as_deref() == Some(artist))
            })
            .take(5)
        {
            let Some(recording_id) = string_field(&track, "recordingId") else {
                continue;
            };
            let title = string_field(&track, "title").unwrap_or_else(|| "Unknown track".to_owned());
            let identity = number_field(&track, "identityScore");
            let action = number_field(&track, "actionScore");
            let node_id = format!("track:{recording_id}");
            graph.add_node(
                node_id.clone(),
                title,
                "track",
                action,
                1,
                "recording",
                "Track candidate sits near this album in the same SongID context.",
            );
            graph.add_edge(
                &center_node_id,
                &node_id,
                "album_context",
                identity,
                "Track candidate shares artist context with the selected album.",
                "songid",
                &[("identity", identity), ("action", action)],
                &candidate_evidence(&track, "Track candidate"),
            );
        }
    }

    graph.title = title;
    graph.summary = "Album neighborhood with artist and nearby track context.".to_owned();
}

fn build_track_graph(graph: &mut Graph, request: &Value, run: Option<&Value>) {
    let track = run.and_then(|run| {
        let requested = normalized_string(request, "recordingId");
        let tracks = graph_track_candidates(run);
        requested
            .as_deref()
            .and_then(|recording_id| {
                tracks.iter().find(|track| {
                    string_field(track, "recordingId")
                        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(recording_id))
                })
            })
            .cloned()
            .or_else(|| tracks.into_iter().next())
    });
    let title = normalized_string(request, "title")
        .or_else(|| {
            track
                .as_ref()
                .and_then(|value| string_field(value, "title"))
        })
        .unwrap_or_else(|| "Unknown track".to_owned());
    let center_id = normalized_string(request, "recordingId")
        .or_else(|| {
            track
                .as_ref()
                .and_then(|value| string_field(value, "recordingId"))
        })
        .unwrap_or_else(|| normalize_id(&title));
    let center_node_id = format!("track:{center_id}");
    graph.add_node(
        center_node_id.clone(),
        title.clone(),
        "track",
        1.0,
        0,
        "center",
        "Seed recording for discovery topology.",
    );

    if let Some(track) = track.as_ref() {
        add_artist_neighbor(
            graph,
            &center_node_id,
            string_field(track, "musicBrainzArtistId"),
            string_field(track, "artist"),
            number_field(track, "identityScore"),
            "Track candidate resolves to this artist.",
        );
    }

    if let Some(run) = run {
        let selected_id = track
            .as_ref()
            .and_then(|value| string_field(value, "recordingId"));
        for sibling in graph_track_candidates(run)
            .into_iter()
            .filter(|candidate| string_field(candidate, "recordingId") != selected_id)
            .take(5)
        {
            let Some(recording_id) = string_field(&sibling, "recordingId") else {
                continue;
            };
            let artist =
                string_field(&sibling, "artist").unwrap_or_else(|| "Unknown artist".to_owned());
            let sibling_title =
                string_field(&sibling, "title").unwrap_or_else(|| "Unknown track".to_owned());
            let node_id = format!("track:{recording_id}");
            let identity = number_field(&sibling, "identityScore");
            let byzantine = number_field(&sibling, "byzantineScore");
            let action = number_field(&sibling, "actionScore");
            graph.add_node(
                node_id.clone(),
                format!("{artist} - {sibling_title}"),
                "track",
                action,
                1,
                "candidate",
                "Alternative or adjacent track candidate from the same SongID run.",
            );
            graph.add_edge(
                &center_node_id,
                &node_id,
                "candidate_neighbor",
                byzantine,
                "Track candidate co-occurred in the same identification neighborhood.",
                "songid",
                &[
                    ("identity", identity),
                    ("byzantine", byzantine),
                    ("action", action),
                ],
                &candidate_evidence(&sibling, "Track candidate"),
            );
        }

        for segment in graph_segments(run).into_iter().take(4) {
            let Some(segment_id) = string_field(&segment, "segmentId") else {
                continue;
            };
            let label = string_field(&segment, "label").unwrap_or_else(|| segment_id.clone());
            let confidence = number_field(&segment, "confidence");
            let reason = string_field(&segment, "decompositionLabel")
                .unwrap_or_else(|| "SongID segment".to_owned());
            let node_id = format!("segment:{segment_id}");
            graph.add_node(
                node_id.clone(),
                label,
                "segment",
                confidence,
                1,
                "segment",
                &reason,
            );
            graph.add_edge(
                &center_node_id,
                &node_id,
                "segment_context",
                confidence,
                "Timestamp/chapter decomposition linked this section to the seed neighborhood.",
                "songid_segment",
                &[("confidence", confidence)],
                std::slice::from_ref(&reason),
            );
        }
    }

    graph.title = title;
    graph.summary =
        "Track neighborhood with artist, alternatives, and segment ambiguity context.".to_owned();
}

fn build_run_graph(
    graph: &mut Graph,
    request: &Value,
    run: Option<&Value>,
    requested_run_id: Option<&str>,
) {
    let Some(run) = run else {
        let fallback_label = normalized_string(request, "title")
            .or_else(|| normalized_string(request, "artist"))
            .or_else(|| normalized_string(request, "album"))
            .unwrap_or_else(|| "SongID seed".to_owned());
        let seed_node_id = format!("seed:{}", normalize_id(&fallback_label));
        graph.add_node(
            seed_node_id.clone(),
            fallback_label.clone(),
            "seed",
            1.0,
            0,
            "center",
            "Fallback Discovery Graph seed.",
        );
        add_fallback_context(graph, &seed_node_id, request);
        graph.title = fallback_label;
        graph.summary = if requested_run_id.is_some() {
            "Fallback discovery seed because the requested SongID run was not found.".to_owned()
        } else {
            "Fallback discovery seed without a SongID run context.".to_owned()
        };
        return;
    };

    let run_id = string_field(run, "id")
        .or_else(|| requested_run_id.map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned());
    let seed_label = string_field(run, "query")
        .or_else(|| nested_string(Some(run), &["metadata", "title"]))
        .unwrap_or_else(|| "SongID seed".to_owned());
    let seed_node_id = format!("songid:{run_id}");
    graph.add_node(
        seed_node_id.clone(),
        seed_label.clone(),
        "songid_run",
        1.0,
        0,
        "center",
        "SongID run seed. Near nodes come from identity, ambiguity, and evidence context.",
    );

    for track in graph_track_candidates(run).into_iter().take(4) {
        let Some(recording_id) = string_field(&track, "recordingId") else {
            continue;
        };
        let artist = string_field(&track, "artist").unwrap_or_else(|| "Unknown artist".to_owned());
        let title = string_field(&track, "title").unwrap_or_else(|| "Unknown track".to_owned());
        let identity = number_field(&track, "identityScore");
        let byzantine = number_field(&track, "byzantineScore");
        let action = number_field(&track, "actionScore");
        let exact = bool_field(&track, "isExact");
        let node_id = format!("track:{recording_id}");
        graph.add_node(
            node_id.clone(),
            format!("{artist} - {title}"),
            "track",
            action,
            1,
            "candidate",
            "Ranked SongID track candidate.",
        );
        graph.add_edge(
            &seed_node_id,
            &node_id,
            "identity_candidate",
            identity,
            if exact {
                "Exact track identity."
            } else {
                "Plausible track identity."
            },
            "songid",
            &[
                ("identity", identity),
                ("byzantine", byzantine),
                ("action", action),
            ],
            &candidate_evidence(&track, "Track candidate"),
        );
    }

    for album in graph_album_candidates(run).into_iter().take(3) {
        let Some(release_id) = string_field(&album, "releaseId") else {
            continue;
        };
        let artist = string_field(&album, "artist").unwrap_or_else(|| "Unknown artist".to_owned());
        let title = string_field(&album, "title").unwrap_or_else(|| "Unknown album".to_owned());
        let identity = number_field(&album, "identityScore");
        let byzantine = number_field(&album, "byzantineScore");
        let action = number_field(&album, "actionScore");
        let node_id = format!("album:{release_id}");
        graph.add_node(
            node_id.clone(),
            format!("{artist} - {title}"),
            "album",
            action,
            1,
            "release",
            "Album candidate near the current SongID result.",
        );
        graph.add_edge(
            &seed_node_id,
            &node_id,
            "album_context",
            identity,
            "Album candidate derived from SongID identity resolution.",
            "songid",
            &[
                ("identity", identity),
                ("byzantine", byzantine),
                ("action", action),
            ],
            &candidate_evidence(&album, "Album candidate"),
        );
    }

    for artist in graph_artist_candidates(run).into_iter().take(3) {
        let Some(artist_id) = string_field(&artist, "artistId") else {
            continue;
        };
        let name = string_field(&artist, "name").unwrap_or_else(|| "Unknown artist".to_owned());
        let identity = number_field(&artist, "identityScore");
        let byzantine = number_field(&artist, "byzantineScore");
        let action = number_field(&artist, "actionScore");
        let node_id = format!("artist:{artist_id}");
        graph.add_node(
            node_id.clone(),
            name,
            "artist",
            action,
            1,
            "neighbor",
            "Artist candidate near the current SongID result.",
        );
        graph.add_edge(
            &seed_node_id,
            &node_id,
            "artist_context",
            identity,
            "Artist context derived from SongID resolution.",
            "songid",
            &[
                ("identity", identity),
                ("byzantine", byzantine),
                ("action", action),
            ],
            &candidate_evidence(&artist, "SongID artist candidate"),
        );
    }

    for segment in graph_segments(run).into_iter().take(4) {
        let Some(segment_id) = string_field(&segment, "segmentId") else {
            continue;
        };
        let label = string_field(&segment, "label").unwrap_or_else(|| segment_id.clone());
        let confidence = number_field(&segment, "confidence");
        let reason = string_field(&segment, "decompositionLabel")
            .unwrap_or_else(|| "SongID segment".to_owned());
        let node_id = format!("segment:{segment_id}");
        graph.add_node(
            node_id.clone(),
            label,
            "segment",
            confidence,
            1,
            "segment",
            &reason,
        );
        graph.add_edge(
            &seed_node_id,
            &node_id,
            "segment_context",
            confidence,
            "Mix or timestamp decomposition branch.",
            "songid_segment",
            &[("confidence", confidence)],
            std::slice::from_ref(&reason),
        );

        for candidate in segment_candidates(&segment).into_iter().take(2) {
            let Some(recording_id) = string_field(&candidate, "recordingId") else {
                continue;
            };
            let artist =
                string_field(&candidate, "artist").unwrap_or_else(|| "Unknown artist".to_owned());
            let title =
                string_field(&candidate, "title").unwrap_or_else(|| "Unknown track".to_owned());
            let identity = number_field(&candidate, "identityScore");
            let byzantine = number_field(&candidate, "byzantineScore");
            let action = number_field(&candidate, "actionScore");
            let candidate_node_id = format!("track:{recording_id}");
            graph.add_node(
                candidate_node_id.clone(),
                format!("{artist} - {title}"),
                "track",
                action,
                2,
                "candidate",
                "Candidate attached to a decomposed SongID segment.",
            );
            graph.add_edge(
                &node_id,
                &candidate_node_id,
                "segment_candidate",
                identity,
                "Segment-level candidate search path.",
                "songid_segment",
                &[
                    ("identity", identity),
                    ("byzantine", byzantine),
                    ("action", action),
                ],
                &candidate_evidence(&candidate, "Segment candidate"),
            );
        }
    }

    for mix in graph_mix_groups(run).into_iter().take(3) {
        let Some(mix_id) = string_field(&mix, "mixId") else {
            continue;
        };
        let label = string_field(&mix, "label").unwrap_or_else(|| mix_id.clone());
        let confidence = number_field(&mix, "confidence");
        let identity = number_field(&mix, "identityScore");
        let action = number_field(&mix, "actionScore");
        let node_id = format!("mix:{mix_id}");
        graph.add_node(
            node_id.clone(),
            label,
            "mix",
            action,
            1,
            "segment",
            "Mix cluster aggregated from contiguous segments.",
        );
        let segment_count = array_field(&mix, "segmentIds").len() as f64;
        graph.add_edge(
            &seed_node_id,
            &node_id,
            "mix_cluster",
            confidence,
            "Detected mix cluster from segment decomposition.",
            "songid_mix",
            &[
                ("confidence", confidence),
                ("identity", identity),
                ("action", action),
            ],
            &[format!("Mix cluster covering {segment_count:.0} segments")],
        );
        for segment_id in array_field(&mix, "segmentIds")
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
        {
            let segment_node_id = format!("segment:{segment_id}");
            graph.add_edge(
                &node_id,
                &segment_node_id,
                "mix_segment",
                confidence,
                "Segment belongs to this mix cluster.",
                "songid_mix",
                &[("confidence", confidence)],
                &[format!("Segment {segment_id} is part of mix cluster")],
            );
        }
    }

    graph.title = seed_label;
    graph.summary =
        "SongID neighborhood with candidates, ambiguity branches, and segment decomposition."
            .to_owned();
}

fn add_artist_neighbor(
    graph: &mut Graph,
    source_node_id: &str,
    artist_id: Option<String>,
    artist_name: Option<String>,
    weight: f64,
    reason: &str,
) {
    let label = artist_name.unwrap_or_else(|| "Unknown artist".to_owned());
    let id = artist_id.unwrap_or_else(|| normalize_id(&label));
    let node_id = format!("artist:{id}");
    graph.add_node(
        node_id.clone(),
        label.clone(),
        "artist",
        weight,
        1,
        "neighbor",
        reason,
    );
    graph.add_edge(
        source_node_id,
        &node_id,
        "performed_by",
        weight,
        reason,
        "songid",
        &[("identity", weight)],
        &[reason.to_owned()],
    );
}

fn add_comparison_overlay(graph: &mut Graph, request: &Value, run: Option<&Value>) {
    let Some(compare_node_id) = normalized_string(request, "compareNodeId") else {
        return;
    };
    let Some((node_type, raw_id)) = compare_node_id.split_once(':') else {
        return;
    };
    let label =
        normalized_string(request, "compareLabel").unwrap_or_else(|| compare_node_id.clone());
    if graph.nodes.iter().any(|node| {
        node["nodeId"]
            .as_str()
            .is_some_and(|id| id.eq_ignore_ascii_case(&compare_node_id))
    }) {
        return;
    }
    graph.add_node(
        compare_node_id.clone(),
        label,
        node_type,
        0.84,
        1,
        "compare",
        "Pinned comparison node.",
    );
    let mut components = vec![(
        "weight_delta",
        comparison_weight_delta(graph, &compare_node_id),
    )];
    if let Some(run) = run {
        components.push(("shared_songid_context", 1.0));
        components.push(("segment_count", array_field(run, "segments").len() as f64));
    }
    graph.add_edge(
        &graph.seed_node_id.clone(),
        &compare_node_id,
        "comparison",
        0.72,
        "User-pinned comparison between the current seed and another graph node.",
        "ui_compare",
        &components,
        &["Pinned from graph UI".to_owned()],
    );

    if run.is_some() && node_type.eq_ignore_ascii_case("artist") {
        for track in graph_track_candidates(run.expect("run checked"))
            .into_iter()
            .filter(|track| string_field(track, "musicBrainzArtistId").as_deref() == Some(raw_id))
            .take(3)
        {
            let Some(recording_id) = string_field(&track, "recordingId") else {
                continue;
            };
            let node_id = format!("track:{recording_id}");
            let artist =
                string_field(&track, "artist").unwrap_or_else(|| "Unknown artist".to_owned());
            let title = string_field(&track, "title").unwrap_or_else(|| "Unknown track".to_owned());
            let identity = number_field(&track, "identityScore");
            let action = number_field(&track, "actionScore");
            graph.add_node(
                node_id.clone(),
                format!("{artist} - {title}"),
                "track",
                action,
                2,
                "candidate",
                "Track attached to the pinned comparison artist.",
            );
            graph.add_edge(
                &compare_node_id,
                &node_id,
                "comparison_context",
                identity,
                "Track branch attached to the pinned comparison artist.",
                "songid",
                &[("identity", identity), ("action", action)],
                &candidate_evidence(&track, "Track candidate"),
            );
        }
    }
}

fn add_fallback_context(graph: &mut Graph, seed_node_id: &str, request: &Value) {
    if let (Some(artist), Some(id)) = (
        normalized_string(request, "artist"),
        normalized_string(request, "artistId").or_else(|| normalized_string(request, "artist")),
    ) {
        add_fallback_context_edge(
            graph,
            seed_node_id,
            "artist",
            &id,
            &artist,
            "neighbor",
            0.74,
            0.68,
        );
    }
    if let (Some(album), Some(id)) = (
        normalized_string(request, "album"),
        normalized_string(request, "releaseId").or_else(|| normalized_string(request, "album")),
    ) {
        add_fallback_context_edge(
            graph,
            seed_node_id,
            "album",
            &id,
            &album,
            "release",
            0.70,
            0.64,
        );
    }
    if let Some(title) = normalized_string(request, "title") {
        let id = normalized_string(request, "recordingId").unwrap_or_else(|| normalize_id(&title));
        add_fallback_context_edge(
            graph,
            seed_node_id,
            "track",
            &id,
            &title,
            "candidate",
            0.76,
            0.72,
        );
    }
}

fn add_fallback_context_edge(
    graph: &mut Graph,
    seed_node_id: &str,
    kind: &str,
    id: &str,
    label: &str,
    accent: &str,
    node_weight: f64,
    edge_weight: f64,
) {
    let node_id = format!("{kind}:{id}");
    let display_kind = capitalize(kind);
    graph.add_node(
        node_id.clone(),
        label,
        kind,
        node_weight,
        1,
        accent,
        &format!("{display_kind} supplied directly as fallback graph context."),
    );
    graph.add_edge(
        seed_node_id,
        &node_id,
        "metadata_context",
        edge_weight,
        &format!("{display_kind} text supplied with the graph seed."),
        "fallback_request",
        &[("metadata", edge_weight)],
        &[label.to_owned()],
    );
}

async fn local_musicbrainz_release_graph(
    state: &AppState,
    artist_id: &str,
    artist_name: &str,
) -> Vec<Value> {
    let library = state.library.read().await;
    library
        .records
        .iter()
        .filter(|item| {
            item.artist.eq_ignore_ascii_case(artist_id)
                || item.artist.eq_ignore_ascii_case(artist_name)
        })
        .map(|item| {
            json!({
                "id": item.id,
                "title": item.title,
                "kind": item.kind,
            })
        })
        .collect()
}

fn graph_track_candidates(run: &Value) -> Vec<Value> {
    let mut tracks = array_field(run, "tracks");
    if tracks.is_empty() {
        tracks = array_field(run, "matches")
            .into_iter()
            .filter_map(|item| {
                let recording_id = string_field(&item, "libraryItemId")?;
                let artist = string_field(&item, "artist")?;
                let title = string_field(&item, "title")?;
                let score = number_field(&item, "score");
                Some(json!({
                    "candidateId": recording_id,
                    "recordingId": recording_id,
                    "artist": artist,
                    "title": title,
                    "identityScore": score,
                    "byzantineScore": score,
                    "actionScore": score,
                    "isExact": score >= 1.0,
                }))
            })
            .collect();
    }
    let can_expand = can_expand_catalog_context(run);
    tracks.retain(|track| {
        string_field(track, "recordingId").is_some()
            && string_field(track, "title").is_some()
            && string_field(track, "artist").is_some()
            && (can_expand
                || bool_field(&track, "isExact")
                || number_field(&track, "identityScore") >= MINIMUM_TRACK_IDENTITY_FOR_WEAK_RUN)
    });
    sort_candidates(&mut tracks);
    tracks
}

fn graph_album_candidates(run: &Value) -> Vec<Value> {
    if !can_expand_catalog_context(run) {
        return Vec::new();
    }
    let mut albums = array_field(run, "albums");
    albums.retain(|album| {
        string_field(album, "releaseId").is_some() && string_field(album, "title").is_some()
    });
    sort_candidates(&mut albums);
    albums
}

fn graph_artist_candidates(run: &Value) -> Vec<Value> {
    if !can_expand_catalog_context(run) {
        return Vec::new();
    }
    let mut artists = array_field(run, "artists");
    artists.retain(|artist| {
        string_field(artist, "artistId").is_some() && string_field(artist, "name").is_some()
    });
    sort_candidates(&mut artists);
    artists
}

fn graph_segments(run: &Value) -> Vec<Value> {
    if !can_expand_catalog_context(run) {
        return Vec::new();
    }
    let mut segments = array_field(run, "segments");
    segments.retain(|segment| {
        number_field(segment, "confidence") >= MINIMUM_SEGMENT_CONFIDENCE_FOR_GRAPH
            && !segment_candidates(segment).is_empty()
    });
    segments.sort_by(|left, right| {
        number_field(right, "confidence")
            .partial_cmp(&number_field(left, "confidence"))
            .unwrap_or(Ordering::Equal)
    });
    segments
}

fn segment_candidates(segment: &Value) -> Vec<Value> {
    let mut candidates = array_field(segment, "candidates");
    candidates.retain(|candidate| {
        string_field(candidate, "recordingId").is_some()
            && string_field(candidate, "title").is_some()
            && string_field(candidate, "artist").is_some()
            && number_field(candidate, "identityScore")
                >= MINIMUM_SEGMENT_CANDIDATE_IDENTITY_FOR_GRAPH
    });
    sort_candidates(&mut candidates);
    candidates
}

fn graph_mix_groups(run: &Value) -> Vec<Value> {
    if !can_expand_catalog_context(run) {
        return Vec::new();
    }
    let segment_ids = graph_segments(run)
        .into_iter()
        .filter_map(|segment| string_field(&segment, "segmentId"))
        .collect::<Vec<_>>();
    let mut groups = array_field(run, "mixGroups");
    groups.retain(|mix| {
        number_field(mix, "identityScore") >= MINIMUM_MIX_IDENTITY_FOR_GRAPH
            && array_field(mix, "segmentIds").into_iter().any(|id| {
                id.as_str().is_some_and(|id| {
                    segment_ids
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(id))
                })
            })
    });
    sort_candidates(&mut groups);
    groups
}

fn can_expand_catalog_context(run: &Value) -> bool {
    if array_field(run, "tracks").into_iter().any(|track| {
        bool_field(&track, "isExact")
            || number_field(&track, "identityScore") >= MINIMUM_TRACK_IDENTITY_FOR_CATALOG_EXPANSION
    }) {
        return true;
    }
    let assessment = run
        .get("identityAssessment")
        .filter(|value| value.is_object())
        .or_else(|| run.get("assessment"));
    let verdict = assessment
        .and_then(|value| string_field(value, "verdict"))
        .unwrap_or_default();
    let confidence = assessment
        .map(|value| number_field(value, "confidence"))
        .unwrap_or(0.0);
    (verdict.eq_ignore_ascii_case("recognized_cataloged_track") && confidence >= 0.65)
        || (verdict.eq_ignore_ascii_case("candidate_match_found") && confidence >= 0.75)
}

fn sort_candidates(values: &mut [Value]) {
    values.sort_by(|left, right| {
        number_field(right, "actionScore")
            .partial_cmp(&number_field(left, "actionScore"))
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                number_field(right, "identityScore")
                    .partial_cmp(&number_field(left, "identityScore"))
                    .unwrap_or(Ordering::Equal)
            })
    });
}

fn artist_name_for_id(run: Option<&Value>, artist_id: Option<&str>) -> Option<String> {
    let candidates = array_field(run?, "artists");
    if let Some(artist_id) = artist_id {
        if let Some(name) = candidates.into_iter().find_map(|artist| {
            (string_field(&artist, "artistId").is_some_and(|id| id.eq_ignore_ascii_case(artist_id)))
                .then(|| string_field(&artist, "name"))
                .flatten()
        }) {
            return Some(name);
        }
    }
    nested_string(run, &["metadata", "artist"])
}

fn first_candidate_string(run: Option<&Value>, field: &str, key: &str) -> Option<String> {
    array_field(run?, field)
        .into_iter()
        .find_map(|candidate| string_field(&candidate, key))
}

fn same_artist_candidate(
    candidate: &Value,
    artist_id: Option<&str>,
    artist_name: Option<&String>,
) -> bool {
    artist_id.is_some_and(|id| {
        string_field(candidate, "musicBrainzArtistId")
            .is_some_and(|candidate_id| candidate_id.eq_ignore_ascii_case(id))
    }) || artist_name.is_some_and(|name| {
        string_field(candidate, "artist")
            .is_some_and(|candidate_name| candidate_name.eq_ignore_ascii_case(name))
    })
}

fn candidate_evidence(candidate: &Value, prefix: &str) -> Vec<String> {
    let id = string_field(candidate, "candidateId")
        .or_else(|| string_field(candidate, "recordingId"))
        .unwrap_or_else(|| prefix.to_owned());
    vec![format!("{prefix} {id}")]
}

fn comparison_weight_delta(graph: &Graph, compare_node_id: &str) -> f64 {
    let seed_weight = graph
        .nodes
        .iter()
        .find(|node| node["nodeId"].as_str() == Some(graph.seed_node_id.as_str()))
        .map(|node| number_field(node, "weight"))
        .unwrap_or(0.0);
    let compare_weight = graph
        .nodes
        .iter()
        .find(|node| node["nodeId"].as_str() == Some(compare_node_id))
        .map(|node| number_field(node, "weight"))
        .unwrap_or(0.0);
    (seed_weight - compare_weight).abs()
}

fn edge_evidence_lanes(
    provenance: &str,
    score_components: &[(&str, f64)],
    evidence: &[String],
) -> Value {
    let mut lanes = Vec::new();
    for (lane, score) in score_components {
        if *score <= 0.0 {
            continue;
        }
        let label = format_lane_label(lane);
        lanes.push(json!({
            "lane": lane,
            "label": label,
            "score": clamp_score(*score),
            "count": 1,
            "summary": format!("{label} contributed {:.0}%.", clamp_score(*score) * 100.0),
        }));
    }
    let evidence_count = evidence
        .iter()
        .filter(|item| !item.trim().is_empty())
        .count();
    if evidence_count > 0 {
        lanes.push(json!({
            "lane": "evidence",
            "label": "Evidence",
            "score": (evidence_count as f64 / 4.0).min(1.0),
            "count": evidence_count,
            "summary": format!("{} evidence item{} attached.", evidence_count, if evidence_count == 1 { "" } else { "s" }),
        }));
    }
    if !provenance.trim().is_empty() {
        lanes.push(json!({
            "lane": "provenance",
            "label": "Provenance",
            "score": 1.0,
            "count": 1,
            "summary": format!("Source: {provenance}."),
        }));
    }
    Value::Array(lanes)
}

fn graph_evidence_summary(edges: &[Value]) -> Value {
    #[derive(Default)]
    struct Summary {
        lane: String,
        label: String,
        score_total: f64,
        count: usize,
        observations: usize,
        first_seen: usize,
    }

    let mut summaries: Vec<Summary> = Vec::new();
    for edge in edges {
        for lane in array_field(edge, "evidenceLanes") {
            let Some(lane_name) = string_field(&lane, "lane") else {
                continue;
            };
            let key = lane_name.to_ascii_lowercase();
            let index = summaries
                .iter()
                .position(|summary| summary.lane.to_ascii_lowercase() == key)
                .unwrap_or_else(|| {
                    let index = summaries.len();
                    summaries.push(Summary {
                        lane: lane_name.clone(),
                        label: string_field(&lane, "label")
                            .unwrap_or_else(|| format_lane_label(&lane_name)),
                        first_seen: index,
                        ..Summary::default()
                    });
                    index
                });
            let summary = &mut summaries[index];
            summary.score_total += number_field(&lane, "score");
            summary.count = summary
                .count
                .saturating_add(number_field(&lane, "count").max(1.0) as usize);
            summary.observations = summary.observations.saturating_add(1);
        }
    }
    summaries.sort_by(|left, right| {
        let left_score = if left.observations == 0 {
            0.0
        } else {
            left.score_total / left.observations as f64
        };
        let right_score = if right.observations == 0 {
            0.0
        } else {
            right.score_total / right.observations as f64
        };
        right_score
            .partial_cmp(&left_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                left.label
                    .to_ascii_lowercase()
                    .cmp(&right.label.to_ascii_lowercase())
            })
            .then_with(|| left.first_seen.cmp(&right.first_seen))
    });
    Value::Array(
        summaries
            .into_iter()
            .map(|summary| {
                let observations = summary.observations;
                json!({
                    "lane": summary.lane,
                    "label": summary.label,
                    "score": if observations == 0 { 0.0 } else { round_three(summary.score_total / observations as f64) },
                    "count": summary.count,
                    "summary": format!("{} appears on {} graph edge{}.", summary.label, observations, if observations == 1 { "" } else { "s" }),
                })
            })
            .collect(),
    )
}

fn normalized_string(value: &Value, field: &str) -> Option<String> {
    string_field(value, field).filter(|value| !value.trim().is_empty())
}

fn nested_string(value: Option<&Value>, path: &[&str]) -> Option<String> {
    let value = value?.pointer(&format!("/{}", path.join("/")))?;
    string_field(value, "")
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    let value = if field.is_empty() {
        value
    } else {
        value.get(field)?
    };
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn number_field(value: &Value, field: &str) -> f64 {
    value.get(field).and_then(Value::as_f64).unwrap_or(0.0)
}

fn bool_field(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_bool).unwrap_or(false)
}

fn array_field(value: &Value, field: &str) -> Vec<Value> {
    value
        .get(field)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn clamp_weight(value: f64) -> f64 {
    value.clamp(0.2, 1.0)
}

fn clamp_score(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn normalize_id(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .split([' ', '/', '\\', ':', '.', ',', '|', '(', ')', '[', ']'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    chars
        .next()
        .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
        .unwrap_or_default()
}

fn format_lane_label(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

fn round_three(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}
