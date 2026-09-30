async fn route_dispatch_group_3(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
    let mut response = route_dispatch_group_3_people_browse(context).await;
    if route_is_unhandled(&response) {
        response = route_dispatch_group_3_admin_controls(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_3_room_membership(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_3_options_diagnostics(context).await;
    }
    response
}
