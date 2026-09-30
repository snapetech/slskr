use super::*;

pub(super) fn rounded_vec(values: &[f64]) -> Vec<f64> {
    values
        .iter()
        .map(|value| (value * 1000.0).round() / 1000.0)
        .collect()
}

pub(super) fn populated_route_html(path: &str) -> String {
    let (endpoint_path, body) = match route_kind(path) {
        RouteKind::Search | RouteKind::DiscoveryGraph => (
            "/searches/:id/responses",
            r#"[{"id":"search-1","username":"peer1","files":[{"filename":"Archive/Track.flac"}],"queueLength":1,"hasFreeUploadSlot":true}]"#,
        ),
        RouteKind::PlaylistIntake => (
            "/source-feed-imports/preview",
            r#"[{"artist":"Archive Artist","title":"Public Domain Theme","status":"Matched"}]"#,
        ),
        RouteKind::Wishlist => (
            "/wishlist",
            r#"[{"id":"wish-1","searchText":"rare live set","filter":"flac","enabled":true,"autoDownload":false}]"#,
        ),
        RouteKind::Downloads => (
            "/transfers/downloads",
            r#"[{"id":77,"username":"peer1","filename":"Remote/Song.mp3","state":"Queued","progress":0.5}]"#,
        ),
        RouteKind::Uploads => (
            "/transfers/uploads",
            r#"[{"id":78,"username":"peer1","filename":"Remote/Upload.mp3","state":"Queued","progress":0.5}]"#,
        ),
        RouteKind::Messages | RouteKind::Rooms => (
            "/conversations",
            r#"[{"username":"peer1","lastMessage":"hello","unreadCount":1}]"#,
        ),
        RouteKind::Users => (
            "/users",
            r#"[{"username":"peer1","status":"Online","files":2}]"#,
        ),
        RouteKind::Contacts => (
            "/contacts",
            r#"[{"nickname":"Peer One","peerId":"peer1","group":"trusted","verified":true}]"#,
        ),
        RouteKind::Solid => (
            "/solid/status",
            r#"{"webId":"https://example.test/profile#me","storage":"ready","status":"connected"}"#,
        ),
        RouteKind::Collections => (
            "/collections",
            r#"[{"id":"collection-1","title":"Fixture Collection","type":"ShareList","itemCount":1}]"#,
        ),
        RouteKind::ShareGroups => (
            "/sharegroups",
            r#"[{"id":"group-1","name":"Fixture Group","memberCount":1,"createdAt":"today"}]"#,
        ),
        RouteKind::SharedWithMe => (
            "/share-grants",
            r#"[{"id":"grant-1","title":"Fixture Share","owner":"peer1","permissions":"read"}]"#,
        ),
        RouteKind::Browse => (
            "/users/:username/browse",
            r#"[{"name":"/Music/Open Sessions","isDirectory":true,"size":0},{"name":"/Music/Open Sessions/Track.flac","isDirectory":false,"size":1234}]"#,
        ),
        RouteKind::System => (
            "/server",
            r#"{"state":"connected","username":"audit-user"}"#,
        ),
    };
    route_workspace_result_html(
        path,
        &[EndpointBody {
            endpoint: ApiEndpoint {
                method: "GET",
                path: endpoint_path,
                surface: "test",
            },
            body: body.to_string(),
        }],
    )
}
