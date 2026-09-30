async fn route_dispatch_group_0(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
    let mut response = route_dispatch_group_0_control_plane(context).await;
    if route_is_unhandled(&response) {
        response = route_dispatch_group_0_mesh_data(context).await;
    }
    response
}
