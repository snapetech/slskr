async fn route_dispatch_group_1(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
    let mut response = route_dispatch_group_1_discovery(context).await;
    if route_is_unhandled(&response) {
        response = route_dispatch_group_1_telemetry(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_1_events(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_1_shares(context).await;
    }
    response
}
