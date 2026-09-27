async fn route_dispatch_group_6_telemetry(
    context: &RouteDispatchContext<'_, '_>,
) -> RouteDispatchResult {
    let mut response = route_dispatch_group_6_metrics_jobs(context).await;
    if route_is_unhandled(&response) {
        response = route_dispatch_group_6_pods(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_6_federation_security(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_6_multisource_graph(context).await;
    }
    response
}
