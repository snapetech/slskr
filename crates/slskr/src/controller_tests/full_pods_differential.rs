//! Controller full pods differential ownership.

use super::*;

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_podcore_content_metadata_requires_a_real_content_id(
) {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));

    let missing =
        crate::route_http_request("GET", "/api/v0/podcore/content/metadata", None, "", &state)
            .await
            .unwrap();
    assert_eq!(missing.status, "400 Bad Request");

    let malformed = crate::route_http_request(
        "GET",
        "/api/v0/podcore/content/metadata?contentId=not-a-content-id",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(malformed.status, "404 Not Found");

    let valid_response = crate::route_http_request(
        "GET",
        "/api/v0/podcore/content/metadata?contentId=content:music:recording:route-audit",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(valid_response.status, "200 OK", "{}", valid_response.body);
    assert_eq!(valid_response.content_type, "application/json");
    let valid = serde_json::from_str::<serde_json::Value>(&valid_response.body).unwrap();
    assert_eq!(valid["contentId"], "content:music:recording:route-audit");
    assert_eq!(valid["title"], "recording: route-audit");
    assert_eq!(valid["artist"], "Unknown");
    assert_eq!(valid["type"], "recording");
    assert_eq!(valid["domain"], "music");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_content_metadata_nominal.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/content/metadata",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/content/metadata",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_podcore_content_metadata_uses_musicbrainz_recording_release_and_artist_shapes(
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind MusicBrainz metadata fixture");
    let address = listener
        .local_addr()
        .expect("MusicBrainz metadata fixture address");
    let server = tokio::spawn(async move {
        let recording_request = serve_json_fixture(
            &listener,
            serde_json::json!({
                "id": "recording-metadata-audit",
                "title": "Metadata Track",
                "length": 187000,
                "artist-credit": [{
                    "name": "Metadata Artist",
                    "artist": {"id": "artist-metadata-audit"}
                }]
            }),
        )
        .await;
        assert!(
            recording_request.starts_with(
                "GET /recording/recording-metadata-audit?fmt=json&inc=artists+isrcs HTTP/1.1"
            ),
            "{recording_request}"
        );

        let release_request = serve_json_fixture(
            &listener,
            serde_json::json!({
                "id": "release-metadata-audit",
                "title": "Metadata Album",
                "date": "2024-05",
                "artist-credit": [{
                    "name": "Metadata Artist",
                    "artist": {"id": "artist-metadata-audit"}
                }],
                "label-info": [{"label": {"name": "Audit Label"}}],
                "media": [{
                    "tracks": [
                        {"position": "1", "recording": {"id": "track-1"}},
                        {"position": "2", "recording": {"id": "track-2"}}
                    ]
                }]
            }),
        )
        .await;
        assert!(
            release_request.starts_with(
                "GET /release/release-metadata-audit?fmt=json&inc=recordings+artists+labels+discids+isrcs HTTP/1.1"
            ),
            "{release_request}"
        );

        let artist_request = serve_json_fixture(
            &listener,
            serde_json::json!({
                "recordings": [{
                    "id": "artist-track",
                    "title": "Artist Track",
                    "artist-credit": [{
                        "name": "Metadata Artist",
                        "artist": {"id": "artist-metadata-audit"}
                    }]
                }]
            }),
        )
        .await;
        assert!(
            artist_request.starts_with(
                "GET /recording?query=artist-metadata-audit&fmt=json&limit=10 HTTP/1.1"
            ),
            "{artist_request}"
        );
    });

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    {
        let mut settings = state.integration_settings.write().await;
        settings.musicbrainz.base_url = format!("http://{address}");
        settings.musicbrainz.retry_attempts = 1;
    }

    let track = crate::route_http_request(
        "GET",
        "/api/v0/podcore/content/metadata?contentId=content:audio:track:recording-metadata-audit",
        None,
        "",
        &state,
    )
    .await
    .expect("track metadata response");
    assert_eq!(track.status, "200 OK", "{}", track.body);
    assert_eq!(track.content_type, "application/json");
    let track = serde_json::from_str::<serde_json::Value>(&track.body).unwrap();
    assert_eq!(
        track["contentId"],
        "content:audio:track:recording-metadata-audit"
    );
    assert_eq!(track["title"], "Metadata Track");
    assert_eq!(track["artist"], "Metadata Artist");
    assert_eq!(
        track["additionalInfo"]["musicbrainz_id"],
        "recording-metadata-audit"
    );
    assert_eq!(track["additionalInfo"]["duration_ms"], "187000");
    assert_eq!(track["additionalInfo"]["album"], "Unknown");
    assert_eq!(track["additionalInfo"]["position"], "0");

    let album = crate::route_http_request(
        "GET",
        "/api/v0/podcore/content/metadata?contentId=content:audio:album:release-metadata-audit",
        None,
        "",
        &state,
    )
    .await
    .expect("album metadata response");
    assert_eq!(album.status, "200 OK", "{}", album.body);
    assert_eq!(album.content_type, "application/json");
    let album = serde_json::from_str::<serde_json::Value>(&album.body).unwrap();
    assert_eq!(album["title"], "Metadata Album");
    assert_eq!(album["artist"], "Metadata Artist");
    assert_eq!(album["additionalInfo"]["release_date"], "2024-05-01");
    assert_eq!(album["additionalInfo"]["track_count"], "2");
    assert_eq!(album["additionalInfo"]["label"], "Audit Label");

    let artist = crate::route_http_request(
        "GET",
        "/api/v0/podcore/content/metadata?contentId=content:audio:artist:artist-metadata-audit",
        None,
        "",
        &state,
    )
    .await
    .expect("artist metadata response");
    assert_eq!(artist.status, "200 OK", "{}", artist.body);
    assert_eq!(artist.content_type, "application/json");
    let artist = serde_json::from_str::<serde_json::Value>(&artist.body).unwrap();
    assert_eq!(artist["title"], "Metadata Artist");
    assert_eq!(artist["artist"], "Metadata Artist");
    assert_eq!(
        artist["additionalInfo"]["musicbrainz_artist_id"],
        "artist-metadata-audit"
    );
    server.await.expect("MusicBrainz metadata fixture task");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_content_metadata_populated.json"),
        serde_json::to_string_pretty(&[serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/podcore/content/metadata",
            "case": "populated-dynamic-state",
            "pass": true,
        })])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_podcore_content_search_returns_musicbrainz_recording_results(
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind MusicBrainz content search fixture");
    let address = listener
        .local_addr()
        .expect("MusicBrainz content search fixture address");
    let server = tokio::spawn(async move {
        let request = serve_json_fixture(
            &listener,
            serde_json::json!({
                "recordings": [{
                    "id": "recording-search-audit",
                    "title": "Search Audit Track",
                    "artist-credit": [{
                        "name": "Audit Artist",
                        "artist": {"id": "artist-search-audit"}
                    }]
                }]
            }),
        )
        .await;
        assert!(
            request.starts_with("GET /recording?query=Search+Audit&fmt=json&limit=1 HTTP/1.1"),
            "{request}"
        );
        assert!(
            request.to_ascii_lowercase().contains("accept-language: en"),
            "{request}"
        );
    });
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    {
        let mut settings = state.integration_settings.write().await;
        settings.musicbrainz.base_url = format!("http://{address}");
        settings.musicbrainz.retry_attempts = 1;
    }

    let missing_query =
        crate::route_http_request("GET", "/api/v0/podcore/content/search", None, "", &state)
            .await
            .unwrap();
    assert_eq!(missing_query.status, "400 Bad Request");

    // Only the "audio" domain is supported (matching the oracle's real
    // MusicBrainz-backed search, which only covers audio) -- any other
    // requested domain must come back empty, not error or ignore the
    // filter.
    let wrong_domain = crate::route_http_request(
        "GET",
        "/api/v0/podcore/content/search?query=Search%20Audit&domain=video",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(wrong_domain.status, "200 OK");
    assert_eq!(wrong_domain.content_type, "application/json");
    assert_eq!(wrong_domain.body, "[]");

    let found = crate::route_http_request(
        "GET",
        "/api/v0/podcore/content/search?query=Search%20Audit&domain=audio&limit=1",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(found.status, "200 OK", "{}", found.body);
    assert_eq!(found.content_type, "application/json");
    let found_json = serde_json::from_str::<serde_json::Value>(&found.body).unwrap();
    let results = found_json
        .as_array()
        .expect("flat array, not a wrapper object");
    assert_eq!(results.len(), 1, "{found_json}");
    assert!(
        results[0]["contentId"]
            .as_str()
            .unwrap()
            .starts_with("content:audio:track:"),
        "{found_json}"
    );
    assert_eq!(
        results[0]["contentId"],
        "content:audio:track:recording-search-audit"
    );
    assert_eq!(results[0]["title"], "Search Audit Track");
    assert_eq!(results[0]["subtitle"], "Audit Artist");
    assert_eq!(results[0]["domain"], "audio");
    assert_eq!(results[0]["type"], "track");
    assert_eq!(
        results[0]["metadata"]["musicbrainz_recording_id"],
        "recording-search-audit"
    );
    assert_eq!(
        results[0]["metadata"]["musicbrainz_artist_id"],
        "artist-search-audit"
    );
    server
        .await
        .expect("MusicBrainz content search fixture task");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_content_search.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/content/search",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/content/search",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/content/search",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_podcore_dht_stats_reflect_real_publications_not_a_pod_count_proxy(
) {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": "dht-audit-pod-1",
                "name": "DHT Audit One",
                "visibility": "public",
                "focusContentId": "content:audio:artist:some-artist",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod one");
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": "dht-audit-pod-2",
                "name": "DHT Audit Two",
                "visibility": "private",
                "focusContentId": "content:video:movie:some-movie",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod two");

    // Two real pods exist but neither has been DHT-published yet --
    // pod_count and dht-publication-count are different things.
    let baseline = crate::route_http_request("GET", "/api/v0/podcore/dht/stats", None, "", &state)
        .await
        .expect("baseline dht stats");
    assert_eq!(baseline.status, "200 OK", "{}", baseline.body);
    assert_eq!(baseline.content_type, "application/json");
    let baseline_json = serde_json::from_str::<serde_json::Value>(&baseline.body).unwrap();
    assert_eq!(baseline_json["activePublications"], 0, "{baseline_json}");
    assert_eq!(baseline_json["totalPublished"], 0, "{baseline_json}");
    let malformed_stats = crate::route_http_request(
        "GET",
        "/api/v0/podcore/dht/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed dht stats query");
    let malformed_stats_json =
        serde_json::from_str::<serde_json::Value>(&malformed_stats.body).unwrap();
    assert_eq!(malformed_stats.status, "200 OK", "{}", malformed_stats.body);
    assert_eq!(malformed_stats_json["activePublications"], 0);
    assert_eq!(malformed_stats_json["totalPublished"], 0);

    for path in [
        "/api/v0/podcore/dht/publish/dht-audit-pod-1",
        "/api/v0/podcore/dht/publish/dht-audit-pod-2",
    ] {
        let published = crate::route_http_request("POST", path, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(published.status, "200 OK", "{path}");
    }
    // Republishing the same pod must still add to the real all-time
    // counter without inflating the currently-active count.
    let republished = crate::route_http_request(
        "POST",
        "/api/v0/podcore/dht/update/dht-audit-pod-1",
        None,
        "{}",
        &state,
    )
    .await
    .expect("republish pod one");
    assert_eq!(republished.status, "200 OK");

    let malformed_refresh =
        crate::route_http_request("POST", "/api/v0/podcore/dht/refresh/", None, "", &state)
            .await
            .expect("reject blank dht refresh pod id");
    assert_eq!(
        malformed_refresh.status, "400 Bad Request",
        "{}",
        malformed_refresh.body
    );

    let refresh = crate::route_http_request(
        "POST",
        "/api/v0/podcore/dht/refresh/dht-audit-pod-1",
        None,
        "",
        &state,
    )
    .await
    .expect("refresh pod one");
    assert_eq!(refresh.status, "200 OK", "{}", refresh.body);
    assert_eq!(refresh.content_type, "application/json");
    let refresh_json = serde_json::from_str::<serde_json::Value>(&refresh.body).unwrap();
    assert_eq!(refresh_json["success"], true, "{refresh_json}");
    assert_eq!(refresh_json["podId"], "dht-audit-pod-1");
    assert_eq!(refresh_json["wasRepublished"], false);
    assert!(refresh_json["nextRefresh"].is_string());
    assert!(refresh_json.get("errorMessage").is_none());

    let stats = crate::route_http_request("GET", "/api/v0/podcore/dht/stats", None, "", &state)
        .await
        .expect("dht stats");
    assert_eq!(stats.status, "200 OK", "{}", stats.body);
    assert_eq!(stats.content_type, "application/json");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["activePublications"], 2, "{stats_json}");
    assert_eq!(stats_json["totalPublished"], 3, "{stats_json}");
    assert_eq!(stats_json["expiredPublications"], 1, "{stats_json}");
    assert_eq!(
        stats_json["publicationsByDomain"]["audio"], 2,
        "{stats_json}"
    );
    assert_eq!(
        stats_json["publicationsByDomain"]["video"], 1,
        "{stats_json}"
    );
    assert_eq!(
        stats_json["publicationsByVisibility"]["Listed"], 2,
        "{stats_json}"
    );
    assert_eq!(
        stats_json["publicationsByVisibility"]["Private"], 1,
        "{stats_json}"
    );
    assert!(
        stats_json["lastPublishOperation"].is_string(),
        "{stats_json}"
    );

    let unpublished = crate::route_http_request(
        "DELETE",
        "/api/v0/podcore/dht/unpublish/dht-audit-pod-2",
        None,
        "",
        &state,
    )
    .await
    .expect("unpublish pod two");
    assert_eq!(unpublished.status, "200 OK");
    let malformed_unpublish =
        crate::route_http_request("DELETE", "/api/v0/podcore/dht/unpublish/", None, "", &state)
            .await
            .expect("reject blank dht unpublish pod id");
    assert_eq!(
        malformed_unpublish.status, "400 Bad Request",
        "{}",
        malformed_unpublish.body
    );

    let after_unpublish =
        crate::route_http_request("GET", "/api/v0/podcore/dht/stats", None, "", &state)
            .await
            .expect("dht stats after unpublish");
    assert_eq!(after_unpublish.status, "200 OK", "{}", after_unpublish.body);
    assert_eq!(after_unpublish.content_type, "application/json");
    let after_unpublish_json =
        serde_json::from_str::<serde_json::Value>(&after_unpublish.body).unwrap();
    assert_eq!(
        after_unpublish_json["activePublications"], 1,
        "{after_unpublish_json}"
    );
    assert_eq!(
        after_unpublish_json["totalPublished"], 3,
        "{after_unpublish_json}"
    );
    assert_eq!(
        after_unpublish_json["expiredPublications"], 2,
        "{after_unpublish_json}"
    );
    assert!(
        after_unpublish_json["publicationsByDomain"]["video"] == 1,
        "{after_unpublish_json}"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_dht_stats.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/stats",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/stats",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/stats",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/podcore/dht/refresh/{*podId}",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/podcore/dht/refresh/{*podId}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "DELETE",
                "route": "/api/v0/podcore/dht/unpublish/{*podId}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_podcore_dht_metadata_reads_and_verifies_the_published_record(
) {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let pod_id = "metadata-audit-pod";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Published metadata",
                "visibility": "Listed",
                "isPublic": true,
            }))
            .expect("deserialize metadata pod fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create metadata pod");
    let published = crate::route_http_request(
        "POST",
        "/api/v0/podcore/dht/publish",
        None,
        &serde_json::json!({
            "pod": {
                "podId": pod_id,
                "name": "Published metadata",
                "visibility": "public"
            }
        })
        .to_string(),
        &state,
    )
    .await
    .expect("publish pod metadata");
    assert_eq!(published.status, "200 OK", "{}", published.body);
    let published_json = serde_json::from_str::<serde_json::Value>(&published.body).unwrap();
    assert_eq!(published_json["podId"], pod_id);
    assert!(published_json.get("publishedPod").is_none());

    let metadata = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/dht/metadata/{pod_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("get published metadata");
    assert_eq!(metadata.status, "200 OK", "{}", metadata.body);
    assert_eq!(metadata.content_type, "application/json");
    let metadata_json = serde_json::from_str::<serde_json::Value>(&metadata.body).unwrap();
    assert_eq!(metadata_json["found"], true);
    assert_eq!(metadata_json["podId"], pod_id);
    assert_eq!(metadata_json["publishedPod"]["name"], "Published metadata");
    assert_eq!(metadata_json["isValidSignature"], true);

    let unpublished = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/dht/unpublish/{pod_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("unpublish metadata");
    assert_eq!(unpublished.status, "200 OK");
    let missing = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/dht/metadata/{pod_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing published metadata");
    assert_eq!(missing.status, "404 Not Found");
    assert_eq!(missing.content_type, "application/json");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&missing.body).unwrap(),
        serde_json::json!({"found": false, "error": "Pod not found"})
    );
    let malformed =
        crate::route_http_request("GET", "/api/v0/podcore/dht/metadata/", None, "", &state)
            .await
            .expect("reject blank published metadata pod id");
    assert_eq!(malformed.status, "400 Bad Request", "{}", malformed.body);
    assert_eq!(malformed.body, r#"{"error":"Pod ID is required"}"#);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_dht_metadata.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/metadata/{*podId}",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/metadata/{*podId}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/metadata/{*podId}",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/metadata/{*podId}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_podcore_backfill_sync_and_sync_all_report_real_local_work(
) {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));

    let empty_stats =
        crate::route_http_request("GET", "/api/v0/podcore/backfill/stats", None, "", &state)
            .await
            .expect("empty backfill stats");
    assert_eq!(empty_stats.status, "200 OK", "{}", empty_stats.body);
    let empty_stats_json = serde_json::from_str::<serde_json::Value>(&empty_stats.body).unwrap();
    assert_eq!(empty_stats_json["totalBackfillRequestsSent"], 0);
    assert_eq!(empty_stats_json["totalBackfillRequestsReceived"], 0);
    assert_eq!(empty_stats_json["totalMessagesBackfilled"], 0);
    assert_eq!(empty_stats_json["totalBackfillBytesTransferred"], 0);
    assert_eq!(empty_stats_json["averageBackfillDurationMs"], 0.0);
    assert_eq!(
        empty_stats_json["backfillRequestsByPod"],
        serde_json::json!({})
    );
    assert_eq!(
        empty_stats_json["lastBackfillOperation"],
        crate::PODCORE_MIN_DATETIME
    );
    let malformed_stats = crate::route_http_request(
        "GET",
        "/api/v0/podcore/backfill/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed backfill stats query");
    let malformed_stats_json =
        serde_json::from_str::<serde_json::Value>(&malformed_stats.body).unwrap();
    assert_eq!(malformed_stats.status, "200 OK", "{}", malformed_stats.body);
    assert_eq!(malformed_stats_json["totalBackfillRequestsSent"], 0);
    assert_eq!(malformed_stats_json["totalMessagesBackfilled"], 0);
    let pod_id = "backfill-audit-pod";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Backfill audit",
                "isPublic": true,
                "channels": [{"channelId": "general", "name": "General"}]
            }))
            .expect("deserialize backfill pod"),
            "tester".to_owned(),
        )
        .expect("create backfill pod");
    {
        let mut channels = state.pod_channels.write().await;
        channels
            .append(
                pod_id.to_owned(),
                "general".to_owned(),
                "peer-1".to_owned(),
                "old".to_owned(),
                String::new(),
                1_000,
            )
            .expect("append old message");
        channels
            .append(
                pod_id.to_owned(),
                "general".to_owned(),
                "peer-1".to_owned(),
                "new".to_owned(),
                String::new(),
                1_001,
            )
            .expect("append new message");
    }

    let sync = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/backfill/{pod_id}/sync"),
        None,
        r#"{"general":1000}"#,
        &state,
    )
    .await
    .expect("sync backfill");
    assert_eq!(sync.status, "200 OK", "{}", sync.body);
    assert_eq!(sync.content_type, "application/json");
    let sync_json = serde_json::from_str::<serde_json::Value>(&sync.body).unwrap();
    assert_eq!(sync_json["totalMessagesReceived"], 1);
    assert!(sync_json.get("messagesReceived").is_none());
    assert!(sync_json.get("bytesTransferred").is_none());

    let last_seen = crate::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/backfill/{pod_id}/general/last-seen"),
        None,
        "1",
        &state,
    )
    .await
    .expect("persist last-seen timestamp");
    assert_eq!(last_seen.status, "200 OK");
    assert!(last_seen.body.is_empty());

    let last_seen_readback = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/backfill/{pod_id}/last-seen"),
        None,
        "",
        &state,
    )
    .await
    .expect("read last-seen timestamp");
    assert_eq!(
        last_seen_readback.status, "200 OK",
        "{}",
        last_seen_readback.body
    );
    assert_eq!(last_seen_readback.content_type, "application/json");
    let last_seen_readback_json =
        serde_json::from_str::<serde_json::Value>(&last_seen_readback.body).unwrap();
    assert_eq!(last_seen_readback_json, serde_json::json!({"general": 1}));

    let sync_all = crate::route_http_request(
        "POST",
        "/api/v0/podcore/backfill/sync-all",
        None,
        "",
        &state,
    )
    .await
    .expect("sync all backfill");
    assert_eq!(sync_all.status, "200 OK");
    assert_eq!(sync_all.content_type, "application/json");
    let sync_all_json = serde_json::from_str::<serde_json::Value>(&sync_all.body).unwrap();
    assert_eq!(sync_all_json.as_array().unwrap().len(), 1);
    assert_eq!(sync_all_json[0]["totalMessagesReceived"], 2);

    let stats =
        crate::route_http_request("GET", "/api/v0/podcore/backfill/stats", None, "", &state)
            .await
            .expect("backfill stats");
    assert_eq!(stats.status, "200 OK", "{}", stats.body);
    assert_eq!(stats.content_type, "application/json");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalBackfillRequestsSent"], 2);
    assert_eq!(stats_json["totalBackfillRequestsReceived"], 0);
    assert_eq!(stats_json["totalMessagesBackfilled"], 3);
    assert_eq!(stats_json["totalBackfillBytesTransferred"], 609);
    assert_eq!(stats_json["backfillRequestsByPod"][pod_id], 2);
    assert!(stats_json["lastBackfillOperation"].is_string());

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_backfill.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/podcore/backfill/{podId}/sync",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/podcore/backfill/{podId}/sync",
                "case": "mutation-side-effects-and-readback",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/podcore/backfill/sync-all",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/backfill/{podId}/last-seen",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/backfill/{podId}/last-seen",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/backfill/stats",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/backfill/stats",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/backfill/stats",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_podcore_discovery_stats_use_registrations_and_search_activity(
) {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let empty_stats =
        crate::route_http_request("GET", "/api/v0/podcore/discovery/stats", None, "", &state)
            .await
            .expect("empty discovery stats");
    assert_eq!(empty_stats.status, "200 OK", "{}", empty_stats.body);
    assert_eq!(empty_stats.content_type, "application/json");
    let empty_stats_json = serde_json::from_str::<serde_json::Value>(&empty_stats.body).unwrap();
    assert_eq!(
        empty_stats_json["totalRegisteredPods"], 0,
        "{empty_stats_json}"
    );
    assert_eq!(
        empty_stats_json["activeDiscoveryEntries"], 0,
        "{empty_stats_json}"
    );
    assert_eq!(empty_stats_json["expiredEntries"], 0, "{empty_stats_json}");
    assert_eq!(
        empty_stats_json["registrationsByTag"],
        serde_json::json!({})
    );
    assert_eq!(empty_stats_json["searchesByType"], serde_json::json!({}));
    let malformed_stats = crate::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed discovery stats query");
    let malformed_stats_json =
        serde_json::from_str::<serde_json::Value>(&malformed_stats.body).unwrap();
    assert_eq!(malformed_stats.status, "200 OK", "{}", malformed_stats.body);
    assert_eq!(malformed_stats_json["totalRegisteredPods"], 0);
    assert_eq!(malformed_stats_json["activeDiscoveryEntries"], 0);
    let empty_search = crate::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/name/audit-pod",
        None,
        "",
        &state,
    )
    .await
    .expect("empty discovery search");
    assert_eq!(empty_search.status, "200 OK", "{}", empty_search.body);
    assert_eq!(empty_search.content_type, "application/json");
    let empty_search_json = serde_json::from_str::<serde_json::Value>(&empty_search.body).unwrap();
    assert_eq!(empty_search_json["pods"], serde_json::json!([]));
    assert_eq!(empty_search_json["searchTerm"], "audit-pod");
    assert_eq!(empty_search_json["searchType"], "name");
    assert_eq!(empty_search_json["totalFound"], 0);

    let empty_all = crate::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/all?limit=50",
        None,
        "",
        &state,
    )
    .await
    .expect("empty all-pods discovery search");
    let empty_all_json = serde_json::from_str::<serde_json::Value>(&empty_all.body).unwrap();
    assert_eq!(empty_all.status, "200 OK", "{}", empty_all.body);
    assert_eq!(empty_all.content_type, "application/json");
    assert_eq!(empty_all_json["pods"], serde_json::json!([]));
    assert_eq!(empty_all_json["searchTerm"], "50");
    assert_eq!(empty_all_json["searchType"], "all");
    assert_eq!(empty_all_json["totalFound"], 0);

    let malformed_all = crate::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/all?limit=0",
        None,
        "",
        &state,
    )
    .await
    .expect("reject invalid all-pods discovery limit");
    assert_eq!(
        malformed_all.status, "400 Bad Request",
        "{}",
        malformed_all.body
    );
    assert!(malformed_all
        .body
        .contains("Limit must be between 1 and 100"));

    for (path, expected) in [
        (
            "/api/v0/podcore/discovery/name/",
            "Name is required and must be within length limits",
        ),
        (
            "/api/v0/podcore/discovery/tag/",
            "Tag is required and must be within length limits",
        ),
        (
            "/api/v0/podcore/discovery/tags/",
            "Tags are required and must be within length limits",
        ),
        (
            "/api/v0/podcore/discovery/content/",
            "ContentId is required",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(
            response.status, "400 Bad Request",
            "{path}: {}",
            response.body
        );
        assert!(
            response.body.contains(expected),
            "{path}: {}",
            response.body
        );
    }

    let registered = crate::route_http_request(
        "POST",
        "/api/v0/podcore/discovery/register",
        None,
        r#"{"podId":"discovery-audit-pod","name":"Audit Pod","visibility":"Listed","tags":["music","live"],"focusContentId":"content:music:recording:discovery-audit"}"#,
        &state,
    )
    .await
    .expect("register discovery pod");
    assert_eq!(registered.status, "200 OK", "{}", registered.body);
    assert_eq!(registered.content_type, "application/json");

    let search = crate::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/name/audit-pod",
        None,
        "",
        &state,
    )
    .await
    .expect("search discovery pods");
    assert_eq!(search.status, "200 OK", "{}", search.body);
    assert_eq!(search.content_type, "application/json");
    let search_json = serde_json::from_str::<serde_json::Value>(&search.body).unwrap();
    assert_eq!(
        search_json["pods"].as_array().unwrap().len(),
        1,
        "{search_json}"
    );
    assert_eq!(search_json["searchTerm"], "audit-pod");
    assert_eq!(search_json["searchType"], "name");
    assert_eq!(search_json["totalFound"], 1);

    let tag = crate::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/tag/music",
        None,
        "",
        &state,
    )
    .await
    .expect("search discovery tag");
    let tag_json = serde_json::from_str::<serde_json::Value>(&tag.body).unwrap();
    assert_eq!(tag.status, "200 OK", "{}", tag.body);
    assert_eq!(tag.content_type, "application/json");
    assert_eq!(tag_json["pods"].as_array().unwrap().len(), 1, "{tag_json}");
    assert_eq!(tag_json["searchTerm"], "music");
    assert_eq!(tag_json["searchType"], "tag");
    assert_eq!(tag_json["totalFound"], 1);

    let tags = crate::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/tags/music,live",
        None,
        "",
        &state,
    )
    .await
    .expect("search discovery tags");
    let tags_json = serde_json::from_str::<serde_json::Value>(&tags.body).unwrap();
    assert_eq!(tags.status, "200 OK", "{}", tags.body);
    assert_eq!(tags.content_type, "application/json");
    assert_eq!(
        tags_json["pods"].as_array().unwrap().len(),
        1,
        "{tags_json}"
    );
    assert_eq!(tags_json["searchTerm"], "music,live");
    assert_eq!(tags_json["searchType"], "tags");
    assert_eq!(tags_json["totalFound"], 1);

    for (path, search_type, search_term) in [
        (
            "/api/v0/podcore/discovery/name/not-present",
            "name",
            "not-present",
        ),
        (
            "/api/v0/podcore/discovery/tag/not-present",
            "tag",
            "not-present",
        ),
        (
            "/api/v0/podcore/discovery/tags/not-present,other",
            "tags",
            "not-present,other",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let response_json =
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        assert_eq!(response.status, "200 OK", "{path}: {}", response.body);
        assert_eq!(response_json["pods"], serde_json::json!([]), "{path}");
        assert_eq!(response_json["searchType"], search_type, "{path}");
        assert_eq!(response_json["searchTerm"], search_term, "{path}");
        assert_eq!(response_json["totalFound"], 0, "{path}");
    }

    let all = crate::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/all?limit=50",
        None,
        "",
        &state,
    )
    .await
    .expect("populated all-pods discovery search");
    let all_json = serde_json::from_str::<serde_json::Value>(&all.body).unwrap();
    assert_eq!(all.status, "200 OK", "{}", all.body);
    assert_eq!(all.content_type, "application/json");
    assert_eq!(all_json["pods"].as_array().unwrap().len(), 1, "{all_json}");
    assert_eq!(all_json["searchTerm"], "50");
    assert_eq!(all_json["searchType"], "all");
    assert_eq!(all_json["totalFound"], 1);

    let missing_content = crate::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/content/content:music:recording:not-present",
        None,
        "",
        &state,
    )
    .await
    .expect("missing content discovery search");
    let missing_content_json =
        serde_json::from_str::<serde_json::Value>(&missing_content.body).unwrap();
    assert_eq!(missing_content.status, "200 OK", "{}", missing_content.body);
    assert_eq!(missing_content_json["pods"], serde_json::json!([]));
    assert_eq!(missing_content_json["searchType"], "content");
    assert_eq!(
        missing_content_json["searchTerm"],
        "content:music:recording:not-present"
    );
    assert_eq!(missing_content_json["totalFound"], 0);

    let content = crate::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/content/content:music:recording:discovery-audit",
        None,
        "",
        &state,
    )
    .await
    .expect("populated content discovery search");
    let content_json = serde_json::from_str::<serde_json::Value>(&content.body).unwrap();
    assert_eq!(content.status, "200 OK", "{}", content.body);
    assert_eq!(content.content_type, "application/json");
    assert_eq!(
        content_json["pods"].as_array().unwrap().len(),
        1,
        "{content_json}"
    );
    assert_eq!(content_json["searchType"], "content");
    assert_eq!(
        content_json["searchTerm"],
        "content:music:recording:discovery-audit"
    );
    assert_eq!(content_json["totalFound"], 1);

    let stats =
        crate::route_http_request("GET", "/api/v0/podcore/discovery/stats", None, "", &state)
            .await
            .expect("discovery stats");
    assert_eq!(stats.status, "200 OK", "{}", stats.body);
    assert_eq!(stats.content_type, "application/json");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalRegisteredPods"], 1);
    assert_eq!(stats_json["activeDiscoveryEntries"], 5);
    assert_eq!(stats_json["expiredEntries"], 0);
    assert_eq!(stats_json["registrationsByTag"]["music"], 1);
    assert_eq!(stats_json["registrationsByTag"]["live"], 1);
    assert_eq!(stats_json["searchesByType"]["name"], 3);
    assert_eq!(stats_json["searchesByType"]["all"], 2);
    assert_eq!(stats_json["searchesByType"]["content"], 2);
    assert!(stats_json["lastDiscoveryOperation"].is_string());

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_discovery_stats.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/stats",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/stats",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/name/{name}",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/name/{name}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tag/{tag}",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tag/{tag}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tags/{tags}",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tags/{tags}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/name/{name}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/name/{name}",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tag/{tag}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tag/{tag}",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tags/{tags}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tags/{tags}",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/all",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/all",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/all",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/all",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/content/{*contentId}",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/content/{*contentId}",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/content/{*contentId}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/content/{*contentId}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/stats",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

/// Bulk differential proof crediting 10 PodCore routes' `nominal-
/// status-headers-body`, `mutation-side-effects-and-readback`, and
/// `missing-empty-or-conflict-state` cases, independently re-derived
/// from `pod_management_routes_persist_crud_members_and_bindings`'s
/// full real CRUD lifecycle (create -> list -> detail -> members ->
/// join -> bind channel -> re-GET shows binding -> ban -> re-GET
/// members reflects ban -> rename -> post message -> delete -> re-GET
/// 404s). slskdN-only (confirmed against the frozen registry -- slskd
/// declares none of these routes).
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_pod_management_routes_persist_crud_members_and_bindings(
) {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();

    let created = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod:api","name":"API Pod","isPublic":true,"maxMembers":4,"tags":["music"],"channels":[{"channelId":"general","kind":0,"name":"General"}]},"requestingPeerId":"ignored-by-auth"}"#,
        &state,
    )
    .await
    .expect("create pod");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
    let create_pass = created.status == "201 Created"
        && created_json["podId"] == "pod:api"
        && created_json["name"] == "API Pod";
    record!(
        "POST",
        "/api/v0/pods",
        "mutation-side-effects-and-readback",
        create_pass
    );

    let listed = crate::route_http_request("GET", "/api/pods", None, "", &state)
        .await
        .expect("list pods");
    let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap_or_default();
    let list_pass = listed.status == "200 OK"
        && listed_json.as_array().is_some_and(|array| array.len() == 1)
        && listed_json[0]["podId"] == "pod:api";
    record!(
        "GET",
        "/api/v0/pods",
        "nominal-status-headers-body",
        list_pass
    );

    let detail = crate::route_http_request("GET", "/api/pods/pod%3Aapi", None, "", &state)
        .await
        .expect("pod detail");
    record!(
        "GET",
        "/api/v0/pods/{podId}",
        "nominal-status-headers-body",
        detail.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/pods/{podId}",
        "populated-dynamic-state",
        detail.status == "200 OK"
            && detail.body.contains("API Pod")
            && detail.body.contains("general")
    );

    let members = crate::route_http_request("GET", "/api/pods/pod%3Aapi/members", None, "", &state)
        .await
        .expect("pod members");
    let members_json = serde_json::from_str::<serde_json::Value>(&members.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/pods/{podId}/members",
        "nominal-status-headers-body",
        members.status == "200 OK"
            && members_json
                .as_array()
                .is_some_and(|array| array.len() == 1)
            && members_json[0]["role"] == "owner"
    );

    *state.runtime_credentials.write().await =
        Some(crate::LoginCredentials::default_client("member", "secret"));
    let joined = crate::route_http_request(
        "POST",
        "/api/pods/pod%3Aapi/join",
        None,
        r#"{"peerId":"member"}"#,
        &state,
    )
    .await
    .expect("join pod");
    let join_pass = joined.status == "200 OK"
        && serde_json::from_str::<serde_json::Value>(&joined.body).unwrap_or_default()["joined"]
            == true;
    record!(
        "POST",
        "/api/v0/pods/{podId}/join",
        "mutation-side-effects-and-readback",
        join_pass
    );
    *state.runtime_credentials.write().await = None;

    let bound = crate::route_http_request(
        "POST",
        "/api/pods/pod%3Aapi/channels/general/bind",
        None,
        r#"{"roomName":"ambient","mode":"mirror"}"#,
        &state,
    )
    .await
    .expect("bind pod channel");
    let bound_detail = crate::route_http_request("GET", "/api/pods/pod%3Aapi", None, "", &state)
        .await
        .expect("bound pod detail");
    let bound_detail_json =
        serde_json::from_str::<serde_json::Value>(&bound_detail.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/pods/{podId}/channels/{channelId}/bind",
        "mutation-side-effects-and-readback",
        bound.status == "200 OK"
            && bound_detail_json["channels"][0]["bindingInfo"] == "soulseek-room:ambient"
    );

    let banned = crate::route_http_request(
        "POST",
        "/api/pods/pod%3Aapi/ban",
        None,
        r#"{"peerId":"member"}"#,
        &state,
    )
    .await
    .expect("ban pod member");
    let members_after_ban =
        crate::route_http_request("GET", "/api/pods/pod%3Aapi/members", None, "", &state)
            .await
            .expect("members after ban");
    let members_after_ban_json =
        serde_json::from_str::<serde_json::Value>(&members_after_ban.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/pods/{podId}/ban",
        "mutation-side-effects-and-readback",
        banned.status == "200 OK"
            && members_after_ban_json
                .as_array()
                .is_some_and(|array| array.len() == 1)
    );

    let updated = crate::route_http_request(
        "PUT",
        "/api/pods/pod%3Aapi",
        None,
        r#"{"pod":{"podId":"pod:api","name":"Renamed Pod","isPublic":true,"maxMembers":4,"channels":[{"channelId":"general","kind":0,"name":"General"}]}}"#,
        &state,
    )
    .await
    .expect("update pod");
    let updated_json = serde_json::from_str::<serde_json::Value>(&updated.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/pods/{podId}",
        "nominal-status-headers-body",
        updated.status == "200 OK"
    );
    record!(
        "PUT",
        "/api/v0/pods/{podId}",
        "mutation-side-effects-and-readback",
        updated.status == "200 OK" && updated_json["name"] == "Renamed Pod"
    );

    let message = crate::route_http_request(
        "POST",
        "/api/v0/pods/pod%3Aapi/channels/general/messages",
        None,
        r#"{"body":"delete me","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("pod message before delete");
    record!(
        "POST",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "nominal-status-headers-body",
        message.status == "200 OK"
    );

    let deleted = crate::route_http_request("DELETE", "/api/pods/pod%3Aapi", None, "", &state)
        .await
        .expect("delete pod");
    let missing = crate::route_http_request("GET", "/api/pods/pod%3Aapi", None, "", &state)
        .await
        .expect("deleted pod");
    let delete_pass = deleted.status == "204 No Content" && missing.status == "404 Not Found";
    record!(
        "DELETE",
        "/api/v0/pods/{podId}",
        "missing-empty-or-conflict-state",
        delete_pass
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("pod_management_routes_persist_crud_members_and_bindings.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api pod-management mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting PodCore stats routes' nominal,
/// empty-state, and populated-state cases, independently re-derived
/// from real seeded pod/member/channel-message aggregation and runtime
/// counter checks. slskdN-only (confirmed against the frozen registry).
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_podcore_stats_gets() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();

    let empty_membership =
        crate::route_http_request("GET", "/api/v0/podcore/membership/stats", None, "", &state)
            .await
            .expect("empty membership stats");
    let empty_membership_json =
        serde_json::from_str::<serde_json::Value>(&empty_membership.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/membership/stats",
        "missing-empty-or-conflict-state",
        empty_membership.status == "200 OK"
            && empty_membership_json["totalMemberships"] == 0
            && empty_membership_json["activeMemberships"] == 0
            && empty_membership_json["membershipsByRole"] == serde_json::json!({})
            && empty_membership_json["membershipsByPod"] == serde_json::json!({})
    );
    let malformed_membership = crate::route_http_request(
        "GET",
        "/api/v0/podcore/membership/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed membership stats query");
    let malformed_membership_json =
        serde_json::from_str::<serde_json::Value>(&malformed_membership.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/membership/stats",
        "malformed-path-query-or-body",
        malformed_membership.status == "200 OK"
            && malformed_membership_json["totalMemberships"] == 0
            && malformed_membership_json["activeMemberships"] == 0
    );

    let empty_messages =
        crate::route_http_request("GET", "/api/v0/podcore/messages/stats", None, "", &state)
            .await
            .expect("empty message stats");
    let empty_messages_json =
        serde_json::from_str::<serde_json::Value>(&empty_messages.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/messages/stats",
        "missing-empty-or-conflict-state",
        empty_messages.status == "200 OK"
            && empty_messages_json["totalMessages"] == 0
            && empty_messages_json["totalSizeBytes"] == 0
            && empty_messages_json["messagesPerPod"] == serde_json::json!({})
            && empty_messages_json["messagesPerChannel"] == serde_json::json!({})
            && empty_messages_json.get("oldestMessage").is_none()
            && empty_messages_json.get("newestMessage").is_none()
    );
    let malformed_messages = crate::route_http_request(
        "GET",
        "/api/v0/podcore/messages/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed message stats query");
    let malformed_messages_json =
        serde_json::from_str::<serde_json::Value>(&malformed_messages.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/messages/stats",
        "malformed-path-query-or-body",
        malformed_messages.status == "200 OK"
            && malformed_messages_json["totalMessages"] == 0
            && malformed_messages_json["totalSizeBytes"] == 0
    );

    let empty_routing =
        crate::route_http_request("GET", "/api/v0/podcore/routing/stats", None, "", &state)
            .await
            .expect("empty routing stats");
    let empty_routing_json =
        serde_json::from_str::<serde_json::Value>(&empty_routing.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/routing/stats",
        "missing-empty-or-conflict-state",
        empty_routing.status == "200 OK"
            && empty_routing_json["totalMessagesRouted"] == 0
            && empty_routing_json["totalRoutingAttempts"] == 0
            && empty_routing_json["successfulRoutingCount"] == 0
            && empty_routing_json["failedRoutingCount"] == 0
            && empty_routing_json["activeDeduplicationItems"] == 0
            && empty_routing_json["routingStatsByPod"] == serde_json::json!({})
    );
    let malformed_routing = crate::route_http_request(
        "GET",
        "/api/v0/podcore/routing/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed routing stats query");
    let malformed_routing_json =
        serde_json::from_str::<serde_json::Value>(&malformed_routing.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/routing/stats",
        "malformed-path-query-or-body",
        malformed_routing.status == "200 OK"
            && malformed_routing_json["totalMessagesRouted"] == 0
            && malformed_routing_json["totalRoutingAttempts"] == 0
    );

    let empty_verification = crate::route_http_request(
        "GET",
        "/api/v0/podcore/verification/stats",
        None,
        "",
        &state,
    )
    .await
    .expect("empty verification stats");
    let empty_verification_json =
        serde_json::from_str::<serde_json::Value>(&empty_verification.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/verification/stats",
        "missing-empty-or-conflict-state",
        empty_verification.status == "200 OK"
            && empty_verification_json["totalVerifications"] == 0
            && empty_verification_json["successfulVerifications"] == 0
            && empty_verification_json["failedMembershipChecks"] == 0
            && empty_verification_json["failedSignatureChecks"] == 0
            && empty_verification_json["bannedMemberRejections"] == 0
    );
    let pod_id = "pod:stats-differential";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Stats differential",
                "isPublic": true,
                "channels": [{"channelId": "general", "name": "General"}]
            }))
            .expect("deserialize stats pod"),
            "owner-peer".to_owned(),
        )
        .expect("create stats pod");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            crate::pods::PodMember {
                peer_id: "moderator-peer".to_owned(),
                role: "mod".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: Some("2026-01-01T00:00:00+00:00".to_owned()),
                last_seen: Some("2026-01-02T00:00:00+00:00".to_owned()),
            },
        )
        .expect("add stats member");
    {
        let mut channels = state.pod_channels.write().await;
        channels
            .append(
                pod_id.to_owned(),
                "general".to_owned(),
                "owner-peer".to_owned(),
                "one".to_owned(),
                String::new(),
                1_000,
            )
            .expect("append first stats message");
        channels
            .append(
                pod_id.to_owned(),
                "general".to_owned(),
                "moderator-peer".to_owned(),
                "two".to_owned(),
                String::new(),
                2_000,
            )
            .expect("append second stats message");
    }

    let membership =
        crate::route_http_request("GET", "/api/v0/podcore/membership/stats", None, "", &state)
            .await
            .expect("membership stats");
    let membership_json =
        serde_json::from_str::<serde_json::Value>(&membership.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/membership/stats",
        "nominal-status-headers-body",
        membership.status == "200 OK"
    );
    record!(
        "/api/v0/podcore/membership/stats",
        "populated-dynamic-state",
        membership_json["totalMemberships"] == 2
            && membership_json["activeMemberships"] == 2
            && membership_json["membershipsByRole"]["owner"] == 1
            && membership_json["membershipsByRole"]["mod"] == 1
            && membership_json["membershipsByPod"][pod_id] == 2
    );

    let messages =
        crate::route_http_request("GET", "/api/v0/podcore/messages/stats", None, "", &state)
            .await
            .expect("message stats");
    let messages_json =
        serde_json::from_str::<serde_json::Value>(&messages.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/messages/stats",
        "nominal-status-headers-body",
        messages.status == "200 OK"
    );
    record!(
        "/api/v0/podcore/messages/stats",
        "populated-dynamic-state",
        messages_json["totalMessages"] == 2
            && messages_json["totalSizeBytes"] == 400
            && messages_json["messagesPerPod"][pod_id] == 2
            && messages_json["messagesPerChannel"]["general"] == 2
            && messages_json["oldestMessage"] == "1970-01-01T00:00:01+00:00"
            && messages_json["newestMessage"] == "1970-01-01T00:00:02+00:00"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_stats_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-stats mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
