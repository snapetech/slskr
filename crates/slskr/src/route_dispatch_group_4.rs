async fn route_dispatch_group_4(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
    let mut response = route_dispatch_group_4_collections(context).await;
    if route_is_unhandled(&response) {
        response = route_dispatch_group_4_wishlist(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_4_contacts_sharegroups(context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_4_notes_interests_grants(context).await;
    }
    response
}
