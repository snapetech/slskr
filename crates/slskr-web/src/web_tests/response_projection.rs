use super::*;

#[test]
fn runtime_probe_html_escapes_api_values() {
    let html =
        runtime_probe_result_html(&[("Probe", "/api/v0/probe", Ok(r#"<script>"bad"</script>"#))]);
    assert!(html.contains("&lt;script&gt;&quot;bad&quot;&lt;/script&gt;"));
    assert!(!html.contains("<script>"));
}

#[test]
fn wishlist_history_renderer_escapes_remote_fields_and_rejects_malformed_json() {
    let html = wishlist_history_response_html(
        r#"[{"searchText":"<script>alert(1)</script>","status":"completed<img>","startedAt":"now&later","result_count":2}]"#,
    );
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(html.contains("completed&lt;img&gt;"));
    assert!(html.contains("now&amp;later"));
    assert!(!html.contains("<script>"));
    assert_eq!(
        wishlist_history_response_html("not json"),
        "<p>Run history could not be read.</p>"
    );
}

#[test]
fn transfer_attempt_renderer_escapes_attempt_history() {
    let html = transfer_attempts_response_html(
        r#"{"attempts":[{"peer_username":"peer<script>","status":"failed<img>","filename":"Remote/A&B.flac"}]}"#,
    );
    assert!(html.contains("peer&lt;script&gt;"));
    assert!(html.contains("failed&lt;img&gt;"));
    assert!(html.contains("Remote/A&amp;B.flac"));
    assert!(!html.contains("<script>"));
    assert_eq!(
        transfer_attempts_response_html("bad"),
        "<p>Attempt history could not be read.</p>"
    );
}

#[test]
fn rust_route_summaries_parse_live_response_shapes() {
    let search = route_summary_result_html(
        "/searches/42",
        &[
            EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/searches/records",
                    surface: "search",
                },
                body: r#"{"entries":[{"id":"1"},{"id":"2"}]}"#.to_string(),
            },
            EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/searches/:id/responses",
                    surface: "search",
                },
                body: r#"[{"username":"peer1"}]"#.to_string(),
            },
        ],
    );
    assert!(search.contains(">2<"));
    assert!(search.contains(">1<"));
    assert!(search.contains("active records"));

    let transfers = route_summary_result_html(
        "/downloads",
        &[
            EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/transfers/downloads",
                    surface: "transfers",
                },
                body: r#"[{"username":"peer1"}]"#.to_string(),
            },
            EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/transfers/uploads",
                    surface: "transfers",
                },
                body: "[]".to_string(),
            },
        ],
    );
    assert!(transfers.contains("Downloads"));
    assert!(transfers.contains(">1<"));
    assert!(transfers.contains("Uploads"));

    let rooms = route_summary_result_html(
        "/rooms",
        &[EndpointBody {
            endpoint: ApiEndpoint {
                method: "GET",
                path: "/rooms/joined",
                surface: "rooms",
            },
            body: r#"["contract-room"]"#.to_string(),
        }],
    );
    assert!(rooms.contains("Joined"));
    assert!(rooms.contains(">1<"));
}

#[test]
fn native_workspaces_parse_nested_live_domain_payloads() {
    let downloads = route_workspace_result_html(
            "/downloads",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/transfers/downloads",
                    surface: "transfers",
                },
                body: r#"[{"username":"parent-peer","state":"InProgress","bytesPerSecond":2048,"eta":"00:42","files":[{"filename":"Album/Track.flac","progress":0.625}]}]"#
                    .to_string(),
            }],
        );
    assert!(downloads.contains("Album/Track.flac"));
    assert!(downloads.contains("parent-peer"));
    assert!(downloads.contains("InProgress / 62% / 2048 B/s / ETA 00:42"));

    let browse = route_workspace_result_html(
            "/browse",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/users/:username/browse",
                    surface: "browse",
                },
                body: r#"{"directories":[{"name":"Music","isDirectory":true,"files":[{"filename":"Music/live.mp3","size":321}]}],"root":{"files":[{"name":"root.flac","fileSize":654}]}}"#
                    .to_string(),
            }],
        );
    assert!(browse.contains("Music"));
    assert!(browse.contains("folder"));
    assert!(browse.contains("Music/live.mp3"));
    assert!(browse.contains("root.flac"));

    let collections = route_workspace_result_html(
        "/collections",
        &[EndpointBody {
            endpoint: ApiEndpoint {
                method: "GET",
                path: "/collections",
                surface: "collections",
            },
            body: r#"[{"title":"Road Trips","kind":"playlist","items":[{"id":"1"},{"id":"2"}]}]"#
                .to_string(),
        }],
    );
    assert!(collections.contains("Road Trips"));
    assert!(collections.contains("2 items"));

    let sharegroups = route_workspace_result_html(
            "/sharegroups",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/sharegroups",
                    surface: "collections",
                },
                body: r#"[{"name":"Trusted","members":[{"username":"peer1"},{"username":"peer2"}],"createdAt":"today"}]"#
                    .to_string(),
            }],
        );
    assert!(sharegroups.contains("Trusted"));
    assert!(sharegroups.contains("2 members"));

    let shared = route_workspace_result_html(
                "/shared",
                &[EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/share-grants",
                        surface: "collections",
                    },
                    body: r#"[{"id":"grant-nested","collection":{"title":"Inbox"},"owner":{"username":"sender"},"grant":{"permissions":"read/write"}}]"#
                        .to_string(),
                }],
        );
    assert!(shared.contains("Inbox"));
    assert!(shared.contains("sender"));
    assert!(shared.contains("read/write"));
}

#[test]
fn rust_route_summaries_escape_live_response_values() {
    let html = route_summary_result_html(
        "/messages",
        &[EndpointBody {
            endpoint: ApiEndpoint {
                method: "GET",
                path: "/conversations/:username",
                surface: "messages",
            },
            body: r#"{"username":"<script>"}"#.to_string(),
        }],
    );
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>"));
}
