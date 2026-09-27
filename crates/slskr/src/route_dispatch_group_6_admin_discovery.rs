async fn route_dispatch_group_6_admin_discovery(
    context: &RouteDispatchContext<'_, '_>,
) -> RouteDispatchResult {
    let mut response = route_dispatch_group_6_admin_routes(context).await;
    if route_is_unhandled(&response) {
        response = route_dispatch_group_6_relay_controller(context).await;
    }
    response
}
