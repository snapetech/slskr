async fn route_dispatch_group_2(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
    let mut response = route_dispatch_group_2_session_search(context).await;
    if route_is_unhandled(&response) {
        response = route_dispatch_group_2_downloads(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_2_transfer_status(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_2_transfer_files(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_2_search_rooms(context).await;
    }
    response
}
