async fn route_dispatch_group_6(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
    let mut response = route_dispatch_group_6_admin_discovery(context).await;
    if route_is_unhandled(&response) {
        response = route_dispatch_group_6_telemetry(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_6_media_jobs(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_6_security_shares(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_6_integrations(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_6_musicbrainz(context).await;
    }
    response
}
